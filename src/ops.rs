use crate::{conflict, progress};
use gtk::{gio, glib, prelude::*};
use std::cell::Cell;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Transfer {
    Copy,
    Move,
}

const NOFOLLOW: gio::FileQueryInfoFlags = gio::FileQueryInfoFlags::NOFOLLOW_SYMLINKS;
const PRIO: glib::Priority = glib::Priority::DEFAULT;

/// What to do about an item whose name is taken in the destination (`OPS-11`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Choice {
    Cancel,
    Skip,
    Rename,
    /// Replace the existing item, or merge when both are folders.
    Replace,
}

pub struct Conflict {
    pub dest: gio::File,
    pub both_dirs: bool,
    /// The next free `name (copy).ext` name, for the Rename entry.
    pub free_name: String,
}

pub struct Answer {
    pub choice: Choice,
    /// The new name when `choice` is `Rename`.
    pub name: String,
    pub all: bool,
}

pub type Ask = Box<dyn Fn(Conflict) -> Pin<Box<dyn Future<Output = Answer>>>>;
type OnProgress = Box<dyn Fn(u64, u64, &gio::File)>;

/// One copy or move: how to resolve conflicts, and its progress (`OPS-12`).
pub struct Job {
    mode: Transfer,
    ask: Ask,
    all: Cell<Option<Choice>>,
    cancellable: gio::Cancellable,
    done: Cell<u64>,
    total: Cell<u64>,
    on_progress: OnProgress,
}

impl Job {
    pub fn new(
        mode: Transfer,
        ask: Ask,
        cancellable: gio::Cancellable,
        on_progress: impl Fn(u64, u64, &gio::File) + 'static,
    ) -> Rc<Self> {
        Rc::new(Self {
            mode,
            ask,
            all: Cell::new(None),
            cancellable,
            done: Cell::new(0),
            total: Cell::new(0),
            on_progress: Box::new(on_progress),
        })
    }

    fn advance(&self, done: u64, file: &gio::File) {
        self.done.set(done);
        (self.on_progress)(done, self.total.get(), file);
    }

    fn check(&self) -> Result<(), glib::Error> {
        if self.cancellable.is_cancelled() {
            return Err(cancelled());
        }
        Ok(())
    }

    async fn resolve(&self, dest: &gio::File, both_dirs: bool) -> Answer {
        let name = dest.basename().unwrap_or_default();
        let free = match dest.parent() {
            Some(dir) => unique_child(&dir, &name).await,
            None => dest.clone(),
        };
        let free_name = free
            .basename()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if let Some(choice) = self.all.get() {
            return Answer {
                choice,
                name: free_name,
                all: true,
            };
        }
        let answer = (self.ask)(Conflict {
            dest: dest.clone(),
            both_dirs,
            free_name,
        })
        .await;
        if answer.all {
            self.all.set(Some(answer.choice));
        }
        answer
    }
}

fn cancelled() -> glib::Error {
    glib::Error::new(gio::IOErrorEnum::Cancelled, "Cancelled")
}

/// Copies or moves `sources` into `dest_dir`, asking about name clashes and showing
/// progress, then shows any errors in one dialog.
pub fn transfer(
    parent: Option<gtk::Window>,
    sources: Vec<gio::File>,
    dest_dir: gio::File,
    mode: Transfer,
) {
    glib::spawn_future_local(async move {
        let window = progress::Window::new(parent.as_ref(), mode, &sources, &dest_dir);
        let job = Job::new(
            mode,
            window.while_asking(conflict::asker(parent.clone())),
            window.cancellable(),
            glib::clone!(
                #[strong]
                window,
                move |done, total, file| window.update(done, total, file)
            ),
        );
        let errors = run(&job, &sources, &dest_dir).await;
        window.finish();
        if !errors.is_empty() {
            gtk::AlertDialog::builder()
                .message(match mode {
                    Transfer::Copy => "Some items could not be copied",
                    Transfer::Move => "Some items could not be moved",
                })
                .detail(errors.join("\n"))
                .build()
                .show(parent.as_ref());
        }
    });
}

/// Runs `job` over `sources`, returning the failures. A cancelled job stops early and
/// isn't a failure.
async fn run(job: &Rc<Job>, sources: &[gio::File], dest_dir: &gio::File) -> Vec<String> {
    let mut sizes = Vec::new();
    for src in sources {
        sizes.push(tree_size(src).await);
    }
    job.total.set(sizes.iter().sum());
    let mut errors = Vec::new();
    let mut base = 0;
    for (src, size) in sources.iter().zip(sizes) {
        match transfer_one(job, src, dest_dir).await {
            Err(e) if e.matches(gio::IOErrorEnum::Cancelled) => break,
            Err(e) => errors.push(format!("{}: {}", src.parse_name(), e.message())),
            Ok(()) => {}
        }
        // Moves within a filesystem and skipped items report no bytes as they go.
        base += size;
        job.advance(base, src);
    }
    errors
}

/// Bytes in the regular files under `file`, not following symlinks.
async fn tree_size(file: &gio::File) -> u64 {
    let Ok(info) = file
        .query_info_future("standard::type,standard::size", NOFOLLOW, PRIO)
        .await
    else {
        return 0;
    };
    if info.file_type() != gio::FileType::Directory {
        return info.size() as u64;
    }
    let mut total = 0;
    for child in children(file).await.unwrap_or_default() {
        total += Box::pin(tree_size(&file.child(child.name()))).await;
    }
    total
}

/// Moves `files` to the XDG trash, then shows any errors in one dialog.
pub fn trash(parent: Option<gtk::Window>, files: Vec<gio::File>) {
    glib::spawn_future_local(async move {
        let mut errors = Vec::new();
        for file in &files {
            if let Err(e) = file.trash_future(PRIO).await {
                errors.push(format!("{}: {}", file.parse_name(), e.message()));
            }
        }
        if !errors.is_empty() {
            gtk::AlertDialog::builder()
                .message("Some items could not be moved to the trash")
                .detail(errors.join("\n"))
                .build()
                .show(parent.as_ref());
        }
    });
}

/// Deletes `files` (folders with everything in them), then shows any errors in one dialog.
pub fn delete(parent: Option<gtk::Window>, files: Vec<gio::File>) {
    glib::spawn_future_local(async move {
        let errors = delete_all(&files).await;
        if !errors.is_empty() {
            gtk::AlertDialog::builder()
                .message("Some items could not be deleted")
                .detail(errors.join("\n"))
                .build()
                .show(parent.as_ref());
        }
    });
}

/// Removes Recent entries (`INT-12`): one plain delete each, never recursive, so a
/// folder's entry can't reach the folder's contents.
pub fn forget(parent: Option<gtk::Window>, entries: Vec<gio::File>) {
    glib::spawn_future_local(async move {
        let mut errors = Vec::new();
        for entry in &entries {
            if let Err(e) = entry.delete_future(PRIO).await {
                errors.push(format!("{}: {}", entry.parse_name(), e.message()));
            }
        }
        if !errors.is_empty() {
            gtk::AlertDialog::builder()
                .message("Some items could not be removed from Recent")
                .detail(errors.join("\n"))
                .build()
                .show(parent.as_ref());
        }
    });
}

async fn delete_all(files: &[gio::File]) -> Vec<String> {
    let mut errors = Vec::new();
    for file in files {
        if let Err(e) = delete_tree(file).await {
            errors.push(format!("{}: {}", file.parse_name(), e.message()));
        }
    }
    errors
}

async fn transfer_one(
    job: &Rc<Job>,
    src: &gio::File,
    dest_dir: &gio::File,
) -> Result<(), glib::Error> {
    let Some(name) = src.basename() else {
        return Ok(());
    };
    if dest_dir.equal(src) || dest_dir.has_prefix(src) {
        return Err(glib::Error::new(
            gio::IOErrorEnum::InvalidArgument,
            "Cannot put a folder inside itself",
        ));
    }
    let same_dir = src.parent().is_some_and(|p| p.equal(dest_dir));
    if same_dir && job.mode == Transfer::Move {
        return Ok(());
    }
    let dest = if same_dir {
        unique_child(dest_dir, &name).await
    } else {
        dest_dir.child(&name)
    };
    place(job, src, dest).await
}

/// Puts `src` at `dest`, asking first when `dest` exists (`OPS-11`).
async fn place(job: &Rc<Job>, src: &gio::File, dest: gio::File) -> Result<(), glib::Error> {
    job.check()?;
    let Ok(existing) = file_type(&dest).await else {
        return put(job, src, &dest).await;
    };
    let both_dirs =
        existing == gio::FileType::Directory && file_type(src).await? == gio::FileType::Directory;
    let answer = job.resolve(&dest, both_dirs).await;
    match answer.choice {
        Choice::Skip => Ok(()),
        Choice::Cancel => {
            job.cancellable.cancel();
            Err(cancelled())
        }
        Choice::Rename => match dest.parent() {
            Some(dir) => Box::pin(place(job, src, dir.child(&answer.name))).await,
            None => Ok(()),
        },
        Choice::Replace if both_dirs => {
            for child in children(src).await? {
                let name = child.name();
                Box::pin(place(job, &src.child(&name), dest.child(&name))).await?;
            }
            match job.mode {
                // Skipped items stay behind, and so does their folder.
                Transfer::Move => match src.delete_future(PRIO).await {
                    Err(e) if e.matches(gio::IOErrorEnum::NotEmpty) => Ok(()),
                    r => r,
                },
                Transfer::Copy => Ok(()),
            }
        }
        Choice::Replace => {
            if src.has_prefix(&dest) {
                return Err(glib::Error::new(
                    gio::IOErrorEnum::InvalidArgument,
                    "Cannot replace a folder with an item inside it",
                ));
            }
            delete_tree(&dest).await?;
            put(job, src, &dest).await
        }
    }
}

/// Copies or moves `src` to `dest`, which doesn't exist.
async fn put(job: &Rc<Job>, src: &gio::File, dest: &gio::File) -> Result<(), glib::Error> {
    match job.mode {
        Transfer::Copy => copy_tree(job, src, dest).await,
        Transfer::Move => match one_file(job, src, dest, Transfer::Move).await {
            // Directories can't be moved across filesystems in one call.
            Err(e) if e.matches(gio::IOErrorEnum::WouldRecurse) => {
                copy_tree(job, src, dest).await?;
                delete_tree(src).await
            }
            r => r,
        },
    }
}

async fn copy_tree(job: &Rc<Job>, src: &gio::File, dest: &gio::File) -> Result<(), glib::Error> {
    job.check()?;
    if file_type(src).await? != gio::FileType::Directory {
        return one_file(job, src, dest, Transfer::Copy).await;
    }
    dest.make_directory_future(PRIO).await?;
    for child in children(src).await? {
        let name = child.name();
        Box::pin(copy_tree(job, &src.child(&name), &dest.child(&name))).await?;
    }
    Ok(())
}

/// `g_file_copy` or `g_file_move` with progress and the job's cancellable. A cancelled
/// copy removes the partly written file.
async fn one_file(
    job: &Rc<Job>,
    src: &gio::File,
    dest: &gio::File,
    mode: Transfer,
) -> Result<(), glib::Error> {
    let base = job.done.get();
    let progress: Box<dyn FnMut(i64, i64)> = Box::new(glib::clone!(
        #[strong]
        job,
        #[strong]
        src,
        move |current, _| job.advance(base + current.max(0) as u64, &src)
    ));
    let flags = gio::FileCopyFlags::NOFOLLOW_SYMLINKS | gio::FileCopyFlags::ALL_METADATA;
    let (to, cancellable) = (dest.clone(), job.cancellable.clone());
    let result = gio::GioFuture::new(src, move |src, _, send| {
        let done = move |r| send.resolve(r);
        match mode {
            Transfer::Copy => {
                src.copy_async(&to, flags, PRIO, Some(&cancellable), Some(progress), done)
            }
            Transfer::Move => {
                src.move_async(&to, flags, PRIO, Some(&cancellable), Some(progress), done)
            }
        }
    })
    .await;
    if let Err(e) = &result
        && e.matches(gio::IOErrorEnum::Cancelled)
        && mode == Transfer::Copy
    {
        let _ = dest.delete_future(PRIO).await;
    }
    result
}

async fn file_type(file: &gio::File) -> Result<gio::FileType, glib::Error> {
    Ok(file
        .query_info_future("standard::type", NOFOLLOW, PRIO)
        .await?
        .file_type())
}

async fn children(dir: &gio::File) -> Result<Vec<gio::FileInfo>, glib::Error> {
    let enumerator = dir
        .enumerate_children_future("standard::name,standard::type", NOFOLLOW, PRIO)
        .await?;
    let mut out = Vec::new();
    loop {
        let batch = enumerator.next_files_future(64, PRIO).await?;
        if batch.is_empty() {
            return Ok(out);
        }
        out.extend(batch);
    }
}

async fn delete_tree(file: &gio::File) -> Result<(), glib::Error> {
    if file_type(file).await? == gio::FileType::Directory {
        for child in children(file).await? {
            Box::pin(delete_tree(&file.child(child.name()))).await?;
        }
    }
    file.delete_future(PRIO).await
}

/// `name (copy).ext`, then `name (copy 2).ext`, … — the first one that doesn't exist.
async fn unique_child(dir: &gio::File, name: &Path) -> gio::File {
    let stem = name
        .file_stem()
        .unwrap_or(name.as_os_str())
        .to_string_lossy();
    let ext = name
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    for n in 1.. {
        let suffix = if n == 1 {
            "copy".to_owned()
        } else {
            format!("copy {n}")
        };
        let candidate = dir.child(format!("{stem} ({suffix}){ext}"));
        if candidate
            .query_info_future("standard::name", NOFOLLOW, PRIO)
            .await
            .is_err()
        {
            return candidate;
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::fs;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("junk-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A job that answers conflicts from `answers` in order, recording each
    /// conflict's name and suggested free name.
    fn job(
        mode: Transfer,
        answers: Vec<(Choice, &'static str, bool)>,
    ) -> (Rc<Job>, Rc<RefCell<Vec<String>>>) {
        let answers = Rc::new(RefCell::new(answers.into_iter()));
        let asked = Rc::new(RefCell::new(Vec::new()));
        let log = asked.clone();
        let ask: Ask = Box::new(move |c: Conflict| {
            let name = c.dest.basename().unwrap().to_string_lossy().into_owned();
            log.borrow_mut().push(format!(
                "{name}>{}{}",
                c.free_name,
                if c.both_dirs { "/" } else { "" }
            ));
            let (choice, new, all) = answers.borrow_mut().next().expect("unexpected conflict");
            let name = new.to_owned();
            Box::pin(async move { Answer { choice, name, all } })
        });
        (
            Job::new(mode, ask, gio::Cancellable::new(), |_, _, _| {}),
            asked,
        )
    }

    fn go(job: &Rc<Job>, srcs: &[std::path::PathBuf], dest: &Path) -> Vec<String> {
        let srcs: Vec<_> = srcs.iter().map(gio::File::for_path).collect();
        glib::MainContext::new().block_on(run(job, &srcs, &gio::File::for_path(dest)))
    }

    fn copy(src: &Path, dest: &Path, mode: Transfer) -> Vec<String> {
        go(&job(mode, vec![]).0, &[src.to_owned()], dest)
    }

    #[test]
    fn copies_directory_recursively() {
        let t = scratch("copy-tree");
        fs::create_dir_all(t.join("a/b")).unwrap();
        fs::write(t.join("a/b/f.txt"), "hi").unwrap();
        fs::create_dir(t.join("out")).unwrap();
        assert!(copy(&t.join("a"), &t.join("out"), Transfer::Copy).is_empty());
        assert_eq!(fs::read_to_string(t.join("out/a/b/f.txt")).unwrap(), "hi");
        assert!(t.join("a/b/f.txt").exists());
    }

    #[test]
    fn copy_into_same_dir_makes_unique_names() {
        let t = scratch("dup");
        fs::write(t.join("n.txt"), "x").unwrap();
        copy(&t.join("n.txt"), &t, Transfer::Copy);
        copy(&t.join("n.txt"), &t, Transfer::Copy);
        assert!(t.join("n (copy).txt").exists());
        assert!(t.join("n (copy 2).txt").exists());
    }

    #[test]
    fn moves_and_refuses_self_nesting() {
        let t = scratch("move");
        fs::create_dir_all(t.join("a")).unwrap();
        fs::create_dir(t.join("b")).unwrap();
        assert_eq!(copy(&t.join("a"), &t.join("a"), Transfer::Move).len(), 1);
        assert!(copy(&t.join("a"), &t.join("b"), Transfer::Move).is_empty());
        assert!(t.join("b/a").is_dir() && !t.join("a").exists());
        assert_eq!(copy(&t.join("b"), &t.join("b/a"), Transfer::Copy).len(), 1);
    }

    #[test]
    fn deletes_trees_and_reports_failures() {
        let t = scratch("delete");
        fs::create_dir_all(t.join("a/b")).unwrap();
        fs::write(t.join("a/b/f"), "x").unwrap();
        fs::write(t.join("g"), "y").unwrap();
        let files = [t.join("a"), t.join("g"), t.join("missing")].map(gio::File::for_path);
        let errors = glib::MainContext::new().block_on(delete_all(&files));
        assert!(!t.join("a").exists() && !t.join("g").exists());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("missing"));
    }

    /// `OPS-8`, `OPS-11`: each answer to a name clash.
    #[test]
    fn conflicts_follow_the_answer() {
        let t = scratch("conflicts");
        let setup = |t: &Path| {
            let _ = fs::remove_dir_all(t.join("out"));
            fs::create_dir_all(t.join("out")).unwrap();
            fs::write(t.join("f"), "new").unwrap();
            fs::write(t.join("out/f"), "old").unwrap();
        };
        let read = |p: &str| fs::read_to_string(t.join(p)).unwrap();

        setup(&t);
        let (j, asked) = job(Transfer::Copy, vec![(Choice::Skip, "", false)]);
        assert!(go(&j, &[t.join("f")], &t.join("out")).is_empty());
        assert_eq!(read("out/f"), "old");
        assert_eq!(*asked.borrow(), ["f>f (copy)"]);

        setup(&t);
        let (j, _) = job(Transfer::Copy, vec![(Choice::Replace, "", false)]);
        go(&j, &[t.join("f")], &t.join("out"));
        assert_eq!(read("out/f"), "new");

        setup(&t);
        let (j, _) = job(Transfer::Move, vec![(Choice::Rename, "g", false)]);
        go(&j, &[t.join("f")], &t.join("out"));
        assert_eq!((read("out/f"), read("out/g")), ("old".into(), "new".into()));
        assert!(!t.join("f").exists());

        setup(&t);
        let (j, _) = job(Transfer::Copy, vec![(Choice::Cancel, "", false)]);
        fs::write(t.join("h"), "h").unwrap();
        go(&j, &[t.join("f"), t.join("h")], &t.join("out"));
        assert!(!t.join("out/h").exists(), "cancel stops the rest");
    }

    /// `OPS-11`: two folders merge, asking again inside, and "all" is remembered.
    #[test]
    fn folders_merge_and_apply_to_all() {
        let t = scratch("merge");
        for p in ["src/d/sub", "out/d"] {
            fs::create_dir_all(t.join(p)).unwrap();
        }
        for (p, s) in [
            ("src/d/a", "new"),
            ("src/d/b", "new"),
            ("src/d/sub/c", "c"),
            ("out/d/a", "old"),
            ("out/d/b", "old"),
            ("out/d/keep", "k"),
        ] {
            fs::write(t.join(p), s).unwrap();
        }
        let (j, asked) = job(
            Transfer::Move,
            vec![(Choice::Replace, "", false), (Choice::Skip, "", true)],
        );
        assert!(go(&j, &[t.join("src/d")], &t.join("out")).is_empty());
        assert_eq!(asked.borrow()[0], "d>d (copy)/");
        assert_eq!(asked.borrow().len(), 2, "the next clash reused the answer");
        let read = |p: &str| fs::read_to_string(t.join(p)).unwrap();
        assert_eq!(
            (read("out/d/a"), read("out/d/b")),
            ("old".into(), "old".into())
        );
        assert_eq!(read("out/d/keep"), "k");
        assert_eq!(read("out/d/sub/c"), "c");
        assert!(!t.join("src/d/sub").exists(), "unclashing items moved");
        assert!(t.join("src/d/a").exists(), "skipped items stay behind");
    }

    /// `OPS-12`: cancelling removes the partly copied file, stops, and isn't an error.
    #[test]
    fn cancel_stops_the_transfer() {
        let t = scratch("cancel");
        fs::create_dir(t.join("out")).unwrap();
        fs::write(t.join("a"), vec![7u8; 1000]).unwrap();
        fs::write(t.join("b"), vec![7u8; 500]).unwrap();
        let cancellable = gio::Cancellable::new();
        let seen = Rc::new(Cell::new(0u64));
        let (c, s) = (cancellable.clone(), seen.clone());
        let ask: Ask = Box::new(|_| unreachable!());
        let j = Job::new(Transfer::Copy, ask, cancellable, move |_, total, _| {
            s.set(total);
            c.cancel();
        });
        assert!(go(&j, &[t.join("a"), t.join("b")], &t.join("out")).is_empty());
        assert_eq!(seen.get(), 1500, "the total is counted first");
        // Cancelled during the first copy: it is removed, and the second never starts.
        assert!(!t.join("out/a").exists() && !t.join("out/b").exists());
    }
}
