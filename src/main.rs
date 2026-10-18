mod caption;
mod cli;
mod clipboard;
mod config;
mod conflict;
mod dialog;
mod dnd;
mod fm1;
mod help;
mod keys;
mod listing;
mod location;
mod menu;
mod openwith;
mod ops;
mod progress;
mod properties;
mod rename;
mod sidebar;
mod sort;
#[cfg(test)]
mod spec_tests;
mod state;
mod theme;
mod thumbnail;
mod window;
mod zoom;

use gtk::{gio, glib, prelude::*};

pub const APP_ID: &str = "io.github.DisposableSoftware.JunkDrawer";

fn main() -> glib::ExitCode {
    if let Some(text) = cli::early(std::env::args_os()) {
        print!("{text}");
        return glib::ExitCode::SUCCESS;
    }
    // Without this, windows other than ApplicationWindows get the binary name as their
    // Wayland app_id, so compositor window rules miss dialogs.
    glib::set_prgname(Some(APP_ID));
    let app = gtk::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_startup(|app| {
        theme::load(config::get());
        if let Some(display) = gtk::gdk::Display::default() {
            theme::follow_scale(&display);
        }
        // Started by D-Bus for a FileManager1 call: stay up long enough to get it.
        if app.flags().contains(gio::ApplicationFlags::IS_SERVICE) {
            app.set_inactivity_timeout(10_000);
        }
        fm1::register(app);
        let quit = gio::ActionEntry::builder("quit")
            .activate(|app: &gtk::Application, _, _| app.quit())
            .build();
        app.add_action_entries([quit]);
        keys::apply(app);
    });
    app.connect_activate(|app| window::open(app, &gio::File::for_path(glib::home_dir())));
    app.connect_open(|app, files, _| {
        for file in files {
            window::open(app, file);
        }
    });

    app.run()
}
