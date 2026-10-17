use crate::{caption, dialog};
use gtk::{gio, glib, prelude::*};

const ATTRS: &str = "standard::display-name,standard::type,standard::size,standard::content-type,\
time::modified,unix::mode,owner::user";
const PRIO: glib::Priority = glib::Priority::DEFAULT;

/// Read-only details of `files` (one item, or several summed up).
pub fn show(parent: &gtk::ApplicationWindow, files: Vec<gio::File>) {
    let grid = gtk::Grid::builder()
        .column_spacing(18)
        .row_spacing(6)
        .margin_top(18)
        .margin_bottom(18)
        .margin_start(18)
        .margin_end(18)
        .build();
    let window = dialog::window(parent, "Properties", "properties");
    window.set_modal(false);
    window.set_default_width(420);
    window.set_child(Some(&grid));
    window.present();
    glib::spawn_future_local(async move { fill(&grid, &files).await });
}

/// Appends a key/value row and returns the value label.
fn row(grid: &gtk::Grid, key: &str, value: &str) -> gtk::Label {
    let mut n = 0;
    while grid.child_at(0, n).is_some() {
        n += 1;
    }
    let key = gtk::Label::builder()
        .label(key)
        .css_classes(["key"])
        .xalign(1.0)
        .yalign(0.0)
        .build();
    let value = gtk::Label::builder()
        .label(value)
        .xalign(0.0)
        .wrap(true)
        .wrap_mode(gtk::pango::WrapMode::WordChar)
        .max_width_chars(40)
        .build();
    grid.attach(&key, 0, n, 1, 1);
    grid.attach(&value, 1, n, 1, 1);
    value
}

async fn fill(grid: &gtk::Grid, files: &[gio::File]) {
    if let [file] = files {
        let info = match file
            .query_info_future(ATTRS, gio::FileQueryInfoFlags::NONE, PRIO)
            .await
        {
            Ok(info) => info,
            Err(e) => {
                row(grid, "Error", e.message());
                return;
            }
        };
        row(grid, "Name", &info.display_name());
        row(grid, "Type", &caption::type_text(&info));
        if let Some(parent) = file.parent() {
            row(grid, "Location", &parent.parse_name());
        }
        if info.file_type() == gio::FileType::Directory {
            let contents = row(grid, "Contents", "…");
            add_dates_and_owner(grid, &info);
            measure(&contents, files, false).await;
        } else {
            let size = info.size() as u64;
            row(grid, "Size", &size_text(size));
            add_dates_and_owner(grid, &info);
        }
    } else {
        row(grid, "Name", &format!("{} items", files.len()));
        let size = row(grid, "Size", "…");
        measure(&size, files, true).await;
    }
}

fn add_dates_and_owner(grid: &gtk::Grid, info: &gio::FileInfo) {
    row(grid, "Modified", &caption::modified_text(info));
    if info.has_attribute("unix::mode") {
        row(
            grid,
            "Permissions",
            &caption::mode_text(info.attribute_uint32("unix::mode")),
        );
    }
    if let Some(owner) = info.attribute_string("owner::user") {
        row(grid, "Owner", &owner);
    }
}

/// Walks `roots` without following symlinks and shows "N items, totalling X" in `label`
/// as it goes. With `count_roots` the roots themselves are counted (a selection);
/// otherwise only what is inside them (a folder's contents). Stops if the window closes.
async fn measure(label: &gtk::Label, roots: &[gio::File], count_roots: bool) {
    let label = label.downgrade();
    let (mut items, mut bytes) = (0u64, 0u64);
    let mut dirs = Vec::new();
    for root in roots {
        let Ok(info) = root
            .query_info_future(
                "standard::type,standard::size",
                gio::FileQueryInfoFlags::NOFOLLOW_SYMLINKS,
                PRIO,
            )
            .await
        else {
            continue;
        };
        if count_roots {
            items += 1;
            bytes += info.size() as u64;
        }
        if info.file_type() == gio::FileType::Directory {
            dirs.push(root.clone());
        }
    }
    let show = |items: u64, bytes: u64, done: bool| {
        let Some(label) = label.upgrade() else {
            return false;
        };
        let noun = if items == 1 { "item" } else { "items" };
        let more = if done { "" } else { "…" };
        label.set_label(&format!(
            "{items} {noun}, totalling {}{more}",
            glib::format_size(bytes)
        ));
        true
    };
    while let Some(dir) = dirs.pop() {
        let Ok(children) = dir
            .enumerate_children_future(
                "standard::name,standard::type,standard::size",
                gio::FileQueryInfoFlags::NOFOLLOW_SYMLINKS,
                glib::Priority::LOW,
            )
            .await
        else {
            continue;
        };
        while let Ok(batch) = children.next_files_future(256, glib::Priority::LOW).await {
            if batch.is_empty() {
                break;
            }
            for info in batch {
                items += 1;
                bytes += info.size() as u64;
                if info.file_type() == gio::FileType::Directory {
                    dirs.push(dir.child(info.name()));
                }
            }
            if !show(items, bytes, false) {
                return;
            }
        }
    }
    show(items, bytes, true);
}

/// `1.2 kB (1,234 bytes)`-style, without repeating the byte count below 1 kB.
fn size_text(size: u64) -> String {
    let short = glib::format_size(size);
    if size < 1000 {
        short.to_string()
    } else {
        format!("{short} ({size} bytes)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_text_skips_repeated_bytes() {
        assert_eq!(size_text(6), "6 bytes");
        assert_eq!(
            size_text(3000).replace('\u{a0}', " "),
            "3.0 kB (3000 bytes)"
        );
    }
}
