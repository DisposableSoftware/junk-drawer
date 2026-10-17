use crate::keys;
use gtk::{gdk, gio, glib, prelude::*};

fn item(label: &str, action: &str) -> gio::MenuItem {
    let item = gio::MenuItem::new(Some(label), Some(action));
    if let Some(accel) = keys::first_accel(action) {
        item.set_attribute_value("accel", Some(&accel.to_variant()));
    }
    item
}

fn section(items: &[gio::MenuItem]) -> gio::MenuItem {
    let section = gio::Menu::new();
    for i in items {
        section.append_item(i);
    }
    gio::MenuItem::new_section(None, &section)
}

/// The menu for selected items; Rename only appears for a single item. In Recent the
/// items are list entries, so they can only be removed from it (`INT-12`).
fn items_menu(single: bool, recent: bool) -> gio::Menu {
    let menu = gio::Menu::new();
    menu.append_item(&section(&[
        item("Open", "win.open"),
        item("Open With…", "win.open-with"),
    ]));
    menu.append_item(&section(&[
        item("Cut", "win.cut"),
        item("Copy", "win.copy"),
    ]));
    let mut edit = Vec::new();
    if recent {
        edit.push(item("Remove from Recent", "win.trash"));
    } else {
        if single {
            edit.push(item("Rename…", "win.rename"));
        }
        edit.push(item("Move to Trash", "win.trash"));
        edit.push(item("Delete Permanently…", "win.delete"));
    }
    menu.append_item(&section(&edit));
    menu.append_item(&section(&[item("Properties", "win.properties")]));
    menu
}

fn folder_menu() -> gio::Menu {
    let menu = gio::Menu::new();
    menu.append_item(&section(&[item("New Folder…", "win.new-folder")]));
    menu.append_item(&section(&[
        item("Paste", "win.paste"),
        item("Select All", "win.select-all"),
    ]));
    menu.append_item(&section(&[item("Show Hidden Files", "win.toggle-hidden")]));
    let sort = gio::Menu::new();
    sort.append_item(&section(&[
        item("Name", "win.sort-by::name"),
        item("Size", "win.sort-by::size"),
        item("Modified", "win.sort-by::modified"),
        item("Type", "win.sort-by::type"),
    ]));
    sort.append_item(&section(&[item("Reversed Order", "win.sort-reversed")]));
    menu.append_submenu(Some("Sort By"), &sort);
    menu.append_item(&section(&[item("Properties", "win.properties")]));
    menu
}

/// Pops up the menu for `selected` items (or the folder menu when 0) at `x`, `y` in
/// `view`'s coordinates.
pub fn popup(view: &gtk::Widget, selected: u32, x: f64, y: f64) {
    let model = if selected == 0 {
        folder_menu()
    } else {
        let recent = view
            .root()
            .and_downcast::<gtk::Window>()
            .and_then(|w| w.title())
            .is_some_and(|t| gio::File::for_parse_name(&t).has_uri_scheme("recent"));
        items_menu(selected == 1, recent)
    };
    let popover = gtk::PopoverMenu::from_model(Some(&model));
    popover.set_parent(view);
    popover.set_has_arrow(false);
    popover.set_position(gtk::PositionType::Right);
    popover.set_valign(gtk::Align::Start);
    popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
    popover.connect_closed(|popover| {
        // Unparenting inside "closed" would destroy it while it activates the item.
        let popover = popover.clone();
        glib::idle_add_local_once(move || popover.unparent());
    });
    // Popping up inside the click handler sizes the popup before the menu is fully
    // laid out, and GTK never grows it afterwards, so the menu scrolled.
    glib::idle_add_local_once(move || popover.popup());
}

/// Right-click on an item (`widget` is its cell): select it unless it already is,
/// then show the menu for the selection.
pub fn add_to_item(
    widget: &impl IsA<gtk::Widget>,
    item: &gtk::ListItem,
    selection: &gtk::MultiSelection,
) {
    let click = gtk::GestureClick::builder()
        .button(gdk::BUTTON_SECONDARY)
        .build();
    click.connect_pressed(glib::clone!(
        #[weak]
        item,
        #[weak]
        selection,
        move |click, _, x, y| {
            click.set_state(gtk::EventSequenceState::Claimed);
            let Some(cell) = click.widget() else { return };
            let Some(view) = list_view_of(&cell) else {
                return;
            };
            let pos = item.position();
            if !selection.is_selected(pos) {
                selection.select_item(pos, true);
            }
            let point = cell
                .compute_point(&view, &gtk::graphene::Point::new(x as f32, y as f32))
                .unwrap_or_else(|| gtk::graphene::Point::new(x as f32, y as f32));
            popup(
                &view,
                selection.selection().size() as u32,
                point.x() as f64,
                point.y() as f64,
            );
        }
    ));
    widget.add_controller(click);
}

/// Right-click on empty space: clear the selection and show the folder menu.
pub fn add_to_view(view: &impl IsA<gtk::Widget>, selection: &gtk::MultiSelection) {
    let click = gtk::GestureClick::builder()
        .button(gdk::BUTTON_SECONDARY)
        .build();
    click.connect_pressed(glib::clone!(
        #[weak]
        selection,
        move |click, _, x, y| {
            click.set_state(gtk::EventSequenceState::Claimed);
            let Some(view) = click.widget() else { return };
            selection.unselect_all();
            popup(&view, 0, x, y);
        }
    ));
    view.add_controller(click);
}

/// The grid or column view that contains `widget`.
fn list_view_of(widget: &gtk::Widget) -> Option<gtk::Widget> {
    widget
        .ancestor(gtk::GridView::static_type())
        .or_else(|| widget.ancestor(gtk::ColumnView::static_type()))
}

/// For the `context-menu` binding: the menu for the selection at the focused item, or
/// the folder menu near the top-left corner when nothing is selected.
pub fn popup_at_focus(window: &gtk::ApplicationWindow, view: &gtk::Widget, selected: u32) {
    let focus = gtk::prelude::GtkWindowExt::focus(window)
        .filter(|f| f.is_ancestor(view))
        .and_then(|f| f.compute_bounds(view));
    let (x, y) = match (selected, focus) {
        (1.., Some(b)) => (
            (b.x() + b.width() / 2.0) as f64,
            (b.y() + b.height() / 2.0) as f64,
        ),
        _ => (24.0, 24.0),
    };
    popup(view, selected, x, y);
}
