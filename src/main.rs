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
mod screenshot;
mod sidebar;
mod sort;
#[cfg(test)]
mod spec_tests;
mod state;
mod status_bar;
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
    let screenshot = cli::screenshot(std::env::args_os());
    if screenshot {
        screenshot::enable();
    }
    glib::set_prgname(Some(APP_ID));
    // Screenshot mode is a process of its own, never forwarded to a running one (`INT-33`).
    let flags = if screenshot {
        gio::ApplicationFlags::HANDLES_OPEN | gio::ApplicationFlags::NON_UNIQUE
    } else {
        gio::ApplicationFlags::HANDLES_OPEN
    };
    let app = gtk::Application::builder()
        .application_id(APP_ID)
        .flags(flags)
        .build();

    app.connect_startup(move |app| {
        theme::load(config::get());
        if let Some(display) = gtk::gdk::Display::default() {
            theme::follow_scale(&display);
        }
        // Started by D-Bus for a FileManager1 call: stay up long enough to get it.
        if app.flags().contains(gio::ApplicationFlags::IS_SERVICE) {
            app.set_inactivity_timeout(10_000);
        }
        if screenshot {
            if let Err(e) = screenshot::create() {
                eprintln!("junk: could not make the screenshot sandbox: {e}");
                std::process::exit(1);
            }
        } else {
            fm1::register(app);
        }
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

    let args: Vec<String> = std::env::args_os()
        .map(|a| a.to_string_lossy().into_owned())
        .filter(|a| !(screenshot && a == "--screenshot"))
        .collect();
    let code = app.run_with_args(&args);
    screenshot::cleanup();
    code
}
