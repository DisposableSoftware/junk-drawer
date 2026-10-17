use crate::dialog;
use gtk::{gio, glib, prelude::*};

/// Lists the apps for `content_type`, default first, and opens `files` in the one picked.
pub fn show(parent: &gtk::ApplicationWindow, files: Vec<gio::File>, content_type: &str) {
    let apps = apps_for(content_type);
    let window = dialog::window(parent, "Open With", "open-with");
    window.set_default_width(360);
    if apps.is_empty() {
        window.set_child(Some(
            &gtk::Label::builder()
                .label("No apps can open this file")
                .margin_top(18)
                .margin_bottom(18)
                .margin_start(18)
                .margin_end(18)
                .build(),
        ));
        window.present();
        return;
    }

    let default = gio::AppInfo::default_for_type(content_type, false);
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .activate_on_single_click(false)
        .build();
    for app in &apps {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        if let Some(icon) = app.icon() {
            row.append(&gtk::Image::builder().gicon(&icon).pixel_size(32).build());
        }
        row.append(
            &gtk::Label::builder()
                .label(app.name())
                .xalign(0.0)
                .hexpand(true)
                .build(),
        );
        if default.as_ref().is_some_and(|d| d.equal(app)) {
            row.append(
                &gtk::Label::builder()
                    .label("Default")
                    .css_classes(["hint"])
                    .build(),
            );
        }
        list.append(&row);
    }
    list.connect_row_activated(glib::clone!(
        #[weak]
        window,
        #[weak]
        parent,
        move |_, row| {
            let Some(app) = apps.get(row.index() as usize).cloned() else {
                return;
            };
            window.close();
            let uris: Vec<String> = files.iter().map(|f| f.uri().to_string()).collect();
            let context = WidgetExt::display(&parent).app_launch_context();
            glib::spawn_future_local(async move {
                let uris: Vec<&str> = uris.iter().map(String::as_str).collect();
                if let Err(e) = app.launch_uris_future(&uris, Some(&context)).await {
                    dialog::error(
                        Some(&parent),
                        &format!("Could not open with {}", app.name()),
                        e.message(),
                    );
                }
            });
        }
    ));
    window.set_child(Some(
        &gtk::ScrolledWindow::builder()
            .child(&list)
            .propagate_natural_height(true)
            .max_content_height(420)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
    ));
    window.present();
    if let Some(first) = list.row_at_index(0) {
        list.select_row(Some(&first));
        first.grab_focus();
    }
}

/// The default app, then the recommended ones, then every other app that declares the
/// type, without repeats.
fn apps_for(content_type: &str) -> Vec<gio::AppInfo> {
    let mut apps: Vec<gio::AppInfo> = Vec::new();
    let candidates = gio::AppInfo::default_for_type(content_type, false)
        .into_iter()
        .chain(gio::AppInfo::recommended_for_type(content_type))
        .chain(gio::AppInfo::all_for_type(content_type));
    for app in candidates {
        if app.should_show() && !apps.iter().any(|a| a.equal(&app)) {
            apps.push(app);
        }
    }
    apps
}
