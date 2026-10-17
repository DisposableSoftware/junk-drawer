use gtk::{gdk, glib, prelude::*};

/// A small window over `parent` that Escape closes, styled like the main window.
pub fn window(parent: &gtk::ApplicationWindow, title: &str, class: &str) -> gtk::Window {
    window_for(Some(parent.upcast_ref()), title, class)
}

/// `window` for any parent, or none.
pub fn window_for(parent: Option<&gtk::Window>, title: &str, class: &str) -> gtk::Window {
    let window = gtk::Window::builder()
        .modal(true)
        .resizable(false)
        .title(title)
        .css_classes(["junk", class])
        .build();
    window.set_transient_for(parent);
    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(glib::clone!(
        #[weak]
        window,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, key, _, _| {
            if key == gdk::Key::Escape {
                window.close();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        }
    ));
    window.add_controller(keys);
    window
}

/// Asks for a name in a one-line entry filled with `initial`, with its first `select`
/// characters selected. Enter closes the box and passes the trimmed text to `accept`
/// unless it is empty; Escape cancels.
pub fn ask_name(
    parent: &gtk::ApplicationWindow,
    title: &str,
    initial: &str,
    select: i32,
    accept: impl Fn(String) + 'static,
) {
    let entry = gtk::Entry::builder().text(initial).width_chars(40).build();
    let dialog = window(parent, title, "ask-name");
    dialog.set_child(Some(&entry));
    entry.connect_activate(glib::clone!(
        #[weak]
        dialog,
        move |entry| {
            let new = entry.text().trim().to_owned();
            dialog.close();
            if !new.is_empty() {
                accept(new);
            }
        }
    ));
    dialog.present();
    entry.grab_focus();
    entry.select_region(0, select);
}

/// Shows `message` with `detail` in an alert over `parent`.
pub fn error(parent: Option<&impl IsA<gtk::Window>>, message: &str, detail: &str) {
    gtk::AlertDialog::builder()
        .message(message)
        .detail(detail)
        .build()
        .show(parent);
}
