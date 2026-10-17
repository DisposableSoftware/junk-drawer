use crate::keys;
use gtk::{gdk, gio, glib, prelude::*};

/// Adds the hidden keybinding panel to `overlay` and the `win.show-help` action. The next
/// key press or click hides it again and is swallowed, then `on_dismiss` runs. Call after
/// `keys::attach_local`: the window runs its newest capture controllers first, so these
/// see events before any binding does.
pub fn attach(
    window: &gtk::ApplicationWindow,
    overlay: &gtk::Overlay,
    on_dismiss: impl Fn() + Clone + 'static,
) {
    let panel = panel();
    overlay.add_overlay(&panel);

    let show = gio::ActionEntry::builder("show-help")
        .activate(glib::clone!(
            #[weak]
            panel,
            move |_: &gtk::ApplicationWindow, _, _| panel.set_visible(true)
        ))
        .build();
    window.add_action_entries([show]);

    let key = gtk::EventControllerKey::new();
    key.set_propagation_phase(gtk::PropagationPhase::Capture);
    let dismiss = on_dismiss.clone();
    key.connect_key_pressed(glib::clone!(
        #[weak]
        panel,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, keyval, _, _| {
            if !panel.is_visible() {
                return glib::Propagation::Proceed;
            }
            // By keysym rather than `KeyEvent::is_modifier`, which is false for keys from
            // virtual keyboards whose keymap has no modifier map (e.g. wtype).
            use gdk::Key;
            let modifier = matches!(
                keyval,
                Key::Shift_L
                    | Key::Shift_R
                    | Key::Control_L
                    | Key::Control_R
                    | Key::Alt_L
                    | Key::Alt_R
                    | Key::Meta_L
                    | Key::Meta_R
                    | Key::Super_L
                    | Key::Super_R
                    | Key::Hyper_L
                    | Key::Hyper_R
                    | Key::ISO_Level3_Shift
            );
            if !modifier {
                panel.set_visible(false);
                dismiss();
            }
            glib::Propagation::Stop
        }
    ));
    window.add_controller(key);

    let click = gtk::GestureClick::builder()
        .button(0)
        .propagation_phase(gtk::PropagationPhase::Capture)
        .build();
    click.connect_pressed(glib::clone!(
        #[weak]
        panel,
        move |click, _, _, _| {
            if !panel.is_visible() {
                click.set_state(gtk::EventSequenceState::Denied);
                return;
            }
            click.set_state(gtk::EventSequenceState::Claimed);
            panel.set_visible(false);
            on_dismiss();
        }
    ));
    window.add_controller(click);
}

/// Two description/keys column pairs, so every binding fits a 900×600 window.
fn panel() -> gtk::Grid {
    let grid = gtk::Grid::builder()
        .css_classes(["help"])
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .column_spacing(18)
        .row_spacing(6)
        .visible(false)
        .build();
    let rows = keys::help_rows();
    let per_column = rows.len().div_ceil(2).max(1);
    for (i, (description, keys)) in rows.into_iter().enumerate() {
        let (col, row) = ((i / per_column) as i32 * 2, (i % per_column) as i32);
        let desc = gtk::Label::builder()
            .label(description)
            .css_classes(["desc"])
            .xalign(0.0)
            .build();
        if col > 0 {
            desc.set_margin_start(30);
        }
        let keys = gtk::Label::builder()
            .label(keys)
            .css_classes(["keys"])
            .xalign(0.0)
            .build();
        grid.attach(&desc, col, row, 1, 1);
        grid.attach(&keys, col + 1, row, 1, 1);
    }
    grid
}
