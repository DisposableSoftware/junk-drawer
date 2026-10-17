use crate::ops::{self, Transfer};
use gtk::{gdk, gio, glib, prelude::*};
use std::cell::RefCell;
use std::rc::Rc;

pub fn file_of(info: &gio::FileInfo) -> Option<gio::File> {
    info.attribute_object("standard::file").and_downcast()
}

/// The file an item stands for: its `standard::target-uri` in `recent:///` and other
/// virtual folders (`INT-12`), otherwise the item itself.
pub fn target_of(info: &gio::FileInfo) -> Option<gio::File> {
    match info.attribute_string("standard::target-uri") {
        Some(uri) => Some(gio::File::for_uri(&uri)),
        None => file_of(info),
    }
}

/// Whether an item is a folder. Recent entries for folders aren't typed as directories,
/// only marked `inode/directory`.
pub fn is_dir(info: &gio::FileInfo) -> bool {
    info.file_type() == gio::FileType::Directory
        || info.content_type().is_some_and(|t| t == "inode/directory")
}

/// Whether `dir` is the Recent list, where items are entries rather than files.
pub fn is_recent(dir: &gio::File) -> bool {
    dir.has_uri_scheme("recent")
}

pub fn selected_infos(selection: &gtk::MultiSelection) -> Vec<gio::FileInfo> {
    let set = selection.selection();
    (0..set.size() as u32)
        .filter_map(|i| selection.item(set.nth(i)).and_downcast::<gio::FileInfo>())
        .collect()
}

pub fn selected_files(selection: &gtk::MultiSelection) -> Vec<gio::File> {
    selected_infos(selection)
        .iter()
        .filter_map(file_of)
        .collect()
}

/// `selected_files`, with Recent items replaced by their targets.
pub fn selected_targets(selection: &gtk::MultiSelection) -> Vec<gio::File> {
    selected_infos(selection)
        .iter()
        .filter_map(target_of)
        .collect()
}

/// Drags the whole selection, or only this item if it isn't part of the selection.
pub fn add_drag_source(
    widget: &impl IsA<gtk::Widget>,
    item: &gtk::ListItem,
    selection: &gtk::MultiSelection,
) {
    let source = gtk::DragSource::builder()
        .actions(gdk::DragAction::COPY | gdk::DragAction::MOVE)
        .build();
    source.connect_prepare(glib::clone!(
        #[weak]
        item,
        #[weak]
        selection,
        #[upgrade_or]
        None,
        move |source, x, y| {
            let pos = item.position();
            if !selection.is_selected(pos) {
                selection.select_item(pos, true);
            }
            let files = selected_targets(&selection);
            if files.is_empty() {
                return None;
            }
            if let Some(widget) = source.widget() {
                let image = gtk::WidgetPaintable::new(Some(&widget));
                source.set_icon(Some(&image), x as i32, y as i32);
            }
            Some(gdk::ContentProvider::for_value(
                &gdk::FileList::from_array(&files).to_value(),
            ))
        }
    ));
    widget.add_controller(source);
}

/// Accepts files dropped onto `widget` into the directory `dest` returns at drop time.
/// Same filesystem moves and other filesystems copy. Ctrl forces copy, Shift forces move.
pub fn add_drop_target(
    widget: &impl IsA<gtk::Widget>,
    dest: impl Fn() -> Option<gio::File> + 'static,
) {
    let target = gtk::DropTarget::new(
        gdk::FileList::static_type(),
        gdk::DragAction::COPY | gdk::DragAction::MOVE,
    );
    target.set_preload(true);
    let dest = Rc::new(dest);
    let same_fs_cache = Rc::new(RefCell::new(None));

    target.connect_accept(glib::clone!(
        #[strong]
        dest,
        move |_, drop| dest().is_some()
            && drop.formats().contains_type(gdk::FileList::static_type())
    ));
    let pick = glib::clone!(
        #[strong]
        dest,
        #[strong]
        same_fs_cache,
        move |t: &gtk::DropTarget, _: f64, _: f64| choose_action(
            t,
            dest().as_ref(),
            &same_fs_cache
        )
    );
    target.connect_enter(pick.clone());
    target.connect_motion(pick);
    target.connect_leave(glib::clone!(
        #[strong]
        same_fs_cache,
        move |_| {
            same_fs_cache.take();
        }
    ));
    target.connect_drop(move |t, value, _, _| {
        let (Some(dir), Ok(list)) = (dest(), value.get::<gdk::FileList>()) else {
            return false;
        };
        let mode = match choose_action(t, Some(&dir), &same_fs_cache) {
            a if a == gdk::DragAction::MOVE => Transfer::Move,
            a if a == gdk::DragAction::COPY => Transfer::Copy,
            _ => return false,
        };
        let parent = t
            .widget()
            .and_then(|w| w.root())
            .and_downcast::<gtk::Window>();
        ops::transfer(parent, list.files(), dir, mode);
        true
    });
    widget.add_controller(target);
}

type SameFsCache = RefCell<Option<(String, String, bool)>>;

fn choose_action(
    t: &gtk::DropTarget,
    dest: Option<&gio::File>,
    cache: &SameFsCache,
) -> gdk::DragAction {
    let none = gdk::DragAction::empty();
    let Some(dest) = dest else { return none };
    let allowed = t.current_drop().map(|d| d.actions()).unwrap_or(none);
    let files = t
        .value()
        .and_then(|v| v.get::<gdk::FileList>().ok())
        .map(|l| l.files())
        .unwrap_or_default();
    if files.iter().any(|f| f.equal(dest) || dest.has_prefix(f)) {
        return none;
    }

    let state = t.current_event_state();
    let wanted = if state.contains(gdk::ModifierType::CONTROL_MASK) {
        gdk::DragAction::COPY
    } else if state.contains(gdk::ModifierType::SHIFT_MASK)
        || files.first().is_some_and(|f| same_fs(f, dest, cache))
    {
        gdk::DragAction::MOVE
    } else {
        gdk::DragAction::COPY
    };

    let already_here = !files.is_empty()
        && files
            .iter()
            .all(|f| f.parent().is_some_and(|p| p.equal(dest)));
    if wanted == gdk::DragAction::MOVE && already_here {
        return none;
    }
    if allowed.contains(wanted) {
        wanted
    } else if allowed.contains(gdk::DragAction::COPY) && !already_here {
        gdk::DragAction::COPY
    } else {
        none
    }
}

/// Motion events fire constantly, so the result is cached per (source, dest) pair.
fn same_fs(src: &gio::File, dest: &gio::File, cache: &SameFsCache) -> bool {
    let key = (src.uri().to_string(), dest.uri().to_string());
    if let Some((s, d, same)) = cache.borrow().as_ref()
        && (s, d) == (&key.0, &key.1)
    {
        return *same;
    }
    let id = |f: &gio::File| {
        f.query_info(
            "id::filesystem",
            gio::FileQueryInfoFlags::NONE,
            gio::Cancellable::NONE,
        )
        .ok()
        .and_then(|i| i.attribute_string("id::filesystem"))
    };
    let a = id(src);
    let same = a.is_some() && a == id(dest);
    cache.replace(Some((key.0, key.1, same)));
    same
}
