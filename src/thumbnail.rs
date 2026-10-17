use gtk::{gdk, gdk_pixbuf, gio, glib, prelude::*};
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Thumbnails being made at once (`VIEW-37`).
const WORKERS: usize = 2;
/// Bigger files are left to external thumbnailers rather than decoded in-process.
const MAX_DECODE: u64 = 50 * 1000 * 1000;
const PRIO: glib::Priority = glib::Priority::LOW;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Size {
    Normal,
    Large,
}

impl Size {
    fn for_pixels(pixels: i32) -> Self {
        if pixels > 128 {
            Size::Large
        } else {
            Size::Normal
        }
    }
    fn dir(self) -> &'static str {
        match self {
            Size::Normal => "normal",
            Size::Large => "large",
        }
    }
    fn pixels(self) -> i32 {
        match self {
            Size::Normal => 128,
            Size::Large => 256,
        }
    }
}

fn root() -> PathBuf {
    glib::user_cache_dir().join("thumbnails")
}

fn md5(uri: &str) -> String {
    glib::compute_checksum_for_string(glib::ChecksumType::Md5, uri)
        .map(|s| s.to_string())
        .unwrap_or_default()
}

/// Where the standard keeps the thumbnail of `uri` at `size`.
fn path_for(uri: &str, size: Size) -> PathBuf {
    root().join(size.dir()).join(format!("{}.png", md5(uri)))
}

fn fail_path(uri: &str) -> PathBuf {
    root()
        .join("fail")
        .join("junk-drawer")
        .join(format!("{}.png", md5(uri)))
}

/// The value of a PNG `tEXt` chunk, which is how thumbnails carry `Thumb::MTime`.
fn png_text(png: &[u8], key: &str) -> Option<String> {
    let mut rest = png.strip_prefix(b"\x89PNG\r\n\x1a\n")?;
    while rest.len() >= 12 {
        let len = u32::from_be_bytes(rest[..4].try_into().ok()?) as usize;
        let kind = &rest[4..8];
        let data = rest.get(8..8 + len)?;
        if kind == b"tEXt" {
            if let Some(nul) = data.iter().position(|&b| b == 0)
                && &data[..nul] == key.as_bytes()
            {
                return Some(String::from_utf8_lossy(&data[nul + 1..]).into_owned());
            }
        } else if kind == b"IDAT" {
            // Text chunks that matter come before the image data.
            return None;
        }
        rest = rest.get(12 + len..)?;
    }
    None
}

/// A `[Thumbnailer Entry]` from `$XDG_DATA_DIRS/thumbnailers`.
struct Thumbnailer {
    exec: Vec<String>,
    mime_types: Vec<String>,
}

fn parse_thumbnailer(text: &str) -> Option<Thumbnailer> {
    let kf = glib::KeyFile::new();
    kf.load_from_data(text, glib::KeyFileFlags::NONE).ok()?;
    let group = "Thumbnailer Entry";
    if let Ok(try_exec) = kf.string(group, "TryExec")
        && glib::find_program_in_path(&try_exec).is_none()
    {
        return None;
    }
    let exec = glib::shell_parse_argv(kf.string(group, "Exec").ok()?.as_str()).ok()?;
    let mime_types = kf
        .string_list(group, "MimeType")
        .ok()?
        .iter()
        .map(|m| m.to_string())
        .collect();
    Some(Thumbnailer {
        exec: exec
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect(),
        mime_types,
    })
}

/// `Exec` with `%i` (input path), `%u` (input URI), `%o` (output path), `%s` (size)
/// and `%%` filled in.
fn expand(exec: &[String], input: &Path, uri: &str, output: &Path, size: i32) -> Vec<String> {
    exec.iter()
        .map(|arg| {
            let mut out = String::new();
            let mut chars = arg.chars();
            while let Some(c) = chars.next() {
                if c != '%' {
                    out.push(c);
                    continue;
                }
                match chars.next() {
                    Some('i') => out.push_str(&input.to_string_lossy()),
                    Some('u') => out.push_str(uri),
                    Some('o') => out.push_str(&output.to_string_lossy()),
                    Some('s') => out.push_str(&size.to_string()),
                    Some('%') => out.push('%'),
                    Some(other) => {
                        out.push('%');
                        out.push(other);
                    }
                    None => out.push('%'),
                }
            }
            out
        })
        .collect()
}

async fn load_thumbnailers() -> Vec<Thumbnailer> {
    let mut dirs = vec![glib::user_data_dir()];
    dirs.extend(glib::system_data_dirs());
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for dir in dirs {
        let dir = gio::File::for_path(dir.join("thumbnailers"));
        let Ok(children) = dir
            .enumerate_children_future("standard::name", gio::FileQueryInfoFlags::NONE, PRIO)
            .await
        else {
            continue;
        };
        while let Ok(batch) = children.next_files_future(64, PRIO).await {
            if batch.is_empty() {
                break;
            }
            for info in batch {
                let name = info.name();
                // Earlier directories win, as with desktop entries.
                if !name.to_string_lossy().ends_with(".thumbnailer") || !seen.insert(name.clone()) {
                    continue;
                }
                if let Ok((bytes, _)) = dir.child(&name).load_contents_future().await
                    && let Some(t) = parse_thumbnailer(&String::from_utf8_lossy(&bytes))
                {
                    out.push(t);
                }
            }
        }
    }
    out
}

struct Request {
    file: gio::File,
    uri: String,
    mime: String,
    mtime: i64,
    bytes: u64,
    size: Size,
}

type Key = (String, Size);

/// Thumbnails for every window: the textures made so far, the queue, and which
/// images are waiting for which file.
#[derive(Default)]
struct Thumbnails {
    textures: RefCell<HashMap<Key, (i64, Option<gdk::Texture>)>>,
    queue: RefCell<VecDeque<Request>>,
    running: Cell<usize>,
    bound: RefCell<Vec<(glib::WeakRef<gtk::Image>, Key)>>,
    thumbnailers: OnceCell<Vec<Thumbnailer>>,
    loading: Cell<bool>,
}

thread_local! {
    static THUMBNAILS: Rc<Thumbnails> = Rc::new(Thumbnails::default());
}

/// Shows the thumbnail of `info` in `image` once there is one; `image` keeps its icon
/// until then (`VIEW-37`). Call after setting the icon on every bind.
pub fn show(image: &gtk::Image, info: &gio::FileInfo, file: Option<gio::File>) {
    let Some(file) = file.filter(|f| f.has_uri_scheme("file")) else {
        return;
    };
    if info.file_type() != gio::FileType::Regular {
        return;
    }
    let uri = file.uri().to_string();
    if file.path().is_some_and(|p| p.starts_with(root())) {
        return;
    }
    let size = Size::for_pixels(image.pixel_size() * image.scale_factor());
    let mtime = info.modification_date_time().map_or(0, |t| t.to_unix());
    THUMBNAILS.with(|t| {
        let key = (uri.clone(), size);
        t.bound
            .borrow_mut()
            .retain(|(w, _)| w.upgrade().is_some_and(|i| &i != image));
        if let Some((when, texture)) = t.textures.borrow().get(&key)
            && *when == mtime
        {
            if let Some(texture) = texture {
                image.set_paintable(Some(texture));
            }
            return;
        }
        t.bound.borrow_mut().push((image.downgrade(), key));
        let mut queue = t.queue.borrow_mut();
        queue.retain(|r| !(r.uri == uri && r.size == size));
        queue.push_front(Request {
            file,
            uri,
            mime: info
                .content_type()
                .map(|c| c.to_string())
                .unwrap_or_default(),
            mtime,
            bytes: info.size().max(0) as u64,
            size,
        });
        drop(queue);
        t.clone().pump();
    });
}

impl Thumbnails {
    fn pump(self: Rc<Self>) {
        if self.thumbnailers.get().is_none() {
            if !self.loading.replace(true) {
                glib::spawn_future_local(async move {
                    let list = load_thumbnailers().await;
                    let _ = self.thumbnailers.set(list);
                    self.pump();
                });
            }
            return;
        }
        while self.running.get() < WORKERS {
            let Some(request) = self.queue.borrow_mut().pop_front() else {
                return;
            };
            // Dropped if nothing shows this file any more (scrolled away, folder left).
            let key = (request.uri.clone(), request.size);
            let wanted = self
                .bound
                .borrow()
                .iter()
                .any(|(w, k)| *k == key && w.upgrade().is_some());
            if !wanted {
                continue;
            }
            self.running.set(self.running.get() + 1);
            let this = self.clone();
            glib::spawn_future_local(async move {
                let texture = this.make(&request).await;
                this.running.set(this.running.get() - 1);
                this.deliver(key, request.mtime, texture);
                this.pump();
            });
        }
    }

    fn deliver(&self, key: Key, mtime: i64, texture: Option<gdk::Texture>) {
        if let Some(texture) = &texture {
            self.bound.borrow_mut().retain(|(w, k)| {
                let Some(image) = w.upgrade() else {
                    return false;
                };
                if *k == key {
                    image.set_paintable(Some(texture));
                    return false;
                }
                true
            });
        }
        self.textures.borrow_mut().insert(key, (mtime, texture));
    }

    /// The cached thumbnail when it is current, else a new one; `None` when the file
    /// can't be thumbnailed or failed before at this mtime.
    async fn make(&self, r: &Request) -> Option<gdk::Texture> {
        let mtime = r.mtime.to_string();
        let cached = gio::File::for_path(path_for(&r.uri, r.size));
        if let Ok((png, _)) = cached.load_contents_future().await
            && png_text(&png, "Thumb::MTime").as_deref() == Some(mtime.as_str())
        {
            return gdk::Texture::from_bytes(&glib::Bytes::from(&png[..])).ok();
        }
        let failed = gio::File::for_path(fail_path(&r.uri));
        if let Ok((png, _)) = failed.load_contents_future().await
            && png_text(&png, "Thumb::MTime").as_deref() == Some(mtime.as_str())
        {
            return None;
        }
        // Nothing here can make one: not a failure to remember.
        if !self.handles(r) {
            return None;
        }
        let made = match self.external(r).await {
            Some(pixbuf) => Some(pixbuf),
            None => self.decode(r).await,
        };
        let Some(pixbuf) = made else {
            let blank = gdk_pixbuf::Pixbuf::new(gdk_pixbuf::Colorspace::Rgb, true, 8, 1, 1)?;
            save(&blank, &failed, &r.uri, &mtime).await;
            return None;
        };
        let png = save(&pixbuf, &cached, &r.uri, &mtime).await?;
        gdk::Texture::from_bytes(&glib::Bytes::from_owned(png)).ok()
    }

    fn handles(&self, r: &Request) -> bool {
        let external = self
            .thumbnailers
            .get()
            .is_some_and(|ts| ts.iter().any(|t| t.mime_types.contains(&r.mime)));
        external || (decodable(&r.mime) && r.bytes <= MAX_DECODE)
    }

    async fn external(&self, r: &Request) -> Option<gdk_pixbuf::Pixbuf> {
        let thumbnailer = self
            .thumbnailers
            .get()?
            .iter()
            .find(|t| t.mime_types.contains(&r.mime))?;
        let input = r.file.path()?;
        let tmp = root().join(format!(
            "junk-drawer-{}-{}.png",
            std::process::id(),
            md5(&r.uri)
        ));
        let argv = expand(&thumbnailer.exec, &input, &r.uri, &tmp, r.size.pixels());
        let argv: Vec<&std::ffi::OsStr> = argv.iter().map(std::ffi::OsStr::new).collect();
        mkdirs(&gio::File::for_path(root())).await;
        let process = gio::Subprocess::newv(
            &argv,
            gio::SubprocessFlags::STDOUT_SILENCE | gio::SubprocessFlags::STDERR_SILENCE,
        )
        .ok()?;
        let ok = process.wait_check_future().await.is_ok();
        let out = gio::File::for_path(&tmp);
        let bytes = out.load_contents_future().await.ok().map(|(b, _)| b);
        let _ = out.delete_future(PRIO).await;
        if !ok {
            return None;
        }
        let stream = gio::MemoryInputStream::from_bytes(&glib::Bytes::from(&bytes?[..]));
        let pixbuf = gdk_pixbuf::Pixbuf::from_stream(&stream, gio::Cancellable::NONE).ok()?;
        Some(fit(pixbuf, r.size.pixels()))
    }

    async fn decode(&self, r: &Request) -> Option<gdk_pixbuf::Pixbuf> {
        if !decodable(&r.mime) || r.bytes > MAX_DECODE {
            return None;
        }
        let stream = r.file.read_future(PRIO).await.ok()?;
        let px = r.size.pixels();
        let pixbuf = gdk_pixbuf::Pixbuf::from_stream_at_scale_future(&stream, px, px, true)
            .await
            .ok()?;
        pixbuf.apply_embedded_orientation()
    }
}

/// Whether GdkPixbuf can load `mime`.
fn decodable(mime: &str) -> bool {
    gdk_pixbuf::Pixbuf::formats()
        .iter()
        .any(|f| f.mime_types().iter().any(|m| m.as_str() == mime))
}

/// `pixbuf` scaled down to fit in `px` square, keeping its shape.
fn fit(pixbuf: gdk_pixbuf::Pixbuf, px: i32) -> gdk_pixbuf::Pixbuf {
    let (w, h) = (pixbuf.width(), pixbuf.height());
    if w <= px && h <= px {
        return pixbuf;
    }
    let scale = px as f64 / w.max(h) as f64;
    let (nw, nh) = (
        ((w as f64 * scale).round() as i32).max(1),
        ((h as f64 * scale).round() as i32).max(1),
    );
    pixbuf
        .scale_simple(nw, nh, gdk_pixbuf::InterpType::Bilinear)
        .unwrap_or(pixbuf)
}

/// Creates `dir` and any missing parents.
async fn mkdirs(dir: &gio::File) {
    match dir.make_directory_future(PRIO).await {
        Err(e) if e.matches(gio::IOErrorEnum::NotFound) => {
            if let Some(parent) = dir.parent() {
                Box::pin(mkdirs(&parent)).await;
            }
            let _ = dir.make_directory_future(PRIO).await;
        }
        _ => {}
    }
}

/// Writes `pixbuf` as a thumbnail PNG for `uri`, private to the user as the standard
/// asks, and returns the PNG.
async fn save(
    pixbuf: &gdk_pixbuf::Pixbuf,
    to: &gio::File,
    uri: &str,
    mtime: &str,
) -> Option<Vec<u8>> {
    let png = pixbuf
        .save_to_bufferv(
            "png",
            &[
                ("tEXt::Thumb::URI", uri),
                ("tEXt::Thumb::MTime", mtime),
                ("tEXt::Software", "Junk Drawer"),
            ],
        )
        .ok()?;
    if let Some(dir) = to.parent() {
        mkdirs(&dir).await;
    }
    let _ = to
        .replace_contents_future(png.clone(), None, false, gio::FileCreateFlags::PRIVATE)
        .await;
    Some(png)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnail_paths() {
        // The example from the Thumbnail Managing Standard.
        assert_eq!(
            md5("file:///home/jens/photos/me.png"),
            "c6ee772d9e49320e97ec29a7eb5b1697"
        );
        assert!(
            path_for("file:///a", Size::Large).ends_with(format!("large/{}.png", md5("file:///a")))
        );
        assert_eq!(Size::for_pixels(96), Size::Normal);
        assert_eq!(Size::for_pixels(192), Size::Large);
    }

    #[test]
    fn reads_png_text() {
        let pixbuf = gdk_pixbuf::Pixbuf::new(gdk_pixbuf::Colorspace::Rgb, true, 8, 2, 2).unwrap();
        let Ok(png) = pixbuf.save_to_bufferv(
            "png",
            &[
                ("tEXt::Thumb::MTime", "1700000000"),
                ("tEXt::Thumb::URI", "file:///x"),
            ],
        ) else {
            eprintln!("skipping: no PNG saver");
            return;
        };
        assert_eq!(
            png_text(&png, "Thumb::MTime").as_deref(),
            Some("1700000000")
        );
        assert_eq!(png_text(&png, "Thumb::URI").as_deref(), Some("file:///x"));
        assert_eq!(png_text(&png, "Thumb::Size"), None);
        assert_eq!(png_text(b"not a png", "Thumb::MTime"), None);
    }

    #[test]
    fn expands_exec() {
        let exec: Vec<String> = ["thumb", "-s", "%s", "%u", "%o", "%i", "100%%"]
            .map(String::from)
            .into();
        assert_eq!(
            expand(
                &exec,
                Path::new("/a b.pdf"),
                "file:///a%20b.pdf",
                Path::new("/t.png"),
                256
            ),
            [
                "thumb",
                "-s",
                "256",
                "file:///a%20b.pdf",
                "/t.png",
                "/a b.pdf",
                "100%"
            ]
        );
    }
}
