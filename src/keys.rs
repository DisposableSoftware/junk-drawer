use crate::config;
use gtk::{gdk, gio, glib, prelude::*};
use std::collections::HashMap;
use std::sync::OnceLock;

pub type Binding = (
    &'static str,
    &'static str,
    &'static str,
    &'static [&'static str],
);

/// Config name, GAction, description for the help overlay, default accelerators. The
/// config name is the key in the `[keybindings]` section of config.ini.
pub const BINDINGS: &[Binding] = &[
    ("new-window", "win.new-window", "New window", &["<Ctrl>n"]),
    (
        "close-window",
        "window.close",
        "Close window",
        &["<Ctrl>w", "q"],
    ),
    ("quit", "app.quit", "Quit", &["<Ctrl>q"]),
    ("open", "win.open", "Open", &["Return", "KP_Enter", "space"]),
    ("rename", "win.rename", "Rename", &["F2"]),
    ("view-icons", "win.view::icons", "Icon view", &["<Ctrl>1"]),
    (
        "view-details",
        "win.view::details",
        "Details view",
        &["<Ctrl>2"],
    ),
    (
        "toggle-hidden",
        "win.toggle-hidden",
        "Toggle hidden files",
        &["<Ctrl>h"],
    ),
    ("go-up", "win.go-up", "Go up", &["<Alt>Up", "BackSpace"]),
    ("go-back", "win.go-back", "Back", &["<Alt>Left"]),
    ("go-forward", "win.go-forward", "Forward", &["<Alt>Right"]),
    ("go-home", "win.go-home", "Home folder", &["<Alt>Home"]),
    ("reload", "win.reload", "Reload", &["F5", "<Ctrl>r"]),
    ("select-all", "win.select-all", "Select all", &["<Ctrl>a"]),
    (
        "unselect-all",
        "win.unselect-all",
        "Select none",
        &["Escape"],
    ),
    ("copy", "win.copy", "Copy", &["<Ctrl>c"]),
    ("cut", "win.cut", "Cut", &["<Ctrl>x"]),
    ("paste", "win.paste", "Paste", &["<Ctrl>v"]),
    ("trash", "win.trash", "Move to trash", &["Delete"]),
    (
        "delete",
        "win.delete",
        "Delete permanently",
        &["<Shift>Delete"],
    ),
    (
        "new-folder",
        "win.new-folder",
        "New folder",
        &["<Ctrl><Shift>n"],
    ),
    ("open-with", "win.open-with", "Open with", &[]),
    (
        "properties",
        "win.properties",
        "Properties",
        &["<Ctrl>i", "<Alt>Return"],
    ),
    (
        "context-menu",
        "win.context-menu",
        "Context menu",
        &["Menu", "<Shift>F10"],
    ),
    (
        "zoom-in",
        "win.zoom-in",
        "Zoom in",
        &["<Ctrl>plus", "<Ctrl>equal", "<Ctrl>KP_Add"],
    ),
    (
        "zoom-out",
        "win.zoom-out",
        "Zoom out",
        &["<Ctrl>minus", "<Ctrl>KP_Subtract"],
    ),
    (
        "zoom-reset",
        "win.zoom-reset",
        "Reset zoom",
        &["<Ctrl>0", "<Ctrl>KP_0"],
    ),
    ("location", "win.location", "Location", &["<Ctrl>l"]),
    (
        "toggle-sidebar",
        "win.toggle-sidebar",
        "Toggle sidebar",
        &["F9"],
    ),
    (
        "add-bookmark",
        "win.add-bookmark",
        "Bookmark this folder",
        &["<Ctrl>d"],
    ),
    ("sort-by-name", "win.sort-by::name", "Sort by name", &[]),
    ("sort-by-size", "win.sort-by::size", "Sort by size", &[]),
    (
        "sort-by-modified",
        "win.sort-by::modified",
        "Sort by modified",
        &[],
    ),
    ("sort-by-type", "win.sort-by::type", "Sort by type", &[]),
    (
        "sort-reversed",
        "win.sort-reversed",
        "Reverse sort order",
        &[],
    ),
    (
        "show-help",
        "win.show-help",
        "Keyboard shortcuts",
        &["question"],
    ),
];

/// Defaults with config overrides applied. Computed once, so config warnings are
/// printed once.
fn resolved() -> &'static [(&'static Binding, Vec<String>)] {
    static RESOLVED: OnceLock<Vec<(&Binding, Vec<String>)>> = OnceLock::new();
    RESOLVED.get_or_init(|| resolve(&config::get().keybindings))
}

fn resolve(overrides: &HashMap<String, Vec<String>>) -> Vec<(&'static Binding, Vec<String>)> {
    for name in overrides.keys() {
        if !BINDINGS.iter().any(|(n, _, _, _)| n == name) {
            glib::g_warning!("junk", "config.ini: unknown keybinding '{name}'");
        }
    }
    BINDINGS
        .iter()
        .map(|binding| {
            let (name, _, _, defaults) = binding;
            let accels = match overrides.get(*name) {
                Some(custom) => custom
                    .iter()
                    .filter(|a| {
                        let ok = gtk::accelerator_parse(a.as_str()).is_some();
                        if !ok {
                            glib::g_warning!(
                                "junk",
                                "config.ini: bad accelerator '{a}' for {name}"
                            );
                        }
                        ok
                    })
                    .cloned()
                    .collect(),
                None => defaults.iter().map(|a| a.to_string()).collect(),
            };
            (binding, accels)
        })
        .collect()
}

/// The first configured accelerator of the binding for `action`, for menu hints.
pub fn first_accel(action: &str) -> Option<String> {
    resolved()
        .iter()
        .find(|((_, a, _, _), _)| *a == action)
        .and_then(|(_, accels)| accels.first().cloned())
}

/// (description, key labels) for each bound action, in `BINDINGS` order.
pub fn help_rows() -> Vec<(&'static str, String)> {
    rows_for(resolved())
}

fn rows_for(resolved: &[(&'static Binding, Vec<String>)]) -> Vec<(&'static str, String)> {
    resolved
        .iter()
        .filter(|(_, accels)| !accels.is_empty())
        .map(|((_, _, description, _), accels)| {
            let labels: Vec<String> = accels
                .iter()
                .filter_map(|a| gtk::accelerator_parse(a.as_str()))
                .map(|(key, mods)| gtk::accelerator_get_label(key, mods).to_string())
                .collect();
            (*description, labels.join(", "))
        })
        .collect()
}

/// Keys without Ctrl/Alt/Super (q, space, Delete, F2…) would also fire while typing
/// in an entry, so they are bound only on the file views instead of app-wide.
fn is_global(accel: &str) -> bool {
    let global = gdk::ModifierType::CONTROL_MASK
        | gdk::ModifierType::ALT_MASK
        | gdk::ModifierType::SUPER_MASK
        | gdk::ModifierType::META_MASK;
    gtk::accelerator_parse(accel).is_some_and(|(_, mods)| mods.intersects(global))
}

pub fn apply(app: &gtk::Application) {
    for ((_, action, _, _), accels) in resolved() {
        let global: Vec<&str> = accels
            .iter()
            .map(String::as_str)
            .filter(|a| is_global(a))
            .collect();
        app.set_accels_for_action(action, &global);
    }
}

/// Attaches the non-global bindings to `window`. They are handled at window level, so
/// they keep working when focus is lost (entering a folder destroys the focused item),
/// and are passed through while a text field has focus.
pub fn attach_local(window: &gtk::ApplicationWindow) {
    let controller = gtk::ShortcutController::new();
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    for ((_, action, _, _), accels) in resolved() {
        let Ok((name, target)) = gio::Action::parse_detailed_name(action) else {
            continue;
        };
        for accel in accels.iter().filter(|a| !is_global(a)) {
            let Some(trigger) = gtk::ShortcutTrigger::parse_string(accel) else {
                continue;
            };
            let (name, target) = (name.clone(), target.clone());
            let callback = gtk::CallbackAction::new(move |widget, _| {
                let typing = widget
                    .downcast_ref::<gtk::Window>()
                    .and_then(GtkWindowExt::focus)
                    .is_some_and(|f| f.is::<gtk::Editable>());
                if typing {
                    return glib::Propagation::Proceed;
                }
                let _ = widget.activate_action(&name, target.as_ref());
                glib::Propagation::Stop
            });
            controller.add_shortcut(gtk::Shortcut::new(Some(trigger), Some(callback)));
        }
    }
    window.add_controller(controller);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One test: GTK may only be initialised on one thread, and needs a display.
    #[test]
    fn accelerators() {
        if gtk::init().is_err() {
            eprintln!("skipping: no display");
            return;
        }
        assert!(is_global("<Ctrl>w"));
        assert!(is_global("<Alt>Up"));
        assert!(!is_global("q"));
        assert!(!is_global("<Shift>Delete"));
        assert!(!is_global("F2"));
        for (name, _, _, accels) in BINDINGS {
            for a in *accels {
                assert!(gtk::accelerator_parse(*a).is_some(), "{name}: {a}");
            }
        }

        let overrides = HashMap::from([
            ("go-home".to_owned(), vec!["<Alt>h".to_owned()]),
            ("trash".to_owned(), vec![]),
        ]);
        let rows = rows_for(&resolve(&overrides));
        let row = |d: &str| {
            rows.iter()
                .find(|(desc, _)| *desc == d)
                .map(|(_, k)| k.as_str())
        };
        assert_eq!(row("Home folder"), Some("Alt+H"));
        assert_eq!(row("Move to trash"), None);
        assert_eq!(row("New window"), Some("Ctrl+N"));
        assert_eq!(row("Reload"), Some("F5, Ctrl+R"));
        assert_eq!(row("Keyboard shortcuts"), Some("?"));
        assert_eq!(row("Open with"), None);
        let unbound_by_default = BINDINGS.iter().filter(|b| b.3.is_empty()).count();
        assert_eq!(rows.len(), BINDINGS.len() - 1 - unbound_by_default);
    }
}
