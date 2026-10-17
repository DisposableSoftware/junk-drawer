//! Checks the machine-readable tables in `spec/` against the code and the example
//! config, so the spec can't silently drift. See spec/README.md.

use crate::config::Config;
use crate::keys::BINDINGS;
use gtk::glib;

const KEYBINDINGS_MD: &str = include_str!("../spec/keybindings.md");
const CONFIGURATION_MD: &str = include_str!("../spec/configuration.md");
const EXAMPLE_INI: &str = include_str!("../data/config.example.ini");
const KEYBINDINGS_INI: &str = include_str!("../data/keybindings.example.ini");

/// Body rows of the table between `<!-- spec:NAME:begin -->` and `...:end -->`.
fn table(doc: &str, name: &str) -> Vec<Vec<String>> {
    let begin = format!("<!-- spec:{name}:begin -->");
    let end = format!("<!-- spec:{name}:end -->");
    let start = doc
        .find(&begin)
        .unwrap_or_else(|| panic!("missing {begin}"))
        + begin.len();
    let stop = doc[start..]
        .find(&end)
        .unwrap_or_else(|| panic!("missing {end}"))
        + start;
    doc[start..stop]
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('|'))
        .skip(2)
        .map(|l| {
            l.trim_matches('|')
                .split('|')
                .map(|c| c.trim().to_owned())
                .collect()
        })
        .collect()
}

/// "`a`, `b`" -> ["a", "b"]
fn code_list(cell: &str) -> Vec<String> {
    cell.split(',')
        .map(|s| s.trim().trim_matches('`').to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

fn default_value(cfg: &Config, section: &str, key: &str) -> Option<String> {
    Some(match (section, key) {
        ("appearance", "flavor") => cfg.flavor.clone(),
        ("appearance", "accent") => cfg.accent.clone(),
        ("appearance", "opacity") => cfg.opacity.to_string(),
        ("view", "default") => cfg.view.clone(),
        ("view", "show-hidden") => cfg.show_hidden.to_string(),
        ("view", "show-path") => cfg.show_path.to_string(),
        ("view", "show-sidebar") => cfg.show_sidebar.to_string(),
        ("view", "show-thumbnails") => cfg.show_thumbnails.to_string(),
        ("view", "remember") => cfg.remember.to_string(),
        ("view", "grid-caption") => cfg.grid_caption.clone(),
        ("view", "icon-size") => cfg.icon_size.to_string(),
        ("view", "list-icon-size") => cfg.list_icon_size.to_string(),
        _ => return None,
    })
}
const CONFIG_KEY_COUNT: usize = 12;

fn config_rows() -> Vec<(String, String, String)> {
    table(CONFIGURATION_MD, "config")
        .into_iter()
        .map(|r| {
            let c = |i: usize| r[i].trim_matches('`').to_owned();
            (c(0), c(1), c(2))
        })
        .collect()
}

#[test]
fn keybindings_match_spec() {
    let spec = table(KEYBINDINGS_MD, "keybindings");
    let spec: Vec<_> = spec
        .iter()
        .map(|r| {
            (
                r[0].trim_matches('`'),
                r[1].trim_matches('`'),
                code_list(&r[2]),
            )
        })
        .collect();
    let code: Vec<_> = BINDINGS
        .iter()
        .map(|(n, a, _, k)| (*n, *a, k.iter().map(|s| s.to_string()).collect::<Vec<_>>()))
        .collect();
    assert_eq!(spec, code, "spec/keybindings.md and keys::BINDINGS differ");
}

#[test]
fn config_defaults_match_spec() {
    let cfg = Config::default();
    let rows = config_rows();
    assert_eq!(
        rows.len(),
        CONFIG_KEY_COUNT,
        "spec/configuration.md must list every config key"
    );
    for (section, key, default) in rows {
        let code = default_value(&cfg, &section, &key)
            .unwrap_or_else(|| panic!("spec lists unknown key [{section}] {key}"));
        assert_eq!(
            code, default,
            "[{section}] {key}: Config::default() vs spec"
        );
    }
}

#[test]
fn example_config_matches_spec() {
    let kf = glib::KeyFile::new();
    kf.load_from_data(EXAMPLE_INI, glib::KeyFileFlags::NONE)
        .unwrap();
    for (section, key, default) in config_rows() {
        let value = kf
            .value(&section, &key)
            .unwrap_or_else(|_| panic!("config.example.ini lacks [{section}] {key}"));
        assert_eq!(
            value.as_str(),
            default,
            "[{section}] {key}: example vs spec"
        );
    }
    assert!(
        !kf.has_group("keybindings"),
        "bindings belong in keybindings.example.ini (KEY-2)"
    );
    let kf = glib::KeyFile::new();
    kf.load_from_data(KEYBINDINGS_INI, glib::KeyFileFlags::NONE)
        .unwrap();
    let keys = kf.keys("keybindings").unwrap();
    assert_eq!(
        keys.len(),
        BINDINGS.len(),
        "keybindings.example.ini must list every binding"
    );
    for (name, _, _, defaults) in BINDINGS {
        let value = kf
            .value("keybindings", name)
            .unwrap_or_else(|_| panic!("keybindings.example.ini lacks {name}"));
        assert_eq!(
            value.as_str(),
            defaults.join(";"),
            "[keybindings] {name}: example vs code"
        );
    }
}

const MAN_PAGE: &str = include_str!("../data/junk.1");

/// The man page's `.TP` entries tagged `.B name`, as (section, subsection, name,
/// body), with the roff escapes the page uses turned back into text.
fn man_entries() -> Vec<(String, String, String, String)> {
    let plain = |s: &str| {
        s.replace("\\-", "-")
            .replace("\\fB", "")
            .replace("\\fR", "")
    };
    let (mut sh, mut ss) = (String::new(), String::new());
    let mut entries: Vec<(String, String, String, String)> = Vec::new();
    let mut lines = MAN_PAGE.lines().peekable();
    while let Some(line) = lines.next() {
        if let Some(t) = line.strip_prefix(".SH ") {
            (sh, ss) = (t.trim_matches('"').to_owned(), String::new());
        } else if let Some(t) = line.strip_prefix(".SS ") {
            ss = t.trim_matches('"').to_owned();
        } else if line == ".TP" {
            if let Some(name) = lines.peek().and_then(|l| l.strip_prefix(".B ")) {
                entries.push((sh.clone(), ss.clone(), plain(name), String::new()));
                lines.next();
            }
        } else if line.starts_with(".SH") || line == ".PP" {
            // An untagged paragraph ends the entry.
        } else if let Some(entry) = entries.last_mut()
            && entry.0 == sh
            && entry.1 == ss
        {
            entry.3.push_str(&plain(line));
            entry.3.push('\n');
        }
    }
    entries
}

#[test]
fn man_page_matches_spec() {
    assert!(
        MAN_PAGE
            .lines()
            .next()
            .unwrap()
            .contains(&format!("\"junk {}\"", env!("CARGO_PKG_VERSION"))),
        "junk.1: .TH must carry the Cargo.toml version"
    );
    let entries = man_entries();
    let bindings: Vec<_> = entries.iter().filter(|e| e.0 == "KEY BINDINGS").collect();
    let names: Vec<_> = bindings.iter().map(|e| e.2.as_str()).collect();
    let code: Vec<_> = BINDINGS.iter().map(|b| b.0).collect();
    assert_eq!(names, code, "junk.1 KEY BINDINGS vs keys::BINDINGS");
    for ((name, _, description, keys), entry) in BINDINGS.iter().zip(&bindings) {
        let body = &entry.3;
        assert!(
            body.starts_with(&format!("{description}.\n")),
            "junk.1 {name}: should start with “{description}.”"
        );
        let keys = if keys.is_empty() {
            "none".to_owned()
        } else {
            keys.join(", ")
        };
        assert!(
            body.lines().any(|l| l == format!("Keys: {keys}")),
            "junk.1 {name}: lacks “Keys: {keys}”"
        );
    }
    let settings: Vec<_> = entries.iter().filter(|e| e.0 == "CONFIGURATION").collect();
    assert_eq!(
        settings.len(),
        CONFIG_KEY_COUNT,
        "junk.1 CONFIGURATION entries"
    );
    for (section, key, default) in config_rows() {
        let entry = settings
            .iter()
            .find(|e| e.1 == format!("[{section}]") && e.2 == key)
            .unwrap_or_else(|| panic!("junk.1 lacks [{section}] {key}"));
        assert!(
            entry.3.lines().any(|l| l == format!("Default: {default}")),
            "junk.1 [{section}] {key}: lacks “Default: {default}”"
        );
    }
}
