//! Remembered preferences in `$XDG_STATE_HOME/junk-drawer/state.ini` (`CONF-18`…`CONF-20`, `CONF-22`).
use crate::config;
use gtk::{gio, glib, prelude::*};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

const SORT_COLUMNS: [&str; 4] = ["name", "size", "modified", "type"];

/// `$XDG_STATE_HOME/junk-drawer`.
pub fn dir() -> PathBuf {
    glib::user_state_dir().join("junk-drawer")
}

#[derive(Clone, Default, PartialEq, Debug)]
pub struct Saved {
    pub view: Option<String>,
    pub sort_column: Option<String>,
    pub sort_reversed: Option<bool>,
    pub show_hidden: Option<bool>,
    pub show_sidebar: Option<bool>,
    pub show_status_bar: Option<bool>,
    pub icon_size: Option<i32>,
    pub list_icon_size: Option<i32>,
}

fn warn_once(message: &str) {
    static WARNED: AtomicBool = AtomicBool::new(false);
    if !WARNED.swap(true, Ordering::Relaxed) {
        glib::g_warning!("junk", "state.ini: {message}");
    }
}

impl Saved {
    /// Reads `path`. A missing file gives the defaults; a bad file or value is ignored.
    pub fn load(path: &Path) -> Self {
        let kf = glib::KeyFile::new();
        match kf.load_from_file(path, glib::KeyFileFlags::NONE) {
            Ok(()) => Self::from_keyfile(&kf),
            Err(e) => {
                if !e.matches(glib::FileError::Noent) {
                    warn_once(&format!("{}: {e}", path.display()));
                }
                Self::default()
            }
        }
    }

    fn from_keyfile(kf: &glib::KeyFile) -> Self {
        let mut s = Self::default();
        let mut bad = false;
        let mut text =
            |group: &str, key: &str, ok: &dyn Fn(&str) -> bool| match kf.string(group, key) {
                Ok(v) if ok(&v) => Some(v.to_string()),
                Ok(_) => {
                    bad = true;
                    None
                }
                Err(_) => None,
            };
        s.view = text("view", "mode", &|v| matches!(v, "icons" | "details"));
        s.sort_column = text("view", "sort-column", &|v| SORT_COLUMNS.contains(&v));
        let flag = |key: &str| kf.boolean("view", key).ok();
        s.sort_reversed = flag("sort-reversed");
        s.show_hidden = flag("show-hidden");
        s.show_sidebar = flag("show-sidebar");
        s.show_status_bar = flag("show-status-bar");
        s.icon_size = kf
            .integer("view", "icon-size")
            .ok()
            .map(|v| v.clamp(16, 512));
        s.list_icon_size = kf
            .integer("view", "list-icon-size")
            .ok()
            .map(|v| v.clamp(16, 128));
        if bad {
            warn_once("ignoring invalid values");
        }
        s
    }

    fn to_data(&self) -> String {
        let kf = glib::KeyFile::new();
        if let Some(v) = &self.view {
            kf.set_string("view", "mode", v);
        }
        if let Some(v) = &self.sort_column {
            kf.set_string("view", "sort-column", v);
        }
        if let Some(v) = self.sort_reversed {
            kf.set_boolean("view", "sort-reversed", v);
        }
        if let Some(v) = self.show_hidden {
            kf.set_boolean("view", "show-hidden", v);
        }
        if let Some(v) = self.show_sidebar {
            kf.set_boolean("view", "show-sidebar", v);
        }
        if let Some(v) = self.show_status_bar {
            kf.set_boolean("view", "show-status-bar", v);
        }
        if let Some(v) = self.icon_size {
            kf.set_integer("view", "icon-size", v);
        }
        if let Some(v) = self.list_icon_size {
            kf.set_integer("view", "list-icon-size", v);
        }
        kf.to_data().to_string()
    }

    /// The remembered value, else the `config.ini` one (`CONF-19`).
    pub fn view(&self) -> String {
        self.view
            .clone()
            .unwrap_or_else(|| config::get().view.clone())
    }
    pub fn show_hidden(&self) -> bool {
        self.show_hidden.unwrap_or(config::get().show_hidden)
    }
    pub fn show_sidebar(&self) -> bool {
        self.show_sidebar.unwrap_or(config::get().show_sidebar)
    }
    pub fn show_status_bar(&self) -> bool {
        self.show_status_bar
            .unwrap_or(config::get().show_status_bar)
    }
    pub fn icon_size(&self) -> i32 {
        self.icon_size.unwrap_or(config::get().icon_size)
    }
    pub fn list_icon_size(&self) -> i32 {
        self.list_icon_size.unwrap_or(config::get().list_icon_size)
    }
    /// Name ascending unless remembered otherwise (`VIEW-10`).
    pub fn sort(&self) -> (String, bool) {
        (
            self.sort_column.clone().unwrap_or_else(|| "name".into()),
            self.sort_reversed.unwrap_or(false),
        )
    }
}

#[derive(Default)]
struct Store {
    saved: Saved,
    loaded: bool,
    /// The state folder, or `None` with `[view] remember = false` (`CONF-22`).
    dir: Option<PathBuf>,
    writing: bool,
    dirty: bool,
}

thread_local! {
    static STORE: RefCell<Store> = RefCell::default();
}

/// Reads the remembered preferences from `dir`, or remembers nothing with `None`.
fn init(store: &mut Store, dir: Option<PathBuf>) {
    store.saved = dir
        .as_ref()
        .map(|d| Saved::load(&d.join("state.ini")))
        .unwrap_or_default();
    store.dir = dir;
    store.loaded = true;
}

/// The remembered preferences, read from disk on first use.
pub fn get() -> Saved {
    STORE.with(|s| {
        let mut s = s.borrow_mut();
        if !s.loaded {
            init(&mut s, config::get().remember.then(dir));
        }
        s.saved.clone()
    })
}

/// Changes the remembered preferences and saves them if they changed (`CONF-20`).
/// Does nothing when remembering is off, so the change stays in its window.
pub fn update(change: impl FnOnce(&mut Saved)) {
    let mut saved = get();
    if STORE.with(|s| s.borrow().dir.is_none()) {
        return;
    }
    let before = saved.clone();
    change(&mut saved);
    if saved == before {
        return;
    }
    STORE.with(|s| {
        let mut s = s.borrow_mut();
        s.saved = saved;
        s.dirty = true;
    });
    flush();
}

/// The data to write and where, if a save is due.
fn pending(s: &mut Store) -> Option<(PathBuf, String)> {
    let dir = s.dir.clone()?;
    s.dirty = false;
    Some((dir, s.saved.to_data()))
}

/// Writes the state file now, for a window that is closing: the app may quit before a
/// background write runs.
pub fn save_now() {
    let job = STORE.with(|s| {
        let mut s = s.borrow_mut();
        if !s.dirty && !s.writing {
            return None;
        }
        pending(&mut s)
    });
    let Some((dir, data)) = job else { return };
    let path = dir.join("state.ini");
    let tmp = dir.join("state.ini.tmp");
    let result = std::fs::create_dir_all(&dir)
        .and_then(|()| std::fs::write(&tmp, data))
        .and_then(|()| std::fs::rename(&tmp, &path));
    if let Err(e) = result {
        warn_once(&format!("can't save: {e}"));
    }
}

/// Writes the state file in the background, one write at a time, so the last change wins.
fn flush() {
    let job = STORE.with(|s| {
        let mut s = s.borrow_mut();
        if s.writing || !s.dirty {
            return None;
        }
        let job = pending(&mut s)?;
        s.writing = true;
        Some(job)
    });
    let Some((dir, data)) = job else { return };
    glib::spawn_future_local(async move {
        let result = match std::fs::create_dir_all(&dir) {
            Ok(()) => gio::File::for_path(dir.join("state.ini"))
                .replace_contents_future(data, None, false, gio::FileCreateFlags::NONE)
                .await
                .map(|_| ())
                .map_err(|(_, e)| e.to_string()),
            Err(e) => Err(e.to_string()),
        };
        if let Err(e) = result {
            warn_once(&format!("can't save: {e}"));
        }
        STORE.with(|s| s.borrow_mut().writing = false);
        flush();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("junk-state-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn remember_false_neither_reads_nor_writes() {
        let dir = temp("off");
        let file = dir.join("state.ini");
        std::fs::write(&file, "[view]\nmode=details\n").unwrap();
        STORE.with(|s| init(&mut s.borrow_mut(), Some(dir.clone())));
        assert_eq!(get().view.as_deref(), Some("details"));

        STORE.with(|s| init(&mut s.borrow_mut(), None));
        assert_eq!(get(), Saved::default());
        update(|s| s.show_hidden = Some(true));
        assert_eq!(get().show_hidden, None);
        save_now();
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "[view]\nmode=details\n"
        );
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn round_trips() {
        let dir = temp("round");
        let saved = Saved {
            view: Some("details".into()),
            sort_column: Some("size".into()),
            sort_reversed: Some(true),
            show_hidden: Some(true),
            show_sidebar: Some(false),
            show_status_bar: Some(true),
            icon_size: Some(168),
            list_icon_size: Some(64),
        };
        std::fs::write(dir.join("state.ini"), saved.to_data()).unwrap();
        assert_eq!(Saved::load(&dir.join("state.ini")), saved);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_or_garbage_file_gives_defaults() {
        let dir = temp("garbage");
        assert_eq!(Saved::load(&dir.join("state.ini")), Saved::default());
        std::fs::write(dir.join("state.ini"), "not an ini file\n[[").unwrap();
        assert_eq!(Saved::load(&dir.join("state.ini")), Saved::default());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn invalid_values_are_ignored() {
        let dir = temp("invalid");
        std::fs::write(
            dir.join("state.ini"),
            "[view]\nmode=grid\nsort-column=colour\nshow-hidden=true\nicon-size=9999\n",
        )
        .unwrap();
        let s = Saved::load(&dir.join("state.ini"));
        assert_eq!(s.view, None);
        assert_eq!(s.sort_column, None);
        assert_eq!(s.show_hidden, Some(true));
        assert_eq!(s.icon_size, Some(512));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn remembered_values_override_config() {
        let cfg = config::get();
        let none = Saved::default();
        assert_eq!(none.view(), cfg.view);
        assert_eq!(none.show_hidden(), cfg.show_hidden);
        assert_eq!(none.icon_size(), cfg.icon_size);
        assert_eq!(none.sort(), ("name".to_string(), false));
        let some = Saved {
            view: Some("details".into()),
            show_hidden: Some(!cfg.show_hidden),
            show_sidebar: Some(!cfg.show_sidebar),
            show_status_bar: Some(!cfg.show_status_bar),
            icon_size: Some(48),
            list_icon_size: Some(16),
            sort_column: Some("type".into()),
            sort_reversed: Some(true),
        };
        assert_eq!(some.view(), "details");
        assert_eq!(some.show_hidden(), !cfg.show_hidden);
        assert_eq!(some.show_sidebar(), !cfg.show_sidebar);
        assert_eq!(some.show_status_bar(), !cfg.show_status_bar);
        assert_eq!((some.icon_size(), some.list_icon_size()), (48, 16));
        assert_eq!(some.sort(), ("type".to_string(), true));
    }
}
