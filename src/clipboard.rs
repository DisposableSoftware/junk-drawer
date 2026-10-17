use crate::ops::{self, Transfer};
use gtk::{gdk, gio, glib, prelude::*};

/// Also understood by Nautilus, Thunar, Nemo and Caja, and carries the cut/copy flag.
const GNOME_MIME: &str = "x-special/gnome-copied-files";

pub fn set(widget: &impl IsA<gtk::Widget>, files: &[gio::File], cut: bool) {
    if files.is_empty() {
        return;
    }
    let uris: Vec<String> = files.iter().map(|f| f.uri().to_string()).collect();
    let text = format!("{}\n{}", if cut { "cut" } else { "copy" }, uris.join("\n"));
    let provider = gdk::ContentProvider::new_union(&[
        gdk::ContentProvider::for_bytes(GNOME_MIME, &glib::Bytes::from_owned(text.into_bytes())),
        gdk::ContentProvider::for_value(&gdk::FileList::from_array(files).to_value()),
    ]);
    let _ = widget.clipboard().set_content(Some(&provider));
}

pub fn paste(widget: &impl IsA<gtk::Widget>, dest: gio::File) {
    let clipboard = widget.clipboard();
    let parent = widget.root().and_downcast::<gtk::Window>();
    glib::spawn_future_local(async move {
        let Some((files, mode)) = read(&clipboard).await else {
            return;
        };
        if mode == Transfer::Move {
            let _ = clipboard.set_content(None::<&gdk::ContentProvider>);
        }
        ops::transfer(parent, files, dest, mode);
    });
}

async fn read(clipboard: &gdk::Clipboard) -> Option<(Vec<gio::File>, Transfer)> {
    if clipboard.formats().contain_mime_type(GNOME_MIME)
        && let Ok((stream, _)) = clipboard
            .read_future(&[GNOME_MIME], glib::Priority::DEFAULT)
            .await
    {
        let sink = gio::MemoryOutputStream::new_resizable();
        let flags =
            gio::OutputStreamSpliceFlags::CLOSE_SOURCE | gio::OutputStreamSpliceFlags::CLOSE_TARGET;
        if sink
            .splice_future(&stream, flags, glib::Priority::DEFAULT)
            .await
            .is_ok()
        {
            return parse_gnome(&String::from_utf8_lossy(&sink.steal_as_bytes()));
        }
    }
    let value = clipboard
        .read_value_future(gdk::FileList::static_type(), glib::Priority::DEFAULT)
        .await
        .ok()?;
    let files = value.get::<gdk::FileList>().ok()?.files();
    (!files.is_empty()).then_some((files, Transfer::Copy))
}

fn parse_gnome(text: &str) -> Option<(Vec<gio::File>, Transfer)> {
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
    let mode = match lines.next()? {
        "cut" => Transfer::Move,
        "copy" => Transfer::Copy,
        _ => return None,
    };
    let files: Vec<gio::File> = lines.map(gio::File::for_uri).collect();
    (!files.is_empty()).then_some((files, mode))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gnome_copied_files() {
        let (files, mode) = parse_gnome("cut\nfile:///tmp/a\nfile:///tmp/b%20c\n").unwrap();
        assert_eq!(mode, Transfer::Move);
        assert_eq!(files.len(), 2);
        assert_eq!(files[1].path().unwrap(), std::path::Path::new("/tmp/b c"));
        assert!(parse_gnome("copy\n").is_none());
        assert!(parse_gnome("bogus\nfile:///x").is_none());
    }
}
