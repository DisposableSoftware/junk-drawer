use crate::{
    caption, clipboard, config, dialog, dnd, help, keys, listing, location, menu, openwith, ops,
    properties, rename, sidebar, sort, state, thumbnail, zoom,
};
use gtk::{gdk, gio, glib, pango, prelude::*};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const THROBBER_DELAY: std::time::Duration = std::time::Duration::from_millis(200);

const ATTRS: &str = "standard::name,standard::display-name,standard::icon,standard::type,\
standard::size,standard::content-type,standard::is-hidden,standard::is-backup,standard::target-uri,\
time::modified,unix::mode";

/// Per-window model and navigation state. Holds no widgets, so closures can keep it alive
/// without creating reference cycles through the window.
struct State {
    list: Rc<listing::Listing>,
    filter: gtk::CustomFilter,
    selection: gtk::MultiSelection,
    show_hidden: Rc<Cell<bool>>,
    captions: Option<Rc<caption::Captions>>,
    back: RefCell<Vec<gio::File>>,
    forward: RefCell<Vec<gio::File>>,
    /// An item to select and focus once it shows up in the listing.
    /// Items to select once listed (`NAV-15`, `NAV-5`).
    reveal: RefCell<Vec<gio::File>>,
}

impl State {
    fn dir(&self) -> Option<gio::File> {
        self.list.file()
    }

    fn navigate(&self, window: &gtk::ApplicationWindow, dest: gio::File, record: bool) {
        let Some(current) = self.dir() else { return };
        if current.equal(&dest) {
            return;
        }
        if record {
            self.back.borrow_mut().push(current);
            self.forward.borrow_mut().clear();
        }
        self.show(window, &dest);
    }

    fn show(&self, window: &gtk::ApplicationWindow, dir: &gio::File) {
        window.set_title(Some(&dir.parse_name()));
        self.selection.unselect_all();
        if let Some(captions) = &self.captions {
            captions.clear();
        }
        self.list.set_file(Some(dir));
        // Replacing the items destroys the focused one; keep arrow keys working.
        focus_view(window);
    }

    fn step(&self, window: &gtk::ApplicationWindow, forward: bool) {
        let (from, to) = if forward {
            (&self.forward, &self.back)
        } else {
            (&self.back, &self.forward)
        };
        let Some(dest) = from.borrow_mut().pop() else {
            return;
        };
        if let Some(current) = self.dir() {
            to.borrow_mut().push(current);
        }
        self.show(window, &dest);
    }
}

pub fn open(app: &gtk::Application, dir: &gio::File) {
    open_selecting(app, dir, &[], None);
}

/// Opens a window on `dir` with `select` selected and the first of them focused once
/// listed. `startup_id` is the caller's activation token, if any.
pub fn open_selecting(
    app: &gtk::Application,
    dir: &gio::File,
    select: &[gio::File],
    startup_id: Option<&str>,
) -> gtk::ApplicationWindow {
    let list = listing::Listing::new(ATTRS, Some(dir));
    let show_hidden = Rc::new(Cell::new(state::get().show_hidden()));
    let filter = gtk::CustomFilter::new(glib::clone!(
        #[strong]
        show_hidden,
        move |obj| {
            let info = obj.downcast_ref::<gio::FileInfo>().unwrap();
            show_hidden.get() || !(info.is_hidden() || info.is_backup())
        }
    ));
    let filtered = gtk::FilterListModel::new(Some(list.model()), Some(filter.clone()));
    let sorted = gtk::SortListModel::new(Some(filtered), None::<gtk::Sorter>);
    let selection = gtk::MultiSelection::new(Some(sorted.clone()));

    let captions = caption::kind().map(|k| caption::Captions::new(k, show_hidden.clone()));
    let sizes = zoom::Sizes::new();
    let icons = icon_view(&selection, captions.clone(), sizes.clone());
    let (details, name_column) = details_view(&selection, sizes.clone());

    // The details view's column sorter also drives the icon view.
    let sorter = gtk::MultiSorter::new();
    sorter.append(sort::dirs_first());
    if let Some(columns) = details.sorter() {
        sorter.append(columns);
    }
    sorted.set_sorter(Some(&sorter));

    for view in [icons.upcast_ref::<gtk::Widget>(), details.upcast_ref()] {
        menu::add_to_view(view, &selection);
        add_background_click(view, &selection);
        dnd::add_drop_target(
            view,
            glib::clone!(
                #[weak]
                list,
                #[upgrade_or]
                None,
                move || list.file()
            ),
        );
    }

    let stack = gtk::Stack::new();
    stack.add_named(&scrolled(&icons), Some("icons"));
    stack.add_named(&scrolled(&details), Some("details"));
    stack.set_visible_child_name(&state::get().view());

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .default_width(900)
        .default_height(600)
        .css_classes(["junk"])
        .build();
    window.set_title(Some(&dir.parse_name()));

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let bar = gtk::Box::builder()
        .css_classes(["path-bar"])
        .visible(config::get().show_path)
        .build();
    let path = gtk::Label::builder()
        .css_classes(["path"])
        .xalign(0.0)
        .ellipsize(pango::EllipsizeMode::Start)
        .focusable(false)
        .can_target(false)
        .build();
    let spinner = gtk::Spinner::builder()
        .css_classes(["loading"])
        .visible(false)
        .focusable(false)
        .can_target(false)
        .valign(gtk::Align::Center)
        .build();
    let up = gtk::Button::builder()
        .icon_name("go-up-symbolic")
        .has_frame(false)
        .focusable(false)
        .focus_on_click(false)
        .valign(gtk::Align::Center)
        .css_classes(["up"])
        .build();
    window
        .bind_property("title", &path, "label")
        .sync_create()
        .build();
    // Disabled where there is no parent folder, such as `/`.
    window
        .bind_property("title", &up, "sensitive")
        .transform_to(|_, title: Option<String>| {
            Some(title.is_some_and(|t| gio::File::for_parse_name(&t).parent().is_some()))
        })
        .sync_create()
        .build();
    // Not `action-name`: the action is always enabled, and would keep the button
    // sensitive at `/`.
    up.connect_clicked(|up| {
        let _ = up.activate_action("win.go-up", None);
    });
    bar.append(&up);
    bar.append(&path);
    bar.append(&spinner);
    content.append(&bar);
    let views = gtk::Overlay::builder()
        .child(&stack)
        .vexpand(true)
        .hexpand(true)
        .build();
    let body = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    body.append(&views);
    content.append(&body);
    window.set_child(Some(&content));
    // The app may quit before a background save runs.
    window.connect_close_request(|_| {
        state::save_now();
        glib::Propagation::Proceed
    });

    let state = Rc::new(State {
        list,
        filter,
        selection,
        show_hidden,
        captions,
        back: RefCell::default(),
        forward: RefCell::default(),
        reveal: RefCell::new(select.to_vec()),
    });
    add_actions(&window, &state, &stack);
    zoom::attach(&window, &stack, &icons, &name_column, sizes);
    sort::attach(&window, &details);
    keys::attach_local(&window);
    add_mouse_nav(&window);
    sidebar::attach(
        &window,
        &body,
        glib::clone!(
            #[weak]
            state,
            #[weak]
            window,
            move |dir| state.navigate(&window, dir, true)
        ),
    );
    location::attach(
        &window,
        &bar,
        &path,
        glib::clone!(
            #[weak]
            state,
            #[weak]
            window,
            move |dir, select| {
                *state.reveal.borrow_mut() = select.into_iter().collect();
                if state.dir().is_some_and(|d| d.equal(&dir)) {
                    glib::idle_add_local_once(move || reveal_pending(&state, &window));
                } else {
                    state.navigate(&window, dir, true);
                }
            }
        ),
        glib::clone!(
            #[weak]
            window,
            move || focus_view(&window)
        ),
    );
    help::attach(
        &window,
        &views,
        glib::clone!(
            #[weak]
            window,
            move || focus_view(&window)
        ),
    );

    let reveal = glib::clone!(
        #[weak(rename_to = state)]
        state,
        #[weak]
        window,
        move || {
            glib::idle_add_local_once(move || reveal_pending(&state, &window));
        }
    );
    let on_items = reveal.clone();
    state.selection.connect_items_changed(glib::clone!(
        #[weak]
        window,
        move |model, pos, removed, added| {
            on_items();
            if removed > added {
                let model = model.clone();
                glib::idle_add_local_once(move || refocus_after_removal(&window, &model, pos));
            }
        }
    ));
    state.list.connect_loading_notify(move |list| {
        if !list.is_loading() {
            reveal();
        }
    });
    attach_throbber(&state.list, &spinner);

    let on_activate = glib::clone!(
        #[strong]
        state,
        #[weak]
        window,
        move |pos: u32| activate(&state, &window, pos)
    );
    let on_icons = on_activate.clone();
    icons.connect_activate(move |_, pos| on_icons(pos));
    details.connect_activate(move |_, pos| on_activate(pos));

    if let Some(id) = startup_id.filter(|id| !id.is_empty()) {
        window.set_startup_id(id);
    }
    window.present();
    focus_view(&window);
    window
}

/// The grid or column view currently shown in `window`.
fn visible_view(window: &gtk::ApplicationWindow) -> Option<gtk::Widget> {
    window
        .child()
        .and_then(|c| c.last_child())
        .and_then(|body| body.last_child())
        .and_downcast::<gtk::Overlay>()
        .and_then(|o| o.child())
        .and_downcast::<gtk::Stack>()
        .and_then(|s| s.visible_child())
        .and_then(|c| c.first_child())
}

fn focus_view(window: &gtk::ApplicationWindow) {
    if let Some(view) = visible_view(window) {
        view.grab_focus();
    }
}

/// Selects `state.reveal` once the listing contains all of it, focusing and scrolling to
/// the first. Selects what was found when the folder has finished loading without the
/// rest.
fn reveal_pending(state: &State, window: &gtk::ApplicationWindow) {
    let targets = state.reveal.borrow().clone();
    if targets.is_empty() {
        return;
    }
    let mut found = vec![None; targets.len()];
    for i in 0..state.selection.n_items() {
        let file = state
            .selection
            .item(i)
            .and_downcast::<gio::FileInfo>()
            .and_then(|info| dnd::file_of(&info));
        if let Some(k) = file.and_then(|f| targets.iter().position(|t| t.equal(&f))) {
            found[k] = Some(i);
        }
    }
    if found.contains(&None) && state.list.is_loading() {
        return;
    }
    state.reveal.borrow_mut().clear();
    let found: Vec<u32> = found.into_iter().flatten().collect();
    let Some(&pos) = found.first() else {
        return;
    };
    state.selection.select_item(pos, true);
    for &other in &found[1..] {
        state.selection.select_item(other, false);
    }
    focus_item(window, pos);
}

/// Scrolls to the item at `pos` in the visible view and gives it keyboard focus.
fn focus_item(window: &gtk::ApplicationWindow, pos: u32) {
    let flags = gtk::ListScrollFlags::FOCUS;
    match visible_view(window) {
        Some(v) if v.is::<gtk::GridView>() => v
            .downcast::<gtk::GridView>()
            .unwrap()
            .scroll_to(pos, flags, None),
        Some(v) if v.is::<gtk::ColumnView>() => v
            .downcast::<gtk::ColumnView>()
            .unwrap()
            .scroll_to(pos, None, flags, None),
        _ => {}
    }
}

/// After items were removed at `pos`: if that left focus nowhere, it goes to the item
/// now in their place, or the last one (`NAV-30`).
fn refocus_after_removal(window: &gtk::ApplicationWindow, model: &gtk::MultiSelection, pos: u32) {
    if gtk::prelude::GtkWindowExt::focus(window).is_some() {
        return;
    }
    focus_view(window);
    if let Some(last) = model.n_items().checked_sub(1) {
        focus_item(window, pos.min(last));
    }
}

fn add_actions(window: &gtk::ApplicationWindow, state: &Rc<State>, stack: &gtk::Stack) {
    type Win = gtk::ApplicationWindow;
    let simple = |name: &str, f: fn(&Win, &State)| {
        let state = state.clone();
        gio::ActionEntry::builder(name)
            .activate(move |win: &Win, _, _| f(win, &state))
            .build()
    };

    let view = gio::ActionEntry::builder("view")
        .parameter_type(Some(&String::static_variant_type()))
        .state(state::get().view().to_variant())
        .activate(glib::clone!(
            #[weak]
            stack,
            move |_: &Win, action, param| {
                let Some(name) = param.and_then(|p| p.get::<String>()) else {
                    return;
                };
                stack.set_visible_child_name(&name);
                if let Some(view) = stack.visible_child().and_then(|c| c.first_child()) {
                    view.grab_focus();
                }
                if matches!(name.as_str(), "icons" | "details") {
                    state::update(|s| s.view = Some(name.clone()));
                }
                action.set_state(&name.to_variant());
            }
        ))
        .build();

    let hidden = gio::ActionEntry::builder("toggle-hidden")
        .state(state.show_hidden.get().to_variant())
        .activate(glib::clone!(
            #[strong]
            state,
            move |_: &Win, action, _| {
                state.show_hidden.set(!state.show_hidden.get());
                action.set_state(&state.show_hidden.get().to_variant());
                let shown = state.show_hidden.get();
                state::update(|s| s.show_hidden = Some(shown));
                state.filter.changed(gtk::FilterChange::Different);
                if let Some(captions) = &state.captions {
                    captions.refresh();
                }
            }
        ))
        .build();

    let new_folder = {
        let state = state.clone();
        gio::ActionEntry::builder("new-folder")
            .activate(move |win: &Win, _, _| ask_new_folder(win, &state))
            .build()
    };

    window.add_action_entries([
        view,
        hidden,
        new_folder,
        simple("new-window", |win, s| {
            if let (Some(app), Some(dir)) = (win.application(), s.dir()) {
                open(&app, &dir);
            }
        }),
        simple("go-up", |win, s| {
            if let Some(parent) = s.dir().and_then(|d| d.parent()) {
                *s.reveal.borrow_mut() = s.dir().into_iter().collect();
                s.navigate(win, parent, true);
            }
        }),
        simple("go-back", |win, s| s.step(win, false)),
        simple("go-forward", |win, s| s.step(win, true)),
        simple("go-home", |win, s| {
            s.navigate(win, gio::File::for_path(glib::home_dir()), true)
        }),
        simple("reload", |_, s| {
            let dir = s.dir();
            if let Some(captions) = &s.captions {
                captions.clear();
            }
            s.list.set_file(None::<&gio::File>);
            s.list.set_file(dir.as_ref());
        }),
        simple("select-all", |_, s| {
            s.selection.select_all();
        }),
        simple("unselect-all", |_, s| {
            s.selection.unselect_all();
        }),
        simple("copy", |win, s| {
            clipboard::set(win, &dnd::selected_targets(&s.selection), false)
        }),
        simple("cut", |win, s| {
            clipboard::set(win, &dnd::selected_targets(&s.selection), true)
        }),
        simple("paste", |win, s| {
            if let Some(dir) = s.dir() {
                clipboard::paste(win, dir);
            }
        }),
        simple("open", |win, s| {
            open_items(s, win, &dnd::selected_infos(&s.selection))
        }),
        simple("rename", |win, s| {
            if let [file] = dnd::selected_files(&s.selection).as_slice() {
                rename::prompt(win, file.clone());
            }
        }),
        simple("trash", |win, s| trash_selection(win, s, true)),
        simple("trash-now", |win, s| trash_selection(win, s, false)),
        simple("delete", |win, s| {
            let files = dnd::selected_files(&s.selection);
            if files.is_empty() {
                return;
            }
            if s.dir().is_some_and(|d| dnd::is_recent(&d)) {
                ops::forget(Some(win.clone().upcast()), files);
            } else {
                confirm_delete(win, files);
            }
        }),
        simple("open-with", |win, s| {
            let infos = dnd::selected_infos(&s.selection);
            if let Some(content_type) = infos.first().and_then(|i| i.content_type()) {
                openwith::show(win, dnd::selected_targets(&s.selection), &content_type);
            }
        }),
        simple("properties", |win, s| {
            let mut files = dnd::selected_targets(&s.selection);
            if files.is_empty() {
                files.extend(s.dir());
            }
            properties::show(win, files);
        }),
        simple("context-menu", |win, s| {
            if let Some(view) = visible_view(win) {
                menu::popup_at_focus(win, &view, s.selection.selection().size() as u32);
            }
        }),
    ]);
}

fn ask_new_folder(window: &gtk::ApplicationWindow, state: &Rc<State>) {
    let Some(dir) = state.dir() else { return };
    let initial = "Untitled Folder";
    let (state, parent) = (state.clone(), window.clone());
    dialog::ask_name(
        window,
        "New Folder",
        initial,
        initial.chars().count() as i32,
        move |name| {
            if name.contains('/') {
                dialog::error(
                    Some(&parent),
                    &format!("Could not create “{name}”"),
                    "Folder names can't contain “/”.",
                );
                return;
            }
            let folder = dir.child(&name);
            let (state, parent) = (state.clone(), parent.clone());
            glib::spawn_future_local(async move {
                match folder.make_directory_future(glib::Priority::DEFAULT).await {
                    Ok(()) => {
                        *state.reveal.borrow_mut() = vec![folder];
                    }
                    Err(e) => dialog::error(
                        Some(&parent),
                        &format!("Could not create “{name}”"),
                        e.message(),
                    ),
                }
            });
        },
    );
}

/// `trash` asks first (`OPS-30`); `trash-now` doesn't (`OPS-37`). Recent entries are only
/// dropped from the list (`INT-12`), so nothing is asked there.
fn trash_selection(win: &gtk::ApplicationWindow, s: &State, ask: bool) {
    let files = dnd::selected_files(&s.selection);
    if files.is_empty() {
        return;
    }
    if s.dir().is_some_and(|d| dnd::is_recent(&d)) {
        ops::forget(Some(win.clone().upcast()), files);
    } else if ask {
        confirm_trash(win, files);
    } else {
        ops::trash(Some(win.clone().upcast()), files);
    }
}

fn confirm_trash(window: &gtk::ApplicationWindow, files: Vec<gio::File>) {
    let message = match files.as_slice() {
        [file] => format!(
            "Move “{}” to the trash?",
            file.basename()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        ),
        _ => format!("Move {} items to the trash?", files.len()),
    };
    let alert = gtk::AlertDialog::builder()
        .message(message)
        .buttons(["Cancel", "Move to Trash"])
        .cancel_button(0)
        .default_button(0)
        .build();
    let parent = window.clone();
    glib::spawn_future_local(async move {
        if alert.choose_future(Some(&parent)).await == Ok(1) {
            ops::trash(Some(parent.upcast()), files);
        }
    });
}

fn confirm_delete(window: &gtk::ApplicationWindow, files: Vec<gio::File>) {
    let message = match files.as_slice() {
        [file] => format!(
            "Permanently delete “{}”?",
            file.basename()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        ),
        _ => format!("Permanently delete {} items?", files.len()),
    };
    let alert = gtk::AlertDialog::builder()
        .message(message)
        .detail("This can't be undone.")
        .buttons(["Cancel", "Delete"])
        .cancel_button(0)
        .default_button(0)
        .build();
    let parent = window.clone();
    glib::spawn_future_local(async move {
        if alert.choose_future(Some(&parent)).await == Ok(1) {
            ops::delete(Some(parent.upcast()), files);
        }
    });
}

fn activate(state: &State, window: &gtk::ApplicationWindow, pos: u32) {
    if let Some(info) = state.selection.item(pos).and_downcast::<gio::FileInfo>() {
        open_items(state, window, &[info]);
    }
}

/// A single folder opens in place. With several items, folders open in new windows
/// and files open in their default apps.
fn open_items(state: &State, window: &gtk::ApplicationWindow, infos: &[gio::FileInfo]) {
    let is_dir = dnd::is_dir;
    if let [info] = infos
        && is_dir(info)
    {
        if let Some(file) = dnd::target_of(info) {
            state.navigate(window, file, true);
        }
        return;
    }
    for info in infos {
        let Some(file) = dnd::target_of(info) else {
            continue;
        };
        if is_dir(info) {
            if let Some(app) = window.application() {
                open(&app, &file);
            }
        } else {
            gtk::FileLauncher::new(Some(&file)).launch(
                Some(window),
                gio::Cancellable::NONE,
                |_| {},
            );
        }
    }
}
fn scrolled(child: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    gtk::ScrolledWindow::builder().child(child).build()
}

fn info_of(item: &glib::Object) -> (gtk::ListItem, gio::FileInfo) {
    let item = item.downcast_ref::<gtk::ListItem>().unwrap().clone();
    let info = item.item().and_downcast::<gio::FileInfo>().unwrap();
    (item, info)
}

/// Lines `text` needs at `width` px when wrapped only between words, capped at 3.
/// A word too long for one line stays on its line and gets ellipsized.
fn word_wrapped_lines(widget: &impl IsA<gtk::Widget>, text: &str, width: i32) -> u32 {
    let layout = widget.create_pango_layout(Some(text));
    layout.set_attributes(Some(&no_hyphens()));
    layout.set_width(width * pango::SCALE);
    layout.set_wrap(pango::WrapMode::Word);
    // Stop at the first line that overflows (an unbreakable word): it becomes the
    // last line, which is the one Pango ellipsizes.
    let mut lines = 0;
    for line in layout.lines_readonly() {
        lines += 1;
        if line.pixel_extents().1.width() > width {
            break;
        }
    }
    lines.clamp(1, 3)
}

fn no_hyphens() -> pango::AttrList {
    let attrs = pango::AttrList::new();
    attrs.insert(pango::AttrInt::new_insert_hyphens(false));
    attrs
}

fn icon(info: &gio::FileInfo) -> gio::Icon {
    info.icon()
        .unwrap_or_else(|| gio::ThemedIcon::new("text-x-generic").upcast())
}

/// Drag source plus a drop target that only accepts when the item is a directory.
fn add_item_dnd(
    widget: &impl IsA<gtk::Widget>,
    item: &gtk::ListItem,
    selection: &gtk::MultiSelection,
) {
    dnd::add_drag_source(widget, item, selection);
    menu::add_to_item(widget, item, selection);
    let item = item.downgrade();
    dnd::add_drop_target(widget, move || {
        let info = item.upgrade()?.item().and_downcast::<gio::FileInfo>()?;
        dnd::is_dir(&info).then(|| dnd::target_of(&info)).flatten()
    });
}

fn icon_view(
    selection: &gtk::MultiSelection,
    captions: Option<Rc<caption::Captions>>,
    sizes: Rc<zoom::Sizes>,
) -> gtk::GridView {
    let factory = gtk::SignalListItemFactory::new();
    let with_caption = captions.is_some();
    factory.connect_setup(glib::clone!(
        #[weak]
        selection,
        #[strong]
        sizes,
        move |_, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            // Fixed width like Nautilus (icon + 18px each side), so long names wrap
            // instead of widening every column.
            let size = sizes.grid.get();
            let cell = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .width_request(size + 36)
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Start)
                .build();
            cell.append(
                &gtk::Image::builder()
                    .pixel_size(size)
                    .margin_bottom(6)
                    .build(),
            );
            // Inscription, not Label: a word-wrapped Label requests the width of its
            // longest word, which would widen every grid column.
            cell.append(
                &gtk::Inscription::builder()
                    .wrap_mode(pango::WrapMode::Word)
                    .text_overflow(gtk::InscriptionOverflow::EllipsizeMiddle)
                    .xalign(0.5)
                    .valign(gtk::Align::Start)
                    .attributes(&no_hyphens())
                    .build(),
            );
            if with_caption {
                cell.append(
                    &gtk::Inscription::builder()
                        .css_classes(["caption"])
                        .text_overflow(gtk::InscriptionOverflow::EllipsizeEnd)
                        .xalign(0.5)
                        .build(),
                );
            }
            add_item_dnd(&cell, item, &selection);
            item.set_child(Some(&cell));
        }
    ));
    let bind_captions = captions.clone();
    factory.connect_bind(move |_, item| {
        let (item, info) = info_of(item);
        let image = item
            .child()
            .and_then(|c| c.first_child())
            .and_downcast::<gtk::Image>()
            .unwrap();
        let label = image
            .next_sibling()
            .and_downcast::<gtk::Inscription>()
            .unwrap();
        image.set_from_gicon(&icon(&info));
        if config::get().show_thumbnails {
            thumbnail::show(&image, &info, dnd::target_of(&info));
        }
        let name = info.display_name();
        let lines = word_wrapped_lines(&label, &name, sizes.grid.get() + 36);
        label.set_min_lines(lines);
        label.set_nat_lines(lines);
        label.set_text(Some(&name));
        label.set_tooltip_text(Some(&name));
        if let (Some(captions), Some(caption)) = (
            &bind_captions,
            label.next_sibling().and_downcast::<gtk::Inscription>(),
        ) {
            captions.bind(&caption, &info);
        }
    });
    if let Some(captions) = captions {
        factory.connect_unbind(move |_, item| {
            let caption = item
                .downcast_ref::<gtk::ListItem>()
                .and_then(|i| i.child())
                .and_then(|c| c.last_child())
                .and_downcast::<gtk::Inscription>();
            if let Some(caption) = caption {
                captions.unbind(&caption);
            }
        });
    }

    let view = gtk::GridView::new(Some(selection.clone()), Some(factory));
    view.set_max_columns(20);
    view.set_enable_rubberband(true);
    view
}

fn details_view(
    selection: &gtk::MultiSelection,
    sizes: Rc<zoom::Sizes>,
) -> (gtk::ColumnView, gtk::ColumnViewColumn) {
    let view = gtk::ColumnView::new(Some(selection.clone()));
    view.set_enable_rubberband(true);
    view.set_reorderable(false);

    let name = gtk::SignalListItemFactory::new();
    name.connect_setup(glib::clone!(
        #[weak]
        selection,
        #[strong]
        sizes,
        move |_, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            row.append(&gtk::Image::builder().pixel_size(sizes.list.get()).build());
            row.append(
                &gtk::Label::builder()
                    .xalign(0.0)
                    .ellipsize(pango::EllipsizeMode::Middle)
                    .attributes(&no_hyphens())
                    .build(),
            );
            add_item_dnd(&row, item, &selection);
            item.set_child(Some(&row));
        }
    ));
    name.connect_bind(|_, item| {
        let (item, info) = info_of(item);
        let image = item
            .child()
            .and_then(|c| c.first_child())
            .and_downcast::<gtk::Image>()
            .unwrap();
        let label = image.next_sibling().and_downcast::<gtk::Label>().unwrap();
        image.set_from_gicon(&icon(&info));
        if config::get().show_thumbnails {
            thumbnail::show(&image, &info, dnd::target_of(&info));
        }
        label.set_label(&info.display_name());
    });
    let name = gtk::ColumnViewColumn::new(Some("Name"), Some(name));
    name.set_expand(true);
    name.set_resizable(true);
    name.set_sorter(Some(&sort::by_name()));
    view.append_column(&name);

    view.append_column(&text_column(
        selection,
        "Size",
        sort::by_size(),
        |info| match info.file_type() {
            gio::FileType::Directory => String::new(),
            _ => glib::format_size(info.size() as u64).to_string(),
        },
    ));
    view.append_column(&text_column(
        selection,
        "Modified",
        sort::by_modified(),
        caption::modified_text,
    ));
    view.append_column(&text_column(
        selection,
        "Type",
        sort::by_type(),
        caption::type_text,
    ));

    view.sort_by_column(Some(&name), gtk::SortType::Ascending);
    (view, name)
}

fn text_column(
    selection: &gtk::MultiSelection,
    title: &str,
    sorter: gtk::CustomSorter,
    text: fn(&gio::FileInfo) -> String,
) -> gtk::ColumnViewColumn {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(glib::clone!(
        #[weak]
        selection,
        move |_, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let label = gtk::Label::builder()
                .xalign(0.0)
                .css_classes(["dim-label"])
                .build();
            menu::add_to_item(&label, item, &selection);
            item.set_child(Some(&label));
        }
    ));
    factory.connect_bind(move |_, item| {
        let (item, info) = info_of(item);
        item.child()
            .and_downcast::<gtk::Label>()
            .unwrap()
            .set_label(&text(&info));
    });
    let column = gtk::ColumnViewColumn::new(Some(title), Some(factory));
    column.set_resizable(true);
    column.set_sorter(Some(&sorter));
    column
}

/// The `go-back`/`go-forward` action for a mouse thumb button (`NAV-17`): GDK numbers
/// evdev `BTN_SIDE`, `BTN_EXTRA`, `BTN_FORWARD`, `BTN_BACK` as 8, 9, 10, 11.
fn mouse_nav_action(button: u32) -> Option<&'static str> {
    match button {
        8 | 11 => Some("win.go-back"),
        9 | 10 => Some("win.go-forward"),
        _ => None,
    }
}

/// A left click on empty space clears the selection, unless Ctrl or Shift is held.
/// The gesture never claims the press, so rubberband selection still starts.
///
/// A press on an item turns rubberband off until the press ends (`OPS-1`): the item's
/// drag source ignores motion for 100 ms after the press, so the view's rubberband
/// gesture would claim a quick drag first.
fn add_background_click(view: &gtk::Widget, selection: &gtk::MultiSelection) {
    let click = gtk::GestureClick::builder()
        .button(gdk::BUTTON_PRIMARY)
        .build();
    click.set_propagation_phase(gtk::PropagationPhase::Capture);
    click.connect_pressed(glib::clone!(
        #[weak]
        selection,
        move |click, _, x, y| {
            let Some(view) = click.widget() else { return };
            let on_item = !is_background(&view, view.pick(x, y, gtk::PickFlags::DEFAULT));
            view.set_property("enable-rubberband", !on_item);
            let held = click.current_event_state();
            if on_item
                || held.intersects(gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK)
            {
                return;
            }
            selection.unselect_all();
        }
    ));
    let restore = |click: &gtk::GestureClick| {
        if let Some(view) = click.widget() {
            view.set_property("enable-rubberband", true);
        }
    };
    click.connect_end(move |click, _| restore(click));
    click.connect_cancel(move |click, _| restore(click));
    view.add_controller(click);
}

/// Whether `picked` is the view's empty space rather than an item (grid `child`,
/// details `row`) or the details view's column `header`.
fn is_background(view: &gtk::Widget, picked: Option<gtk::Widget>) -> bool {
    let mut widget = picked;
    while let Some(w) = widget {
        if &w == view {
            return true;
        }
        if matches!(w.css_name().as_str(), "child" | "row" | "header") {
            return false;
        }
        widget = w.parent();
    }
    false
}

fn add_mouse_nav(window: &gtk::ApplicationWindow) {
    let click = gtk::GestureClick::builder().button(0).build();
    click.set_propagation_phase(gtk::PropagationPhase::Capture);
    click.connect_pressed(|click, _, _, _| {
        let Some(action) = mouse_nav_action(click.current_button()) else {
            click.set_state(gtk::EventSequenceState::Denied);
            return;
        };
        click.set_state(gtk::EventSequenceState::Claimed);
        if let Some(widget) = click.widget() {
            let _ = widget.activate_action(action, None);
        }
    });
    window.add_controller(click);
}

/// Shows `spinner` once loading has lasted `THROBBER_DELAY`, so quick folders don't flash it.
fn attach_throbber(list: &listing::Listing, spinner: &gtk::Spinner) {
    let timer: Rc<RefCell<Option<glib::SourceId>>> = Rc::default();
    let update = glib::clone!(
        #[weak]
        spinner,
        move |list: &listing::Listing| {
            if let Some(id) = timer.borrow_mut().take() {
                id.remove();
            }
            if list.is_loading() {
                let slot = timer.clone();
                let id = glib::timeout_add_local_once(THROBBER_DELAY, move || {
                    slot.borrow_mut().take();
                    spinner.set_visible(true);
                    spinner.set_spinning(true);
                });
                *timer.borrow_mut() = Some(id);
            } else {
                spinner.set_spinning(false);
                spinner.set_visible(false);
            }
        }
    );
    update(list);
    list.connect_loading_notify(update);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumb_buttons_navigate() {
        assert_eq!(mouse_nav_action(8), Some("win.go-back"));
        assert_eq!(mouse_nav_action(11), Some("win.go-back"));
        assert_eq!(mouse_nav_action(9), Some("win.go-forward"));
        assert_eq!(mouse_nav_action(10), Some("win.go-forward"));
        for other in [1, 2, 3, 12] {
            assert_eq!(mouse_nav_action(other), None);
        }
    }
}
