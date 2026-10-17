use crate::{config, dnd};
use gtk::{gio, glib, prelude::*};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Size,
    Modified,
    Type,
    Permissions,
}

/// The `[view] grid-caption` setting; `None` means no caption line.
pub fn kind() -> Option<Kind> {
    match config::get().grid_caption.as_str() {
        "size" => Some(Kind::Size),
        "modified" => Some(Kind::Modified),
        "type" => Some(Kind::Type),
        "permissions" => Some(Kind::Permissions),
        _ => None,
    }
}

pub fn modified_text(info: &gio::FileInfo) -> String {
    info.modification_date_time()
        .and_then(|t| t.to_local().ok())
        .and_then(|t| t.format("%Y-%m-%d %H:%M").ok())
        .map(|s| s.to_string())
        .unwrap_or_default()
}

pub fn type_text(info: &gio::FileInfo) -> String {
    info.content_type()
        .map(|ct| gio::content_type_get_description(&ct).to_string())
        .unwrap_or_default()
}

fn count_text(count: u32) -> String {
    if count == 1 {
        "1 item".into()
    } else {
        format!("{count} items")
    }
}

/// `ls -l` style, e.g. `drwxr-xr-x`, from a `unix::mode` value.
pub fn mode_text(mode: u32) -> String {
    let kind = match mode & 0o170000 {
        0o040000 => 'd',
        0o120000 => 'l',
        0o020000 => 'c',
        0o060000 => 'b',
        0o010000 => 'p',
        0o140000 => 's',
        _ => '-',
    };
    let bit = |mask: u32, c: char| if mode & mask != 0 { c } else { '-' };
    let exec = |x: u32, special: u32, set: char| match (mode & x != 0, mode & special != 0) {
        (true, true) => set,
        (false, true) => set.to_ascii_uppercase(),
        (true, false) => 'x',
        (false, false) => '-',
    };
    [
        kind,
        bit(0o400, 'r'),
        bit(0o200, 'w'),
        exec(0o100, 0o4000, 's'),
        bit(0o040, 'r'),
        bit(0o020, 'w'),
        exec(0o010, 0o2000, 's'),
        bit(0o004, 'r'),
        bit(0o002, 'w'),
        exec(0o001, 0o1000, 't'),
    ]
    .into_iter()
    .collect()
}

enum Count {
    Pending,
    Unreadable,
    Done { all: u32, visible: u32 },
}

/// Per-window caption state: folder item counts, and the captions currently bound so
/// they can be redrawn when a count arrives or hidden files are toggled.
pub struct Captions {
    kind: Kind,
    show_hidden: Rc<Cell<bool>>,
    counts: RefCell<HashMap<String, Count>>,
    bound: RefCell<Vec<(glib::WeakRef<gtk::Inscription>, gio::FileInfo)>>,
}

impl Captions {
    pub fn new(kind: Kind, show_hidden: Rc<Cell<bool>>) -> Rc<Self> {
        Rc::new(Self {
            kind,
            show_hidden,
            counts: RefCell::default(),
            bound: RefCell::default(),
        })
    }

    pub fn bind(self: &Rc<Self>, label: &gtk::Inscription, info: &gio::FileInfo) {
        self.bound
            .borrow_mut()
            .push((label.downgrade(), info.clone()));
        label.set_text(Some(&self.text(info)));
    }

    pub fn unbind(&self, label: &gtk::Inscription) {
        self.bound
            .borrow_mut()
            .retain(|(l, _)| l.upgrade().is_some_and(|l| &l != label));
    }

    pub fn refresh(self: &Rc<Self>) {
        let bound: Vec<_> = self.bound.borrow().clone();
        for (label, info) in bound {
            if let Some(label) = label.upgrade() {
                label.set_text(Some(&self.text(&info)));
            }
        }
    }

    /// Forgets folder counts, so they are counted again when next shown.
    pub fn clear(&self) {
        self.counts.borrow_mut().clear();
    }

    fn text(self: &Rc<Self>, info: &gio::FileInfo) -> String {
        match self.kind {
            Kind::Size if dnd::is_dir(info) => self.count(info),
            Kind::Size => glib::format_size(info.size() as u64).to_string(),
            Kind::Modified => modified_text(info),
            Kind::Type => type_text(info),
            Kind::Permissions => {
                if info.has_attribute("unix::mode") {
                    mode_text(info.attribute_uint32("unix::mode"))
                } else {
                    String::new()
                }
            }
        }
    }

    fn count(self: &Rc<Self>, info: &gio::FileInfo) -> String {
        let Some(dir) = dnd::target_of(info) else {
            return String::new();
        };
        let uri = dir.uri().to_string();
        match self.counts.borrow().get(&uri) {
            Some(Count::Done { all, visible }) => {
                return count_text(if self.show_hidden.get() {
                    *all
                } else {
                    *visible
                });
            }
            Some(Count::Unreadable) => return "? items".into(),
            Some(Count::Pending) => return "…".into(),
            None => {}
        }
        self.counts.borrow_mut().insert(uri.clone(), Count::Pending);
        let this = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            let count = count_children(&dir).await;
            let Some(this) = this.upgrade() else { return };
            // After a clear() nobody is waiting for this folder any more.
            if matches!(this.counts.borrow().get(&uri), Some(Count::Pending)) {
                this.counts.borrow_mut().insert(uri, count);
                this.refresh();
            }
        });
        "…".into()
    }
}

async fn count_children(dir: &gio::File) -> Count {
    let Ok(children) = dir
        .enumerate_children_future(
            "standard::is-hidden,standard::is-backup",
            gio::FileQueryInfoFlags::NONE,
            glib::Priority::LOW,
        )
        .await
    else {
        return Count::Unreadable;
    };
    let (mut all, mut visible) = (0, 0);
    loop {
        match children.next_files_future(256, glib::Priority::LOW).await {
            Ok(batch) if batch.is_empty() => break,
            Ok(batch) => {
                all += batch.len() as u32;
                visible += batch
                    .iter()
                    .filter(|i| !(i.is_hidden() || i.is_backup()))
                    .count() as u32;
            }
            Err(_) => return Count::Unreadable,
        }
    }
    Count::Done { all, visible }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts() {
        assert_eq!(count_text(0), "0 items");
        assert_eq!(count_text(1), "1 item");
        assert_eq!(count_text(12), "12 items");
    }

    #[test]
    fn sizes() {
        // GLib separates number and unit with a no-break space.
        let size = |n| glib::format_size(n).replace('\u{a0}', " ");
        assert_eq!(size(512), "512 bytes");
        assert_eq!(size(1_200), "1.2 kB");
        assert_eq!(size(3_400_000), "3.4 MB");
        assert_eq!(size(1_100_000_000), "1.1 GB");
    }

    #[test]
    fn modes() {
        assert_eq!(mode_text(0o040755), "drwxr-xr-x");
        assert_eq!(mode_text(0o100644), "-rw-r--r--");
        assert_eq!(mode_text(0o104755), "-rwsr-xr-x");
        assert_eq!(mode_text(0o041777), "drwxrwxrwt");
        assert_eq!(mode_text(0o102644), "-rw-r-Sr--");
        assert_eq!(mode_text(0o120777), "lrwxrwxrwx");
    }
}
