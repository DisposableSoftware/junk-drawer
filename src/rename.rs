use crate::dialog;
use gtk::{gio, glib, prelude::*};

/// Small modal entry. Pre-selects the name without its extension, like Nautilus.
pub fn prompt(parent: &gtk::ApplicationWindow, file: gio::File) {
    let Some(name) = file.basename().map(|n| n.to_string_lossy().into_owned()) else {
        return;
    };
    let window = parent.clone();
    dialog::ask_name(
        parent,
        "Rename",
        &name.clone(),
        stem_len(&name),
        move |new| {
            if new == name {
                return;
            }
            let (file, window) = (file.clone(), window.clone());
            glib::spawn_future_local(async move {
                if let Err(e) = file
                    .set_display_name_future(&new, glib::Priority::DEFAULT)
                    .await
                {
                    dialog::error(
                        Some(&window),
                        &format!("Could not rename to “{new}”"),
                        e.message(),
                    );
                }
            });
        },
    );
}

/// Length in chars of the part before the extension; the whole name for dotfiles
/// and names without an extension.
fn stem_len(name: &str) -> i32 {
    let stem = match name.rfind('.') {
        Some(i) if i > 0 => &name[..i],
        _ => name,
    };
    stem.chars().count() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stem_selection() {
        assert_eq!(stem_len("photo.jpg"), 5);
        assert_eq!(stem_len("archive.tar.gz"), 11);
        assert_eq!(stem_len(".bashrc"), 7);
        assert_eq!(stem_len("README"), 6);
        assert_eq!(stem_len("café.txt"), 4);
    }

    #[test]
    fn rename_refuses_to_overwrite() {
        let dir = std::env::temp_dir().join(format!("junk-test-rename-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a"), "a").unwrap();
        std::fs::write(dir.join("b"), "b").unwrap();
        let a = gio::File::for_path(dir.join("a"));
        let res = glib::MainContext::new()
            .block_on(a.set_display_name_future("b", glib::Priority::DEFAULT));
        assert!(res.is_err());
        assert_eq!(std::fs::read_to_string(dir.join("b")).unwrap(), "b");
    }
}
