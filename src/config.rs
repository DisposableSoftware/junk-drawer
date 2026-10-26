use gtk::glib;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const CONFIG_EXAMPLE: &str = include_str!("../data/config.example.ini");
const KEYBINDINGS_EXAMPLE: &str = include_str!("../data/keybindings.example.ini");

/// `$XDG_CONFIG_HOME/junk-drawer`.
pub fn dir() -> PathBuf {
    glib::user_config_dir().join("junk-drawer")
}

pub struct Config {
    pub flavor: String,
    pub accent: String,
    pub opacity: f64,
    /// `icons` or `details`.
    pub view: String,
    pub show_hidden: bool,
    pub show_path: bool,
    pub show_sidebar: bool,
    pub show_status_bar: bool,
    pub show_thumbnails: bool,
    /// `false` turns off `state.ini` (`CONF-22`).
    pub remember: bool,
    /// `size`, `modified`, `type`, `permissions` or `none`.
    pub grid_caption: String,
    /// Icon size in pixels for the icon view (Nautilus default: 96).
    pub icon_size: i32,
    /// Icon size in pixels for the details view (Nautilus default: 32).
    pub list_icon_size: i32,
    /// Overrides from `keybindings.ini`; see `keys::BINDINGS` for names and defaults.
    pub keybindings: HashMap<String, Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            flavor: "mocha".into(),
            accent: "mauve".into(),
            opacity: 0.85,
            view: "icons".into(),
            show_hidden: false,
            show_path: true,
            show_sidebar: true,
            show_status_bar: false,
            show_thumbnails: true,
            remember: true,
            grid_caption: "size".into(),
            icon_size: 96,
            list_icon_size: 32,
            keybindings: HashMap::new(),
        }
    }
}

pub fn get() -> &'static Config {
    static CONFIG: OnceLock<Config> = OnceLock::new();
    CONFIG.get_or_init(Config::load)
}

impl Config {
    /// Reads `config.ini` and `keybindings.ini`, first creating whichever is missing
    /// (`CONF-17`); missing keys fall back to defaults.
    fn load() -> Self {
        Self::load_from(&dir())
    }

    fn load_from(dir: &Path) -> Self {
        create_missing(dir);
        let mut cfg = match read(&dir.join("config.ini")) {
            Some(kf) => {
                if kf.has_group("keybindings") {
                    glib::g_warning!(
                        "junk",
                        "config.ini: [keybindings] is ignored; bindings now live in keybindings.ini"
                    );
                }
                Self::from_keyfile(&kf)
            }
            None => Self::default(),
        };
        if let Some(kf) = read(&dir.join("keybindings.ini")) {
            cfg.keybindings = bindings(&kf);
        }
        cfg
    }

    fn from_keyfile(kf: &glib::KeyFile) -> Self {
        let mut cfg = Self::default();
        if let Ok(v) = kf.string("appearance", "flavor") {
            cfg.flavor = v.to_string();
        }
        if let Ok(v) = kf.string("appearance", "accent") {
            cfg.accent = v.to_string();
        }
        if let Ok(v) = kf.double("appearance", "opacity") {
            cfg.opacity = v.clamp(0.0, 1.0);
        }
        if let Ok(v) = kf.string("view", "default")
            && matches!(v.as_str(), "icons" | "details")
        {
            cfg.view = v.to_string();
        }
        if let Ok(v) = kf.boolean("view", "show-hidden") {
            cfg.show_hidden = v;
        }
        if let Ok(v) = kf.boolean("view", "show-path") {
            cfg.show_path = v;
        }
        if let Ok(v) = kf.boolean("view", "show-sidebar") {
            cfg.show_sidebar = v;
        }
        if let Ok(v) = kf.boolean("view", "show-status-bar") {
            cfg.show_status_bar = v;
        }
        if let Ok(v) = kf.boolean("view", "show-thumbnails") {
            cfg.show_thumbnails = v;
        }
        if let Ok(v) = kf.boolean("view", "remember") {
            cfg.remember = v;
        }
        if let Ok(v) = kf.string("view", "grid-caption")
            && matches!(
                v.as_str(),
                "size" | "modified" | "type" | "permissions" | "none"
            )
        {
            cfg.grid_caption = v.to_string();
        }
        if let Ok(v) = kf.integer("view", "icon-size") {
            cfg.icon_size = v.clamp(16, 512);
        }
        if let Ok(v) = kf.integer("view", "list-icon-size") {
            cfg.list_icon_size = v.clamp(16, 128);
        }
        cfg
    }
}

/// The `[keybindings]` group of `keybindings.ini` (`KEY-2`).
fn bindings(kf: &glib::KeyFile) -> HashMap<String, Vec<String>> {
    let mut out = HashMap::new();
    for key in kf
        .keys("keybindings")
        .map(|k| k.to_vec())
        .unwrap_or_default()
    {
        if let Ok(list) = kf.string_list("keybindings", &key) {
            let accels = list
                .iter()
                .map(|a| a.trim().to_owned())
                .filter(|a| !a.is_empty())
                .collect();
            out.insert(key.to_string(), accels);
        }
    }
    out
}

/// The key file at `path`, or `None` when it is missing or broken; broken is warned about.
fn read(path: &Path) -> Option<glib::KeyFile> {
    let kf = glib::KeyFile::new();
    match kf.load_from_file(path, glib::KeyFileFlags::NONE) {
        Ok(_) => Some(kf),
        Err(e) => {
            if !e.matches(glib::FileError::Noent) {
                glib::g_warning!("junk", "{}: {}", path.display(), e.message());
            }
            None
        }
    }
}

/// `example` with every setting commented out, so the file documents the defaults
/// without pinning them.
fn commented(example: &str) -> String {
    let mut out = String::from(
        "# Created by Junk Drawer with every default commented out. To change one,\n\
         # remove the leading '# ' and edit the value.\n",
    );
    for line in example.lines() {
        if line.starts_with(|c: char| c.is_ascii_lowercase()) && line.contains(" =") {
            out.push_str("# ");
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Writes the commented examples for files missing from `dir` (`CONF-17`). Anything
/// already at the path, a dangling symlink included, is left alone.
fn create_missing(dir: &Path) {
    for (name, example) in [
        ("config.ini", CONFIG_EXAMPLE),
        ("keybindings.ini", KEYBINDINGS_EXAMPLE),
    ] {
        let path = dir.join(name);
        if path.symlink_metadata().is_ok() {
            continue;
        }
        let written = std::fs::create_dir_all(dir).and_then(|()| {
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?
                .write_all(commented(example).as_bytes())
        });
        if let Err(e) = written {
            glib::g_warning!("junk", "could not create {}: {e}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("junk-test-config-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn reads_settings_and_keybindings_from_their_files() {
        let dir = temp("read");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("config.ini"),
            "[view]\nshow-hidden=true\nshow-path=false\nshow-sidebar=false\nshow-status-bar=true\nshow-thumbnails=false\nremember=false\ngrid-caption=permissions\n[keybindings]\ncopy=<Ctrl>k\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("keybindings.ini"),
            "[keybindings]\ngo-up=<Alt>Up;<Ctrl>u\ntrash=\n",
        )
        .unwrap();
        let cfg = Config::load_from(&dir);
        assert!(cfg.show_hidden);
        assert!(!cfg.show_path);
        assert!(!cfg.show_sidebar);
        assert!(cfg.show_status_bar);
        assert!(!cfg.show_thumbnails);
        assert!(!cfg.remember);
        assert_eq!(cfg.grid_caption, "permissions");
        assert_eq!(cfg.keybindings["go-up"], ["<Alt>Up", "<Ctrl>u"]);
        assert!(cfg.keybindings["trash"].is_empty());
        // KEY-2: a [keybindings] section in config.ini is ignored.
        assert!(!cfg.keybindings.contains_key("copy"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn creates_missing_files_with_the_defaults_commented_out() {
        let dir = temp("create").join("junk-drawer");
        let cfg = Config::load_from(&dir);
        let default = Config::default();
        assert_eq!(cfg.flavor, default.flavor);
        assert_eq!(cfg.opacity, default.opacity);
        assert_eq!(cfg.icon_size, default.icon_size);
        assert!(cfg.keybindings.is_empty());

        let config = std::fs::read_to_string(dir.join("config.ini")).unwrap();
        let keys = std::fs::read_to_string(dir.join("keybindings.ini")).unwrap();
        assert!(config.contains("\n# flavor = mocha\n"));
        assert!(keys.contains("\n# go-up = <Alt>Up;BackSpace\n"));
        for line in config.lines().chain(keys.lines()) {
            assert!(
                line.is_empty() || line.starts_with('#') || line.starts_with('['),
                "not commented out: {line}"
            );
        }

        // Uncommenting a line takes effect, and a second start leaves the files alone.
        std::fs::write(
            dir.join("config.ini"),
            config.replace("# flavor = mocha", "flavor = latte"),
        )
        .unwrap();
        std::fs::write(
            dir.join("keybindings.ini"),
            keys.replace("# go-up = <Alt>Up;BackSpace", "go-up = <Ctrl>u"),
        )
        .unwrap();
        let cfg = Config::load_from(&dir);
        assert_eq!(cfg.flavor, "latte");
        assert_eq!(cfg.keybindings["go-up"], ["<Ctrl>u"]);
        std::fs::remove_dir_all(dir.parent().unwrap()).unwrap();
    }

    #[test]
    fn never_writes_to_existing_files_or_symlinks() {
        let dir = temp("keep");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("config.ini"), "[view]\ndefault=details\n").unwrap();
        // Like a dotfiles link whose target isn't there yet.
        std::os::unix::fs::symlink(dir.join("elsewhere.ini"), dir.join("keybindings.ini")).unwrap();
        let cfg = Config::load_from(&dir);
        assert_eq!(cfg.view, "details");
        assert_eq!(
            std::fs::read_to_string(dir.join("config.ini")).unwrap(),
            "[view]\ndefault=details\n"
        );
        assert!(!dir.join("elsewhere.ini").exists());
        assert!(dir.join("keybindings.ini").symlink_metadata().is_ok());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn starts_with_defaults_when_the_files_cant_be_created() {
        let blocker = temp("blocked");
        std::fs::write(&blocker, "a file, not a folder").unwrap();
        let cfg = Config::load_from(&blocker.join("junk-drawer"));
        assert_eq!(cfg.flavor, Config::default().flavor);
        std::fs::remove_file(&blocker).unwrap();
    }

    #[test]
    fn ignores_unknown_grid_caption() {
        let kf = glib::KeyFile::new();
        kf.load_from_data("[view]\ngrid-caption=owner\n", glib::KeyFileFlags::NONE)
            .unwrap();
        assert_eq!(Config::from_keyfile(&kf).grid_caption, "size");
    }
}
