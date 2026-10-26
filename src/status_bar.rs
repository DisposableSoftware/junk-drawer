use crate::state;
use gtk::{gio, glib, prelude::*};
use std::cell::Cell;
use std::rc::Rc;

/// `N items, SIZE` (`VIEW-39`).
pub fn text(items: u32, bytes: u64) -> String {
    let noun = if items == 1 { "item" } else { "items" };
    format!("{items} {noun}, {}", glib::format_size(bytes))
}

/// The count and the summed file sizes of `model`'s `FileInfo` items; folders add no size.
fn totals(model: &gio::ListModel) -> (u32, u64) {
    let mut bytes = 0;
    for info in model.iter::<glib::Object>().flatten() {
        let info = info.downcast_ref::<gio::FileInfo>().unwrap();
        if info.file_type() != gio::FileType::Directory {
            bytes += info.size().max(0) as u64;
        }
    }
    (model.n_items(), bytes)
}

/// Appends the status bar to `content` and adds the `toggle-status-bar` action.
/// `model` holds the listed items, after the hidden-files filter.
pub fn attach(window: &gtk::ApplicationWindow, content: &gtk::Box, model: &gio::ListModel) {
    let label = gtk::Label::builder()
        .css_classes(["status-bar"])
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .focusable(false)
        .can_target(false)
        .visible(state::get().show_status_bar())
        .build();
    content.append(&label);

    // A large folder reports thousands of changes while loading: sum once per idle.
    let pending = Rc::new(Cell::new(false));
    let refresh = glib::clone!(
        #[weak]
        label,
        #[weak]
        model,
        #[strong]
        pending,
        move || {
            if !label.get_visible() || pending.replace(true) {
                return;
            }
            let pending = pending.clone();
            glib::idle_add_local_once(move || {
                pending.set(false);
                let (items, bytes) = totals(&model);
                label.set_label(&text(items, bytes));
            });
        }
    );
    model.connect_items_changed({
        let refresh = refresh.clone();
        move |_, _, _, _| refresh()
    });
    refresh();

    let toggle = gio::SimpleAction::new("toggle-status-bar", None);
    toggle.connect_activate(move |_, _| {
        let shown = !label.get_visible();
        label.set_visible(shown);
        state::update(|s| s.show_status_bar = Some(shown));
        refresh();
    });
    window.add_action(&toggle);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_and_sizes() {
        // GLib puts a no-break space before the unit.
        let text = |items, bytes| text(items, bytes).replace('\u{a0}', " ");
        assert_eq!(text(0, 0), "0 items, 0 bytes");
        assert_eq!(text(1, 1200), "1 item, 1.2 kB");
        assert_eq!(text(120, 18_300_000), "120 items, 18.3 MB");
        assert_eq!(text(3, 4_100_000_000), "3 items, 4.1 GB");
    }

    #[test]
    fn folders_add_no_size() {
        let file = gio::FileInfo::new();
        file.set_file_type(gio::FileType::Regular);
        file.set_size(1000);
        let dir = gio::FileInfo::new();
        dir.set_file_type(gio::FileType::Directory);
        dir.set_size(4096);
        let store = gio::ListStore::new::<gio::FileInfo>();
        store.append(&file);
        store.append(&dir);
        assert_eq!(totals(store.upcast_ref()), (2, 1000));
    }
}
