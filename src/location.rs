use gtk::{gdk, gio, glib, prelude::*};
use std::rc::Rc;

/// The file that `text` names, typed in a window showing `cwd` (`NAV-16`).
fn resolve(text: &str, cwd: &gio::File) -> gio::File {
    let text = text.trim();
    let home = glib::home_dir();
    if text == "~" {
        return gio::File::for_path(home);
    }
    if let Some(rest) = text.strip_prefix("~/") {
        return gio::File::for_path(home.join(rest));
    }
    if text.starts_with('/') || glib::Uri::peek_scheme(text).is_some() {
        return gio::File::for_commandline_arg(text);
    }
    cwd.resolve_relative_path(text)
}

/// The longest prefix that every name starting with `typed` shares, and whether only
/// one name matched. Dot names match only a typed dot.
fn complete<'a>(typed: &str, names: impl IntoIterator<Item = &'a str>) -> Option<(String, bool)> {
    let mut matches = names
        .into_iter()
        .filter(|n| n.starts_with(typed) && (typed.starts_with('.') || !n.starts_with('.')));
    let first = matches.next()?.to_owned();
    let mut common = first.clone();
    let mut unique = true;
    for name in matches {
        unique = false;
        let shared = common
            .char_indices()
            .zip(name.chars())
            .find(|((_, a), b)| a != b)
            .map(|((i, _), _)| i)
            .unwrap_or(common.len().min(name.len()));
        common.truncate(shared);
    }
    Some((common, unique))
}

/// Tab completion of the entry's last path segment.
async fn tab(entry: &gtk::Entry, cwd: &gio::File) {
    let text = entry.text().to_string();
    let (dir_text, typed) = match text.rfind('/') {
        Some(i) => (&text[..=i], &text[i + 1..]),
        None => ("", text.as_str()),
    };
    let dir = if dir_text.is_empty() {
        cwd.clone()
    } else {
        resolve(dir_text, cwd)
    };
    let Ok(children) = dir
        .enumerate_children_future(
            "standard::name,standard::type",
            gio::FileQueryInfoFlags::NONE,
            glib::Priority::DEFAULT,
        )
        .await
    else {
        return;
    };
    let mut infos = Vec::new();
    while let Ok(batch) = children
        .next_files_future(256, glib::Priority::DEFAULT)
        .await
    {
        if batch.is_empty() {
            break;
        }
        infos.extend(batch);
    }
    let names: Vec<String> = infos
        .iter()
        .map(|i| i.name().to_string_lossy().into_owned())
        .collect();
    let Some((common, unique)) = complete(typed, names.iter().map(String::as_str)) else {
        return;
    };
    let is_dir = unique
        && infos.iter().any(|i| {
            i.name().to_string_lossy() == common && i.file_type() == gio::FileType::Directory
        });
    let completed = format!("{dir_text}{common}{}", if is_dir { "/" } else { "" });
    // The entry may have changed while the folder was read.
    if entry.text() == text {
        entry.set_text(&completed);
        entry.set_position(-1);
    }
}

/// Adds the `location` action: it swaps `path` in `bar` for an entry. `open` gets the
/// folder to show and the file to select in it, if any.
pub fn attach(
    window: &gtk::ApplicationWindow,
    bar: &gtk::Box,
    path: &gtk::Label,
    open: impl Fn(gio::File, Option<gio::File>) + 'static,
    focus_view: impl Fn() + 'static,
) {
    let entry = gtk::Entry::builder()
        .hexpand(true)
        .visible(false)
        .css_classes(["location"])
        .build();
    bar.append(&entry);
    let bar_shown = bar.get_visible();
    let current = |window: &gtk::ApplicationWindow| {
        window
            .title()
            .map(|t| gio::File::for_parse_name(&t))
            .unwrap_or_else(|| gio::File::for_path("/"))
    };

    let close: Rc<dyn Fn()> = Rc::new(glib::clone!(
        #[weak]
        bar,
        #[weak]
        path,
        #[weak]
        entry,
        move || {
            if !entry.get_visible() {
                return;
            }
            entry.set_visible(false);
            path.set_visible(true);
            bar.set_visible(bar_shown);
            focus_view();
        }
    ));

    let action = gio::SimpleAction::new("location", None);
    action.connect_activate(glib::clone!(
        #[weak]
        window,
        #[weak]
        bar,
        #[weak]
        path,
        #[weak]
        entry,
        move |_, _| {
            entry.set_text(&current(&window).parse_name());
            path.set_visible(false);
            entry.set_visible(true);
            bar.set_visible(true);
            entry.grab_focus();
            entry.select_region(0, -1);
        }
    ));
    window.add_action(&action);

    let open = Rc::new(open);
    entry.connect_activate(glib::clone!(
        #[weak]
        window,
        #[strong]
        close,
        move |entry| {
            let target = resolve(&entry.text(), &current(&window));
            let (open, close, entry) = (open.clone(), close.clone(), entry.clone());
            glib::spawn_future_local(async move {
                let info = target
                    .query_info_future(
                        "standard::type",
                        gio::FileQueryInfoFlags::NONE,
                        glib::Priority::DEFAULT,
                    )
                    .await;
                match info {
                    Ok(info) if info.file_type() == gio::FileType::Directory => {
                        close();
                        open(target, None);
                    }
                    Ok(_) => match target.parent() {
                        Some(dir) => {
                            close();
                            open(dir, Some(target));
                        }
                        None => close(),
                    },
                    Err(e) => {
                        // The dialog takes focus; keep the entry open behind it.
                        entry.set_sensitive(false);
                        let _ = gtk::AlertDialog::builder()
                            .message(format!("Could not open “{}”", entry.text()))
                            .detail(e.message())
                            .buttons(["Close"])
                            .build()
                            .choose_future(Some(&window))
                            .await;
                        entry.set_sensitive(true);
                        entry.grab_focus();
                        entry.set_position(-1);
                    }
                }
            });
        }
    ));

    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(glib::clone!(
        #[weak]
        window,
        #[weak]
        entry,
        #[strong]
        close,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, key, _, mods| match key {
            gdk::Key::Escape => {
                close();
                glib::Propagation::Stop
            }
            gdk::Key::Tab if mods.is_empty() => {
                let cwd = current(&window);
                glib::spawn_future_local(async move { tab(&entry, &cwd).await });
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    ));
    entry.add_controller(keys);

    let focus = gtk::EventControllerFocus::new();
    focus.connect_leave(glib::clone!(
        #[weak]
        entry,
        #[strong]
        close,
        move |_| {
            // Insensitive while the error dialog is up; that leave isn't "elsewhere".
            if entry.is_sensitive() {
                close();
            }
        }
    ));
    entry.add_controller(focus);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_typed_locations() {
        let cwd = gio::File::for_path("/srv/data");
        let path = |t: &str| resolve(t, &cwd).path().unwrap();
        let home = glib::home_dir();
        assert_eq!(path("~"), home);
        assert_eq!(path("~/Music"), home.join("Music"));
        assert_eq!(path("/etc"), std::path::Path::new("/etc"));
        assert_eq!(path("sub/x"), std::path::Path::new("/srv/data/sub/x"));
        assert_eq!(path("../other"), std::path::Path::new("/srv/other"));
        assert_eq!(path(" /tmp "), std::path::Path::new("/tmp"));
        assert_eq!(resolve("sftp://host/srv", &cwd).uri(), "sftp://host/srv");
        assert_eq!(path("file:///opt"), std::path::Path::new("/opt"));
    }

    #[test]
    fn completes_shared_prefixes() {
        let names = ["Documents", "Downloads", "Desktop", ".config", "music"];
        assert_eq!(complete("Do", names), Some(("Do".into(), false)));
        assert_eq!(complete("Doc", names), Some(("Documents".into(), true)));
        assert_eq!(complete("D", names), Some(("D".into(), false)));
        assert_eq!(complete("", names), Some(("".into(), false)));
        assert_eq!(complete(".c", names), Some((".config".into(), true)));
        assert_eq!(complete("x", names), None);
        assert_eq!(
            complete("ab", ["abcd", "abce"]),
            Some(("abc".into(), false))
        );
    }
}
