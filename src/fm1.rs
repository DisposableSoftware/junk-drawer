use crate::{properties, window};
use gtk::{gio, glib, prelude::*};

const NAME: &str = "org.freedesktop.FileManager1";
const PATH: &str = "/org/freedesktop/FileManager1";
const XML: &str = r#"<node>
  <interface name="org.freedesktop.FileManager1">
    <method name="ShowFolders">
      <arg type="as" name="URIs" direction="in"/>
      <arg type="s" name="StartupId" direction="in"/>
    </method>
    <method name="ShowItems">
      <arg type="as" name="URIs" direction="in"/>
      <arg type="s" name="StartupId" direction="in"/>
    </method>
    <method name="ShowItemProperties">
      <arg type="as" name="URIs" direction="in"/>
      <arg type="s" name="StartupId" direction="in"/>
    </method>
  </interface>
</node>"#;

/// Each item's folder, in the order first seen, with its items (`NAV-5`).
fn by_folder(files: &[gio::File]) -> Vec<(gio::File, Vec<gio::File>)> {
    let mut groups: Vec<(gio::File, Vec<gio::File>)> = Vec::new();
    for file in files {
        let Some(dir) = file.parent() else {
            continue;
        };
        match groups.iter_mut().find(|(d, _)| d.equal(&dir)) {
            Some((_, items)) => items.push(file.clone()),
            None => groups.push((dir, vec![file.clone()])),
        }
    }
    groups
}

fn call(app: &gtk::Application, method: &str, files: &[gio::File], startup_id: &str) {
    let id = Some(startup_id);
    match method {
        "ShowFolders" => {
            for dir in files {
                window::open_selecting(app, dir, &[], id);
            }
        }
        "ShowItems" => {
            for (dir, items) in by_folder(files) {
                window::open_selecting(app, &dir, &items, id);
            }
        }
        "ShowItemProperties" => {
            if let Some((dir, items)) = by_folder(files).into_iter().next() {
                let parent = window::open_selecting(app, &dir, &items, id);
                properties::show(&parent, files.to_vec());
            }
        }
        _ => {}
    }
}

/// Exports `org.freedesktop.FileManager1` on the app's session bus connection and asks
/// for the name, queueing behind a current owner (`INT-11`).
pub fn register(app: &gtk::Application) {
    let Some(connection) = app.dbus_connection() else {
        return;
    };
    let Some(interface) = gio::DBusNodeInfo::for_xml(XML)
        .ok()
        .and_then(|node| node.lookup_interface(NAME))
    else {
        return;
    };
    let app = app.downgrade();
    let registered = connection
        .register_object(PATH, &interface)
        .method_call(move |_, _, _, _, method, params, invocation| {
            let Some(app) = app.upgrade() else {
                return;
            };
            let Some((uris, startup_id)) = params.get::<(Vec<String>, String)>() else {
                invocation.return_error(gio::DBusError::InvalidArgs, "expected (as, s)");
                return;
            };
            // Callers send URIs; a few send plain paths.
            let files: Vec<gio::File> = uris.iter().map(gio::File::for_commandline_arg).collect();
            call(&app, method, &files, &startup_id);
            invocation.return_value(None);
        })
        .build();
    if let Err(e) = registered {
        glib::g_warning!("junk", "FileManager1: {}", e.message());
        return;
    }
    // A plain RequestName, not g_bus_own_name: gio-rs panics in its name-lost callback
    // when the bus goes away (the connection is NULL then). The bus drops the name with
    // the connection anyway. Flags 0 queue behind a current owner.
    connection.call(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
        "RequestName",
        Some(&(NAME, 0u32).to_variant()),
        None,
        gio::DBusCallFlags::NONE,
        -1,
        gio::Cancellable::NONE,
        |_| {},
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_items_by_folder() {
        let f = gio::File::for_path;
        let groups = by_folder(&[f("/a/x"), f("/b/y"), f("/a/z"), f("/")]);
        let paths: Vec<(String, Vec<String>)> = groups
            .iter()
            .map(|(d, items)| {
                let p = |f: &gio::File| f.path().unwrap().to_string_lossy().into_owned();
                (p(d), items.iter().map(p).collect())
            })
            .collect();
        assert_eq!(
            paths,
            [
                ("/a".to_owned(), vec!["/a/x".to_owned(), "/a/z".to_owned()]),
                ("/b".to_owned(), vec!["/b/y".to_owned()]),
            ]
        );
    }
}
