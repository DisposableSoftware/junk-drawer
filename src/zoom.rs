use crate::config;
use gtk::{gdk, gio, glib, prelude::*};
use std::cell::Cell;
use std::rc::Rc;

/// Nautilus 50's zoom levels (`src/nautilus-enums.h`).
const GRID: &[i32] = &[48, 64, 96, 168, 256];
const LIST: &[i32] = &[16, 32, 64];

/// The next level past `current` in direction `dir` (+1 bigger, -1 smaller), if any.
fn step(levels: &[i32], current: i32, dir: i32) -> Option<i32> {
    if dir > 0 {
        levels.iter().copied().find(|&l| l > current)
    } else {
        levels.iter().rev().copied().find(|&l| l < current)
    }
}

/// Per-window icon sizes for the two views. The item factories read them at setup.
pub struct Sizes {
    pub grid: Cell<i32>,
    pub list: Cell<i32>,
}

impl Sizes {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            grid: Cell::new(config::get().icon_size),
            list: Cell::new(config::get().list_icon_size),
        })
    }
}

/// Adds `win.zoom-in`, `win.zoom-out`, `win.zoom-reset` and Ctrl+scroll over `stack`.
pub fn attach(
    window: &gtk::ApplicationWindow,
    stack: &gtk::Stack,
    icons: &gtk::GridView,
    name_column: &gtk::ColumnViewColumn,
    sizes: Rc<Sizes>,
) {
    // dir 0 resets to the configured size.
    let zoom = Rc::new(glib::clone!(
        #[weak]
        stack,
        #[weak]
        icons,
        #[weak]
        name_column,
        move |dir: i32| {
            let grid = stack.visible_child_name().as_deref() == Some("icons");
            let (cell, levels, configured) = if grid {
                (&sizes.grid, GRID, config::get().icon_size)
            } else {
                (&sizes.list, LIST, config::get().list_icon_size)
            };
            let new = if dir == 0 {
                configured
            } else {
                match step(levels, cell.get(), dir) {
                    Some(size) => size,
                    None => return,
                }
            };
            if new == cell.get() {
                return;
            }
            cell.set(new);
            // Setting the factory again re-runs setup, which reads the new size. It also
            // destroys the focused item, so focus goes back to the view (NAV-30).
            if grid {
                let factory = icons.factory();
                icons.set_factory(None::<&gtk::ListItemFactory>);
                icons.set_factory(factory.as_ref());
                match first_selected(icons.model()) {
                    Some(pos) => icons.scroll_to(pos, gtk::ListScrollFlags::FOCUS, None),
                    None => {
                        icons.grab_focus();
                    }
                }
            } else {
                let factory = name_column.factory();
                name_column.set_factory(None::<&gtk::ListItemFactory>);
                name_column.set_factory(factory.as_ref());
                if let Some(view) = name_column.column_view() {
                    match first_selected(view.model()) {
                        Some(pos) => view.scroll_to(pos, None, gtk::ListScrollFlags::FOCUS, None),
                        None => {
                            view.grab_focus();
                        }
                    }
                }
            }
        }
    ));

    let action = |name: &str, dir: i32| {
        let zoom = zoom.clone();
        gio::ActionEntry::builder(name)
            .activate(move |_: &gtk::ApplicationWindow, _, _| zoom(dir))
            .build()
    };
    window.add_action_entries([
        action("zoom-in", 1),
        action("zoom-out", -1),
        action("zoom-reset", 0),
    ]);

    let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
    scroll.set_propagation_phase(gtk::PropagationPhase::Capture);
    let pending = Cell::new(0.0);
    scroll.connect_scroll(move |scroll, _, dy| {
        if !scroll
            .current_event_state()
            .contains(gdk::ModifierType::CONTROL_MASK)
        {
            return glib::Propagation::Proceed;
        }
        // Touchpads send fractions; zoom once per whole step, like a wheel click.
        pending.set(pending.get() + dy);
        while pending.get().abs() >= 1.0 {
            let dir = -pending.get().signum();
            pending.set(pending.get() + dir);
            zoom(dir as i32);
        }
        glib::Propagation::Stop
    });
    stack.add_controller(scroll);
}

fn first_selected(model: Option<gtk::SelectionModel>) -> Option<u32> {
    let set = model?.selection();
    (!set.is_empty()).then(|| set.minimum())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_through_levels() {
        assert_eq!(step(GRID, 96, 1), Some(168));
        assert_eq!(step(GRID, 96, -1), Some(64));
        assert_eq!(step(GRID, 256, 1), None);
        assert_eq!(step(GRID, 48, -1), None);
        assert_eq!(step(GRID, 100, 1), Some(168));
        assert_eq!(step(GRID, 100, -1), Some(96));
        assert_eq!(step(LIST, 32, 1), Some(64));
        assert_eq!(step(LIST, 16, -1), None);
        assert_eq!(step(LIST, 500, -1), Some(64));
    }
}
