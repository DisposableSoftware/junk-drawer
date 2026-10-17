use gtk::glib;
use std::collections::HashMap;
use std::sync::OnceLock;

pub struct Config {
    pub flavor: String,
    pub accent: String,
    pub opacity: f64,
    /// `icons` or `details`.
    pub view: String,
    pub show_hidden: bool,
    pub show_path: bool,
    pub show_sidebar: bool,
    pub show_thumbnails: bool,
    /// `size`, `modified`, `type`, `permissions` or `none`.
    pub grid_caption: String,
    /// Icon size in pixels for the icon view (Nautilus default: 96).
    pub icon_size: i32,
    /// Icon size in pixels for the details view (Nautilus default: 32).
    pub list_icon_size: i32,
    /// Overrides from `[keybindings]`; see `keys::BINDINGS` for names and defaults.
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
            show_thumbnails: true,
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
    /// Reads `$XDG_CONFIG_HOME/junk-drawer/config.ini`; missing keys fall back to defaults.
    fn load() -> Self {
        let kf = glib::KeyFile::new();
        let path = glib::user_config_dir()
            .join("junk-drawer")
            .join("config.ini");
        match kf.load_from_file(&path, glib::KeyFileFlags::NONE) {
            Ok(_) => Self::from_keyfile(&kf),
            Err(e) => {
                if !e.matches(glib::FileError::Noent) {
                    glib::g_warning!("junk", "{}: {}", path.display(), e.message());
                }
                Self::default()
            }
        }
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
        if let Ok(v) = kf.boolean("view", "show-thumbnails") {
            cfg.show_thumbnails = v;
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
                cfg.keybindings.insert(key.to_string(), accels);
            }
        }
        cfg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_keybindings() {
        let kf = glib::KeyFile::new();
        kf.load_from_data(
            "[view]\nshow-hidden=true\nshow-path=false\nshow-sidebar=false\nshow-thumbnails=false\ngrid-caption=permissions\n[keybindings]\ngo-up=<Alt>Up;<Ctrl>u\ntrash=\n",
            glib::KeyFileFlags::NONE,
        )
        .unwrap();
        let cfg = Config::from_keyfile(&kf);
        assert!(cfg.show_hidden);
        assert!(!cfg.show_path);
        assert!(!cfg.show_sidebar);
        assert!(!cfg.show_thumbnails);
        assert_eq!(cfg.grid_caption, "permissions");
        assert_eq!(cfg.keybindings["go-up"], ["<Alt>Up", "<Ctrl>u"]);
        assert!(cfg.keybindings["trash"].is_empty());
    }

    #[test]
    fn ignores_unknown_grid_caption() {
        let kf = glib::KeyFile::new();
        kf.load_from_data("[view]\ngrid-caption=owner\n", glib::KeyFileFlags::NONE)
            .unwrap();
        assert_eq!(Config::from_keyfile(&kf).grid_caption, "size");
    }
}
