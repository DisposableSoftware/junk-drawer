//! `--screenshot` mode: one window on a sandbox of dummy files that nothing can leave
//! or change (`INT-33`, `INT-34`, `NAV-45`, `NAV-46`, `OPS-38`, `CONF-24`).
use gtk::{gdk, gio, glib, prelude::*};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

static ON: AtomicBool = AtomicBool::new(false);

pub const DIR: &str = "/tmp/junk-screenshot";

/// 2025-06-01 12:00 UTC; the items are spread over the days after it.
const BASE_TIME: u64 = 1_748_779_200;

pub fn enable() {
    ON.store(true, Ordering::Relaxed);
}

pub fn enabled() -> bool {
    ON.load(Ordering::Relaxed)
}

pub fn sandbox() -> gio::File {
    gio::File::for_path(DIR)
}

/// Window actions that screenshot mode turns into no-ops: they would leave the sandbox
/// or change a file (`NAV-45`, `OPS-38`).
const LOCKED: &[&str] = &[
    "go-up",
    "go-home",
    "go-back",
    "go-forward",
    "new-window",
    "location",
    "cut",
    "copy",
    "paste",
    "rename",
    "new-folder",
    "trash",
    "trash-now",
    "delete",
    "open-with",
    "add-bookmark",
    "remove-bookmark",
];

/// Replaces the `LOCKED` actions of `window` with ones that do nothing, so menus and
/// keys that reach them still look the same.
pub fn lock_actions(window: &gtk::ApplicationWindow) {
    for name in LOCKED {
        let Some(old) = window.lookup_action(name) else {
            continue;
        };
        let none = gio::SimpleAction::new(name, old.parameter_type().as_deref());
        window.remove_action(name);
        window.add_action(&none);
    }
}

/// Makes the sandbox afresh. Anything already at `DIR` is removed first, unless it is
/// not a plain folder.
pub fn create() -> std::io::Result<()> {
    create_in(Path::new(DIR))
}

pub fn cleanup() {
    if enabled() {
        let _ = remove(Path::new(DIR));
    }
}

fn remove(dir: &Path) -> std::io::Result<()> {
    match std::fs::symlink_metadata(dir) {
        Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(dir),
        Ok(_) => Err(std::io::Error::other(format!(
            "{} exists and is not a folder",
            dir.display()
        ))),
        Err(_) => Ok(()),
    }
}

/// `(name, size in bytes)` of the files in the sandbox; `holiday.png` is an image.
const FILES: &[(&str, usize)] = &[
    ("notes.txt", 1_240),
    ("todo.md", 386),
    ("report.pdf", 482_000),
    ("budget.csv", 18_300),
    ("archive.zip", 7_400_000),
    (".hidden", 120),
];

/// Folder names and how many files each holds.
const FOLDERS: &[(&str, usize)] = &[("Projects", 5), ("Photos", 12), ("Music", 3)];

fn create_in(dir: &Path) -> std::io::Result<()> {
    remove(dir)?;
    std::fs::create_dir_all(dir)?;
    let mut day = 0;
    let mut stamp = |path: &Path| {
        day += 1;
        let file = gio::File::for_path(path);
        let _ = file.set_attribute_uint64(
            "time::modified",
            BASE_TIME + day * 86_400 + day * 3_671,
            gio::FileQueryInfoFlags::NONE,
            gio::Cancellable::NONE,
        );
    };
    for (name, count) in FOLDERS {
        let folder = dir.join(name);
        std::fs::create_dir(&folder)?;
        for i in 1..=*count {
            std::fs::write(folder.join(format!("item-{i}.txt")), "dummy\n")?;
        }
        stamp(&folder);
    }
    for (name, size) in FILES {
        let path = dir.join(name);
        let text = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit.\n";
        let content: Vec<u8> = text.iter().copied().cycle().take(*size).collect();
        std::fs::write(&path, content)?;
        stamp(&path);
    }
    let image = dir.join("holiday.png");
    write_image(&image)?;
    stamp(&image);
    Ok(())
}

/// A small gradient, so the file gets a thumbnail (`VIEW-37`).
fn write_image(path: &Path) -> std::io::Result<()> {
    let (w, h) = (240usize, 160usize);
    let mut pixels = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            pixels.extend([
                (x * 255 / w) as u8,
                (y * 255 / h) as u8,
                200 - (x * 100 / w) as u8,
                255,
            ]);
        }
    }
    let texture = gdk::MemoryTexture::new(
        w as i32,
        h as i32,
        gdk::MemoryFormat::R8g8b8a8,
        &glib::Bytes::from_owned(pixels),
        w * 4,
    );
    texture
        .save_to_png(path)
        .map_err(|e| std::io::Error::other(format!("could not write holiday.png: {e}")))
}

/// The path of a place in the sidebar that exists only as a label (`NAV-46`).
pub fn place(name: &str) -> gio::File {
    gio::File::for_path(PathBuf::from(DIR).join(".places").join(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_has_the_dummy_items() {
        let dir = std::env::temp_dir().join(format!("junk-screenshot-test-{}", std::process::id()));
        create_in(&dir).unwrap();
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                ".hidden",
                "Music",
                "Photos",
                "Projects",
                "archive.zip",
                "budget.csv",
                "holiday.png",
                "notes.txt",
                "report.pdf",
                "todo.md"
            ]
        );
        assert_eq!(
            std::fs::metadata(dir.join("notes.txt")).unwrap().len(),
            1_240
        );
        assert_eq!(std::fs::read_dir(dir.join("Photos")).unwrap().count(), 12);
        assert!(std::fs::metadata(dir.join("holiday.png")).unwrap().len() > 100);
        // A second run starts afresh.
        std::fs::write(dir.join("extra"), "x").unwrap();
        create_in(&dir).unwrap();
        assert!(!dir.join("extra").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
