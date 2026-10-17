use gtk::glib;
use std::ffi::OsString;

/// What `--help` or `--version` prints (`INT-30`, `INT-31`). GLib's own help would
/// say "Usage: io.github.DisposableSoftware.JunkDrawer", because the prgname must stay the app ID (`NAV-4`).
pub fn early(args: impl IntoIterator<Item = OsString>) -> Option<String> {
    for arg in args.into_iter().skip(1) {
        match arg.to_str() {
            Some("--") => return None,
            Some("-h" | "--help") => return Some(help()),
            Some("-V" | "--version") => return Some(version()),
            _ => {}
        }
    }
    None
}

fn version() -> String {
    format!("junk {}\n", env!("CARGO_PKG_VERSION"))
}

fn help() -> String {
    let dir = glib::user_config_dir().join("junk-drawer");
    let config = dir.join("config.ini").display().to_string();
    let style = dir.join("style.css").display().to_string();
    let w = config.chars().count().max(style.chars().count());
    format!(
        "\
Usage: junk [OPTION…] [PATH|URI…]

A minimal file manager for Hyprland. Opens a window on each PATH or URI, or on
your home folder when there are none. If Junk Drawer is already running, the windows
open in it.

Options:
  -h, --help     Show this help and exit
  -V, --version  Show the version and exit

Files:
  {config:<w$}  settings and keybindings
  {style:<w$}  extra GTK CSS

See junk(1) for the keybindings, every setting and how thumbnails work.
",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str]) -> Option<String> {
        early(
            std::iter::once("junk")
                .chain(args.iter().copied())
                .map(OsString::from),
        )
    }

    #[test]
    fn help_and_version() {
        let help = run(&["--help"]).unwrap();
        assert!(help.starts_with("Usage: junk [OPTION…] [PATH|URI…]\n"));
        for part in ["--help", "--version", "config.ini", "style.css", "junk(1)"] {
            assert!(help.contains(part), "help lacks {part}");
        }
        assert_eq!(run(&["/tmp", "-h"]), Some(help));
        assert_eq!(
            run(&["-V"]).unwrap(),
            format!("junk {}\n", env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(run(&["--version"]), run(&["-V"]));
        assert_eq!(run(&[]), None);
        assert_eq!(run(&["/tmp", "sftp://host/"]), None);
        assert_eq!(run(&["--", "--help"]), None);
    }
}
