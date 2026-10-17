use crate::state;
use gtk::{gdk, gio, glib, prelude::*};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

const USER_DIRS: &[(glib::UserDirectory, &str)] = &[
    (glib::UserDirectory::Desktop, "user-desktop-symbolic"),
    (glib::UserDirectory::Documents, "folder-documents-symbolic"),
    (glib::UserDirectory::Downloads, "folder-download-symbolic"),
    (glib::UserDirectory::Music, "folder-music-symbolic"),
    (glib::UserDirectory::Pictures, "folder-pictures-symbolic"),
    (glib::UserDirectory::Videos, "folder-videos-symbolic"),
];

/// GTK's bookmarks file, shared with Nautilus and the file chooser (`INT-9`).
fn bookmarks_file() -> gio::File {
    gio::File::for_path(glib::user_config_dir().join("gtk-3.0").join("bookmarks"))
}

/// `(uri, label)` for each line of a bookmarks file: `uri[ label]`.
fn parse_bookmarks(text: &str) -> Vec<(String, Option<String>)> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| match line.split_once(' ') {
            Some((uri, label)) if !label.trim().is_empty() => {
                (uri.to_owned(), Some(label.trim().to_owned()))
            }
            Some((uri, _)) => (uri.to_owned(), None),
            None => (line.trim_end().to_owned(), None),
        })
        .collect()
}

/// The file with `uri` appended, or `None` if it is already bookmarked.
fn with_bookmark(text: &str, uri: &str) -> Option<String> {
    if parse_bookmarks(text).iter().any(|(u, _)| u == uri) {
        return None;
    }
    let mut out = text.to_owned();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(uri);
    out.push('\n');
    Some(out)
}

fn without_bookmark(text: &str, uri: &str) -> String {
    text.lines()
        .filter(|line| line.split(' ').next() != Some(uri))
        .map(|line| format!("{line}\n"))
        .collect()
}

async fn read_bookmarks() -> String {
    match bookmarks_file().load_contents_future().await {
        Ok((bytes, _)) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(_) => String::new(),
    }
}

/// Rewrites the bookmarks file with `edit` applied; `edit` returns `None` for no change.
async fn edit_bookmarks(edit: impl FnOnce(&str) -> Option<String>) -> Result<(), glib::Error> {
    let Some(text) = edit(&read_bookmarks().await) else {
        return Ok(());
    };
    let file = bookmarks_file();
    if let Some(dir) = file.parent() {
        match dir.make_directory_future(glib::Priority::DEFAULT).await {
            Err(e) if !e.matches(gio::IOErrorEnum::Exists) => return Err(e),
            _ => {}
        }
    }
    file.replace_contents_future(text, None, false, gio::FileCreateFlags::NONE)
        .await
        .map(|_| ())
        .map_err(|(_, e)| e)
}

fn display_name(file: &gio::File) -> String {
    match file.path() {
        Some(path) => path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned()),
        None => file
            .basename()
            .filter(|b| b.as_os_str() != "/")
            .map(|b| b.to_string_lossy().into_owned())
            .unwrap_or_else(|| file.parse_name().to_string()),
    }
}

#[derive(Clone)]
enum Target {
    Folder(gio::File),
    /// A volume that isn't mounted yet: clicking mounts it.
    Volume(gio::Volume),
}

struct Group {
    rows: gtk::Box,
    line: Option<gtk::Separator>,
    files: RefCell<Vec<(Option<gio::File>, gtk::Widget)>>,
}

impl Group {
    fn new(column: &gtk::Box, with_line: bool) -> Self {
        let line = with_line.then(|| {
            let line = gtk::Separator::new(gtk::Orientation::Horizontal);
            column.append(&line);
            line
        });
        let rows = gtk::Box::new(gtk::Orientation::Vertical, 0);
        column.append(&rows);
        Self {
            rows,
            line,
            files: RefCell::default(),
        }
    }

    fn clear(&self) {
        while let Some(child) = self.rows.first_child() {
            self.rows.remove(&child);
        }
        self.files.borrow_mut().clear();
    }

    fn update_visibility(&self) {
        let any = self.files.borrow().iter().any(|(_, w)| w.get_visible());
        self.rows.set_visible(any);
        if let Some(line) = &self.line {
            line.set_visible(any);
        }
    }
}

struct Sidebar {
    root: gtk::ScrolledWindow,
    places: Group,
    bookmarks: Group,
    volumes: Group,
    current: RefCell<Option<gio::File>>,
    open: Box<dyn Fn(gio::File)>,
    monitor: RefCell<Option<gio::FileMonitor>>,
    window: glib::WeakRef<gtk::ApplicationWindow>,
}

impl Sidebar {
    fn groups(&self) -> [&Group; 3] {
        [&self.places, &self.bookmarks, &self.volumes]
    }

    fn add_row(
        self: &Rc<Self>,
        group: &Group,
        target: Target,
        icon: &gio::Icon,
        name: &str,
        bookmark: Option<String>,
    ) -> gtk::Box {
        let row = gtk::Box::builder()
            .spacing(10)
            .css_classes(["place"])
            .build();
        row.append(&gtk::Image::from_gicon(icon));
        row.append(
            &gtk::Label::builder()
                .label(name)
                .xalign(0.0)
                .hexpand(true)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .build(),
        );
        let click = gtk::GestureClick::builder().button(0).build();
        let this = Rc::downgrade(self);
        let clicked = target.clone();
        click.connect_pressed(move |click, _, x, y| {
            let Some(this) = this.upgrade() else { return };
            // Claiming a press on the eject button would cancel its own click.
            let on_button = click
                .widget()
                .and_then(|row| row.pick(x, y, gtk::PickFlags::DEFAULT))
                .is_some_and(|w| {
                    w.is::<gtk::Button>() || w.ancestor(gtk::Button::static_type()).is_some()
                });
            if on_button {
                click.set_state(gtk::EventSequenceState::Denied);
                return;
            }
            match (click.current_button(), &clicked, &bookmark) {
                (gdk::BUTTON_PRIMARY, Target::Folder(dir), _) => (this.open)(dir.clone()),
                (gdk::BUTTON_PRIMARY, Target::Volume(volume), _) => this.mount(volume),
                (gdk::BUTTON_SECONDARY, _, Some(uri)) => {
                    if let Some(row) = click.widget() {
                        bookmark_menu(&row, uri, x, y);
                    }
                }
                _ => return,
            }
            click.set_state(gtk::EventSequenceState::Claimed);
        });
        row.add_controller(click);
        group.rows.append(&row);
        let file = match target {
            Target::Folder(dir) => Some(dir),
            Target::Volume(_) => None,
        };
        let current = self.current.borrow();
        if let (Some(file), Some(current)) = (&file, &*current)
            && file.equal(current)
        {
            row.add_css_class("current");
        }
        group.files.borrow_mut().push((file, row.clone().upcast()));
        row
    }

    fn fill_places(self: &Rc<Self>) {
        let home = glib::home_dir();
        let icon = |name: &str| gio::ThemedIcon::new(name).upcast::<gio::Icon>();
        // Only when gvfs serves `recent:` (`INT-12`).
        if gio::Vfs::default()
            .supported_uri_schemes()
            .iter()
            .any(|s| s == "recent")
        {
            self.add_row(
                &self.places,
                Target::Folder(gio::File::for_uri("recent:///")),
                &icon("document-open-recent-symbolic"),
                "Recent",
                None,
            );
        }
        self.add_row(
            &self.places,
            Target::Folder(gio::File::for_path(&home)),
            &icon("user-home-symbolic"),
            "Home",
            None,
        );
        for (dir, icon_name) in USER_DIRS {
            let Some(path) = glib::user_special_dir(*dir).filter(|p| *p != home) else {
                continue;
            };
            let file = gio::File::for_path(&path);
            let row = self.add_row(
                &self.places,
                Target::Folder(file.clone()),
                &icon(icon_name),
                &display_name(&file),
                None,
            );
            // Shown once it is known to exist (`INT-8`).
            row.set_visible(false);
            let this = Rc::downgrade(self);
            glib::spawn_future_local(async move {
                let exists = file
                    .query_info_future(
                        "standard::type",
                        gio::FileQueryInfoFlags::NONE,
                        glib::Priority::DEFAULT,
                    )
                    .await
                    .is_ok_and(|i| i.file_type() == gio::FileType::Directory);
                row.set_visible(exists);
                if let Some(this) = this.upgrade() {
                    this.places.update_visibility();
                }
            });
        }
        self.add_row(
            &self.places,
            Target::Folder(gio::File::for_path("/")),
            &icon("drive-harddisk-symbolic"),
            "File System",
            None,
        );
        self.places.update_visibility();
    }

    fn fill_bookmarks(self: &Rc<Self>) {
        let this = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            let text = read_bookmarks().await;
            let Some(this) = this.upgrade() else { return };
            this.bookmarks.clear();
            for (uri, label) in parse_bookmarks(&text) {
                let file = gio::File::for_uri(&uri);
                let icon = if file.has_uri_scheme("file") {
                    "folder-symbolic"
                } else {
                    "folder-remote-symbolic"
                };
                let name = label.unwrap_or_else(|| display_name(&file));
                this.add_row(
                    &this.bookmarks,
                    Target::Folder(file),
                    &gio::ThemedIcon::new(icon).upcast(),
                    &name,
                    Some(uri),
                );
            }
            this.bookmarks.update_visibility();
        });
    }

    fn fill_volumes(self: &Rc<Self>) {
        self.volumes.clear();
        let monitor = gio::VolumeMonitor::get();
        for mount in monitor.mounts() {
            if mount.is_shadowed() {
                continue;
            }
            let row = self.add_row(
                &self.volumes,
                Target::Folder(mount.root()),
                &mount.symbolic_icon(),
                &mount.name(),
                None,
            );
            if mount.can_eject() || mount.can_unmount() {
                let eject = gtk::Button::builder()
                    .icon_name("media-eject-symbolic")
                    .has_frame(false)
                    .focusable(false)
                    .focus_on_click(false)
                    .valign(gtk::Align::Center)
                    .css_classes(["eject"])
                    .build();
                let this = Rc::downgrade(self);
                eject.connect_clicked(move |_| {
                    if let Some(this) = this.upgrade() {
                        this.eject(&mount);
                    }
                });
                row.append(&eject);
            }
        }
        for volume in monitor.volumes() {
            if volume.get_mount().is_some() || !volume.can_mount() {
                continue;
            }
            let row = self.add_row(
                &self.volumes,
                Target::Volume(volume.clone()),
                &volume.symbolic_icon(),
                &volume.name(),
                None,
            );
            row.add_css_class("unmounted");
        }
        self.volumes.update_visibility();
    }

    fn mount(self: &Rc<Self>, volume: &gio::Volume) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        let op = gtk::MountOperation::new(Some(&window));
        let (this, volume) = (Rc::downgrade(self), volume.clone());
        glib::spawn_future_local(async move {
            let result = volume
                .mount_future(gio::MountMountFlags::NONE, Some(&op))
                .await;
            match result {
                Ok(()) => {
                    if let (Some(this), Some(mount)) = (this.upgrade(), volume.get_mount()) {
                        (this.open)(mount.root());
                    }
                }
                Err(e) => error(&window, &format!("Could not mount “{}”", volume.name()), &e),
            }
        });
    }

    /// Ejects the drive when it can be ejected, otherwise unmounts (`NAV-43`).
    fn eject(&self, mount: &gio::Mount) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        let op = gtk::MountOperation::new(Some(&window));
        let mount = mount.clone();
        glib::spawn_future_local(async move {
            let flags = gio::MountUnmountFlags::NONE;
            let result = if mount.can_eject() {
                mount.eject_with_operation_future(flags, Some(&op)).await
            } else {
                mount.unmount_with_operation_future(flags, Some(&op)).await
            };
            if let Err(e) = result {
                error(&window, &format!("Could not eject “{}”", mount.name()), &e);
            }
        });
    }

    /// Moves the window to Home when it shows a folder on `mount`, which is going away.
    fn leave(&self, mount: &gio::Mount) {
        let root = mount.root();
        let inside = self
            .current
            .borrow()
            .as_ref()
            .is_some_and(|dir| dir.equal(&root) || dir.has_prefix(&root));
        if inside {
            (self.open)(gio::File::for_path(glib::home_dir()));
        }
    }

    fn set_current(&self, dir: &gio::File) {
        *self.current.borrow_mut() = Some(dir.clone());
        for group in self.groups() {
            for (file, row) in group.files.borrow().iter() {
                if file.as_ref().is_some_and(|f| f.equal(dir)) {
                    row.add_css_class("current");
                } else {
                    row.remove_css_class("current");
                }
            }
        }
    }
}

fn bookmark_menu(row: &gtk::Widget, uri: &str, x: f64, y: f64) {
    let menu = gio::Menu::new();
    let item = gio::MenuItem::new(Some("Remove Bookmark"), None);
    item.set_action_and_target_value(Some("win.remove-bookmark"), Some(&uri.to_variant()));
    menu.append_item(&item);
    let popover = gtk::PopoverMenu::from_model(Some(&menu));
    popover.set_parent(row);
    popover.set_has_arrow(false);
    popover.set_position(gtk::PositionType::Right);
    popover.set_valign(gtk::Align::Start);
    popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
    popover.connect_closed(|popover| {
        let popover = popover.clone();
        glib::idle_add_local_once(move || popover.unparent());
    });
    // See `menu::popup`: popping up inside the click handler sizes it short.
    glib::idle_add_local_once(move || popover.popup());
}

fn report(window: &gtk::ApplicationWindow, result: Result<(), glib::Error>) {
    if let Err(e) = result {
        error(window, "Could not update the bookmarks", &e);
    }
}

fn error(window: &gtk::ApplicationWindow, message: &str, e: &glib::Error) {
    // The user cancelled, or a dialog already said what went wrong.
    if e.matches(gio::IOErrorEnum::FailedHandled) || e.matches(gio::IOErrorEnum::Cancelled) {
        return;
    }
    gtk::AlertDialog::builder()
        .message(message)
        .detail(e.message())
        .build()
        .show(Some(window));
}

/// Puts the sidebar at the start of `body` and adds the `toggle-sidebar`,
/// `add-bookmark` and `remove-bookmark` actions. `open` navigates the window.
pub fn attach(
    window: &gtk::ApplicationWindow,
    body: &gtk::Box,
    open: impl Fn(gio::File) + 'static,
) {
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(["places"])
        .build();
    let root = gtk::ScrolledWindow::builder()
        // As wide as the widest row, up to 280px with the 6px padding (LOOK-12).
        // With Never, GTK ignores max-content-width; External has no scrollbar either.
        .hscrollbar_policy(gtk::PolicyType::External)
        .propagate_natural_width(true)
        .max_content_width(274)
        .child(&column)
        .css_classes(["sidebar"])
        .focusable(false)
        .hexpand(false)
        .visible(state::get().show_sidebar())
        .build();
    body.prepend(&root);

    let sidebar = Rc::new(Sidebar {
        places: Group::new(&column, false),
        bookmarks: Group::new(&column, true),
        volumes: Group::new(&column, true),
        root,
        current: RefCell::default(),
        open: Box::new(open),
        monitor: RefCell::default(),
        window: window.downgrade(),
    });
    sidebar.fill_places();
    sidebar.fill_bookmarks();
    sidebar.fill_volumes();

    if let Ok(monitor) =
        bookmarks_file().monitor_file(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE)
    {
        let this = Rc::downgrade(&sidebar);
        monitor.connect_changed(move |_, _, _, event| {
            let done = matches!(
                event,
                gio::FileMonitorEvent::ChangesDoneHint
                    | gio::FileMonitorEvent::Deleted
                    | gio::FileMonitorEvent::Created
            );
            if let (true, Some(this)) = (done, this.upgrade()) {
                this.fill_bookmarks();
            }
        });
        *sidebar.monitor.borrow_mut() = Some(monitor);
    }

    let volumes = gio::VolumeMonitor::get();
    let refill = |this: &Weak<Sidebar>| {
        if let Some(this) = this.upgrade() {
            this.fill_volumes();
        }
    };
    let leave = |this: &Weak<Sidebar>, mount: &gio::Mount| {
        if let Some(this) = this.upgrade() {
            this.leave(mount);
        }
    };
    let handlers = [
        volumes.connect_mount_added(glib::clone!(
            #[strong(rename_to = this)]
            Rc::downgrade(&sidebar),
            move |_, _| refill(&this)
        )),
        volumes.connect_mount_removed(glib::clone!(
            #[strong(rename_to = this)]
            Rc::downgrade(&sidebar),
            move |_, mount| {
                leave(&this, mount);
                refill(&this);
            }
        )),
        volumes.connect_mount_changed(glib::clone!(
            #[strong(rename_to = this)]
            Rc::downgrade(&sidebar),
            move |_, _| refill(&this)
        )),
        volumes.connect_volume_added(glib::clone!(
            #[strong(rename_to = this)]
            Rc::downgrade(&sidebar),
            move |_, _| refill(&this)
        )),
        volumes.connect_volume_removed(glib::clone!(
            #[strong(rename_to = this)]
            Rc::downgrade(&sidebar),
            move |_, _| refill(&this)
        )),
        volumes.connect_volume_changed(glib::clone!(
            #[strong(rename_to = this)]
            Rc::downgrade(&sidebar),
            move |_, _| refill(&this)
        )),
        // Leave before the unmount, so this window doesn't keep the volume busy.
        volumes.connect_mount_pre_unmount(glib::clone!(
            #[strong(rename_to = this)]
            Rc::downgrade(&sidebar),
            move |_, mount| leave(&this, mount)
        )),
    ];
    let handlers = RefCell::new(Some(handlers));
    window.connect_destroy(move |_| {
        for id in handlers.take().into_iter().flatten() {
            volumes.disconnect(id);
        }
    });

    let current =
        |window: &gtk::ApplicationWindow| window.title().map(|t| gio::File::for_parse_name(&t));
    if let Some(dir) = current(window) {
        sidebar.set_current(&dir);
    }
    let this = Rc::downgrade(&sidebar);
    window.connect_title_notify(move |window| {
        if let (Some(this), Some(dir)) = (this.upgrade(), current(window)) {
            this.set_current(&dir);
        }
    });

    let toggle = gio::SimpleAction::new("toggle-sidebar", None);
    toggle.connect_activate(glib::clone!(
        #[strong]
        sidebar,
        move |_, _| {
            let shown = !sidebar.root.get_visible();
            sidebar.root.set_visible(shown);
            state::update(|s| s.show_sidebar = Some(shown));
        }
    ));
    window.add_action(&toggle);

    let add = gio::SimpleAction::new("add-bookmark", None);
    add.connect_activate(glib::clone!(
        #[weak]
        window,
        move |_, _| {
            let Some(dir) = current(&window) else { return };
            let uri = dir.uri().to_string();
            glib::spawn_future_local(async move {
                let result = edit_bookmarks(|text| with_bookmark(text, &uri)).await;
                report(&window, result);
            });
        }
    ));
    window.add_action(&add);

    let remove = gio::SimpleAction::new("remove-bookmark", Some(glib::VariantTy::STRING));
    remove.connect_activate(glib::clone!(
        #[weak]
        window,
        move |_, uri| {
            let Some(uri) = uri.and_then(|u| u.get::<String>()) else {
                return;
            };
            glib::spawn_future_local(async move {
                let result = edit_bookmarks(|text| Some(without_bookmark(text, &uri))).await;
                report(&window, result);
            });
        }
    ));
    window.add_action(&remove);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bookmarks_file_format() {
        let text = "file:///home/u/Music Tunes\nfile:///srv/data\n\nsftp://host/srv  Server  \n";
        assert_eq!(
            parse_bookmarks(text),
            [
                ("file:///home/u/Music".to_owned(), Some("Tunes".to_owned())),
                ("file:///srv/data".to_owned(), None),
                ("sftp://host/srv".to_owned(), Some("Server".to_owned())),
            ]
        );
        assert_eq!(with_bookmark(text, "file:///srv/data"), None);
        assert_eq!(
            with_bookmark("file:///a A", "file:///b").as_deref(),
            Some("file:///a A\nfile:///b\n")
        );
        assert_eq!(
            with_bookmark("", "file:///b").as_deref(),
            Some("file:///b\n")
        );
        assert_eq!(
            without_bookmark("file:///a A\nfile:///ab\nfile:///b\n", "file:///a"),
            "file:///ab\nfile:///b\n"
        );
    }

    #[test]
    fn names() {
        assert_eq!(display_name(&gio::File::for_path("/srv/data")), "data");
        assert_eq!(display_name(&gio::File::for_path("/")), "/");
    }
}
