use gtk::{gio, glib, prelude::*};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

const PRIO: glib::Priority = glib::Priority::DEFAULT;
const BATCH: i32 = 200;

/// The items of one folder, kept in step with the disk (`NAV-13`).
///
/// Stands in for `gtk::DirectoryList`, whose monitor stops applying changes for good once
/// the lookup behind one event fails because the file is already gone.
pub struct Listing {
    attrs: &'static str,
    store: gio::ListStore,
    file: RefCell<Option<gio::File>>,
    loading: Cell<bool>,
    /// Bumped by every `set_file`, so work for an earlier folder drops its results.
    generation: Cell<u64>,
    monitor: RefCell<Option<gio::FileMonitor>>,
    /// Changes that arrive while the folder is still being read.
    pending: RefCell<Vec<Change>>,
    /// The newest change number per file. A lookup applies its result only if no newer
    /// change to that file came in meanwhile.
    newest: RefCell<HashMap<String, u64>>,
    counter: Cell<u64>,
    on_loading: RefCell<Vec<LoadingCallback>>,
}

type LoadingCallback = Rc<dyn Fn(&Listing)>;

enum Change {
    /// Look the file up again, and list it or replace its entry.
    Look(gio::File),
    Forget(gio::File),
}

impl Listing {
    pub fn new(attrs: &'static str, file: Option<&gio::File>) -> Rc<Self> {
        let listing = Rc::new(Self {
            attrs,
            store: gio::ListStore::new::<gio::FileInfo>(),
            file: RefCell::default(),
            loading: Cell::new(false),
            generation: Cell::new(0),
            monitor: RefCell::default(),
            pending: RefCell::default(),
            newest: RefCell::default(),
            counter: Cell::new(0),
            on_loading: RefCell::default(),
        });
        listing.set_file(file);
        listing
    }

    pub fn model(&self) -> gio::ListStore {
        self.store.clone()
    }

    pub fn file(&self) -> Option<gio::File> {
        self.file.borrow().clone()
    }

    pub fn is_loading(&self) -> bool {
        self.loading.get()
    }

    pub fn connect_loading_notify(&self, f: impl Fn(&Listing) + 'static) {
        self.on_loading.borrow_mut().push(Rc::new(f));
    }

    /// Lists `file` from scratch. Also the way to reload the current folder.
    pub fn set_file(self: &Rc<Self>, file: Option<&gio::File>) {
        let generation = self.generation.get() + 1;
        self.generation.set(generation);
        self.monitor.borrow_mut().take();
        self.pending.borrow_mut().clear();
        self.newest.borrow_mut().clear();
        self.store.remove_all();
        *self.file.borrow_mut() = file.cloned();
        let Some(dir) = file.cloned() else {
            self.set_loading(false);
            return;
        };
        self.set_loading(true);

        if let Ok(monitor) =
            dir.monitor_directory(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE)
        {
            let this = Rc::downgrade(self);
            monitor.connect_changed(move |_, file, other, event| {
                if let Some(this) = this.upgrade() {
                    this.on_event(file, other, event);
                }
            });
            *self.monitor.borrow_mut() = Some(monitor);
        }

        let (this, attrs) = (Rc::downgrade(self), self.attrs);
        glib::spawn_future_local(async move {
            let infos = read(&dir, attrs, &this, generation).await;
            let Some(this) = this.upgrade().filter(|l| l.generation.get() == generation) else {
                return;
            };
            this.store.splice(0, 0, &infos);
            this.set_loading(false);
            let pending = std::mem::take(&mut *this.pending.borrow_mut());
            for change in pending {
                this.apply(change);
            }
        });
    }

    fn set_loading(&self, loading: bool) {
        if self.loading.replace(loading) == loading {
            return;
        }
        let callbacks = self.on_loading.borrow().clone();
        for f in callbacks {
            f(self);
        }
    }

    fn on_event(
        self: &Rc<Self>,
        file: &gio::File,
        other: Option<&gio::File>,
        event: gio::FileMonitorEvent,
    ) {
        use gio::FileMonitorEvent as E;
        let dir = self.file();
        if dir.as_ref().is_some_and(|d| d.equal(file)) {
            return;
        }
        let mut changes = Vec::new();
        match event {
            E::Created | E::MovedIn | E::AttributeChanged => {
                changes.push(Change::Look(file.clone()))
            }
            E::Deleted | E::MovedOut => changes.push(Change::Forget(file.clone())),
            E::Renamed => {
                changes.push(Change::Forget(file.clone()));
                if let Some(other) = other {
                    changes.push(Change::Look(other.clone()));
                }
            }
            _ => {}
        }
        for change in changes {
            if self.loading.get() {
                self.pending.borrow_mut().push(change);
            } else {
                self.apply(change);
            }
        }
    }

    fn apply(self: &Rc<Self>, change: Change) {
        match change {
            Change::Forget(file) => {
                self.stamp(&file);
                if let Some(pos) = self.position(&file) {
                    self.store.remove(pos);
                }
            }
            Change::Look(file) => {
                let number = self.stamp(&file);
                let (this, generation) = (Rc::downgrade(self), self.generation.get());
                let attrs = self.attrs;
                glib::spawn_future_local(async move {
                    let info = file
                        .query_info_future(attrs, gio::FileQueryInfoFlags::NONE, PRIO)
                        .await;
                    let Some(this) = this.upgrade().filter(|l| l.generation.get() == generation)
                    else {
                        return;
                    };
                    if this.newest.borrow().get(&key(&file)) != Some(&number) {
                        return;
                    }
                    this.settle(&file, info.ok());
                });
            }
        }
    }

    /// Records a new change to `file` and returns its number.
    fn stamp(&self, file: &gio::File) -> u64 {
        let number = self.counter.get() + 1;
        self.counter.set(number);
        self.newest.borrow_mut().insert(key(file), number);
        number
    }

    /// Puts what a lookup found into the list; a failed lookup means the file is gone.
    fn settle(&self, file: &gio::File, info: Option<gio::FileInfo>) {
        let pos = self.position(file);
        let Some(info) = info else {
            if let Some(pos) = pos {
                self.store.remove(pos);
            }
            return;
        };
        info.set_attribute_object("standard::file", file);
        match pos {
            Some(pos) => self.store.splice(pos, 1, &[info]),
            None => self.store.append(&info),
        }
    }

    fn position(&self, file: &gio::File) -> Option<u32> {
        (0..self.store.n_items()).find(|&i| {
            self.store
                .item(i)
                .and_downcast::<gio::FileInfo>()
                .and_then(|info| crate::dnd::file_of(&info))
                .is_some_and(|f| f.equal(file))
        })
    }
}

fn key(file: &gio::File) -> String {
    file.uri().to_string()
}

/// Reads the whole folder. Gives up quietly (with what it has) on an error, or when the
/// listing is replaced or dropped.
async fn read(
    dir: &gio::File,
    attrs: &str,
    this: &Weak<Listing>,
    generation: u64,
) -> Vec<gio::FileInfo> {
    let mut all = Vec::new();
    let Ok(enumerator) = dir
        .enumerate_children_future(attrs, gio::FileQueryInfoFlags::NONE, PRIO)
        .await
    else {
        return all;
    };
    loop {
        let Ok(batch) = enumerator.next_files_future(BATCH, PRIO).await else {
            break;
        };
        if batch.is_empty() {
            break;
        }
        if this
            .upgrade()
            .is_none_or(|l| l.generation.get() != generation)
        {
            return Vec::new();
        }
        for info in batch {
            info.set_attribute_object("standard::file", &enumerator.child(&info));
            all.push(info);
        }
    }
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("junk-test-listing-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn names(listing: &Listing) -> Vec<String> {
        let mut names: Vec<String> = (0..listing.store.n_items())
            .filter_map(|i| listing.store.item(i).and_downcast::<gio::FileInfo>())
            .map(|info| info.name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn on_disk(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// Runs `body` on a private main context, which it can pump with `settle`.
    fn with_loop(body: impl FnOnce(&glib::MainContext)) {
        let ctx = glib::MainContext::new();
        ctx.with_thread_default(|| body(&ctx)).unwrap();
    }

    fn settle(ctx: &glib::MainContext) {
        let end = Instant::now() + Duration::from_millis(400);
        while Instant::now() < end {
            while ctx.iteration(false) {}
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn open(ctx: &glib::MainContext, dir: &Path) -> Rc<Listing> {
        let listing = Listing::new("standard::name", Some(&gio::File::for_path(dir)));
        settle(ctx);
        assert!(!listing.is_loading());
        listing
    }

    #[test]
    fn lists_what_is_there_and_reloads() {
        let dir = scratch("initial");
        fs::write(dir.join("a"), "").unwrap();
        fs::create_dir(dir.join("b")).unwrap();
        with_loop(|ctx| {
            let listing = open(ctx, &dir);
            assert_eq!(names(&listing), ["a", "b"]);
            assert_eq!(listing.file().unwrap().path().unwrap(), dir);
            listing.set_file(listing.file().as_ref());
            settle(ctx);
            assert_eq!(names(&listing), ["a", "b"]);
            listing.set_file(None);
            assert!(names(&listing).is_empty());
        });
    }

    #[test]
    fn follows_creates_renames_and_deletes() {
        let dir = scratch("follow");
        fs::write(dir.join("a"), "").unwrap();
        with_loop(|ctx| {
            let listing = open(ctx, &dir);
            fs::write(dir.join("new"), "").unwrap();
            fs::rename(dir.join("a"), dir.join("renamed")).unwrap();
            settle(ctx);
            assert_eq!(names(&listing), ["new", "renamed"]);
            fs::remove_file(dir.join("new")).unwrap();
            settle(ctx);
            assert_eq!(names(&listing), ["renamed"]);
        });
    }

    #[test]
    fn a_new_file_touched_at_once_is_still_listed() {
        let dir = scratch("touched");
        with_loop(|ctx| {
            let listing = open(ctx, &dir);
            let file = fs::File::create(dir.join("new")).unwrap();
            file.set_modified(std::time::SystemTime::now() - Duration::from_secs(60))
                .unwrap();
            settle(ctx);
            assert_eq!(names(&listing), ["new"]);
        });
    }

    #[test]
    fn a_file_renamed_twice_in_a_row_does_not_stop_later_changes() {
        let dir = scratch("twice");
        fs::write(dir.join("a"), "").unwrap();
        with_loop(|ctx| {
            let listing = open(ctx, &dir);
            fs::rename(dir.join("a"), dir.join("b")).unwrap();
            fs::rename(dir.join("b"), dir.join("c")).unwrap();
            settle(ctx);
            assert_eq!(names(&listing), ["c"]);
            fs::write(dir.join("later"), "").unwrap();
            settle(ctx);
            assert_eq!(names(&listing), ["c", "later"]);
        });
    }

    #[test]
    fn a_file_gone_before_it_is_listed_does_not_stop_later_changes() {
        let dir = scratch("gone");
        with_loop(|ctx| {
            let listing = open(ctx, &dir);
            fs::write(dir.join("temp"), "").unwrap();
            fs::remove_file(dir.join("temp")).unwrap();
            fs::write(dir.join("kept"), "").unwrap();
            settle(ctx);
            assert_eq!(names(&listing), ["kept"]);
            fs::write(dir.join("later"), "").unwrap();
            settle(ctx);
            assert_eq!(names(&listing), ["kept", "later"]);
        });
    }

    #[test]
    fn follows_moves_in_and_out() {
        let (dir, other) = (scratch("move-a"), scratch("move-b"));
        fs::write(dir.join("x"), "").unwrap();
        fs::write(other.join("y"), "").unwrap();
        with_loop(|ctx| {
            let listing = open(ctx, &dir);
            fs::rename(dir.join("x"), other.join("x")).unwrap();
            fs::rename(other.join("y"), dir.join("y")).unwrap();
            settle(ctx);
            assert_eq!(names(&listing), ["y"]);
        });
    }

    #[test]
    fn a_burst_of_changes_ends_matching_the_disk() {
        let dir = scratch("burst");
        with_loop(|ctx| {
            let listing = open(ctx, &dir);
            for i in 0..150 {
                fs::write(dir.join(format!("f{i}")), "").unwrap();
            }
            for i in (0..150).step_by(2) {
                fs::rename(dir.join(format!("f{i}")), dir.join(format!("g{i}"))).unwrap();
            }
            for i in (0..150).step_by(6) {
                fs::rename(dir.join(format!("g{i}")), dir.join(format!("h{i}"))).unwrap();
            }
            for i in (1..150).step_by(10) {
                fs::remove_file(dir.join(format!("f{i}"))).unwrap();
            }
            settle(ctx);
            assert_eq!(names(&listing), on_disk(&dir));
        });
    }

    #[test]
    fn changes_during_loading_are_not_lost() {
        let dir = scratch("loading");
        for i in 0..500 {
            fs::write(dir.join(format!("f{i}")), "").unwrap();
        }
        with_loop(|ctx| {
            let listing = Listing::new("standard::name", Some(&gio::File::for_path(&dir)));
            assert!(listing.is_loading());
            fs::write(dir.join("during"), "").unwrap();
            fs::remove_file(dir.join("f0")).unwrap();
            settle(ctx);
            assert!(!listing.is_loading());
            assert_eq!(names(&listing), on_disk(&dir));
        });
    }

    #[test]
    fn loading_notifies() {
        let dir = scratch("notify");
        with_loop(|ctx| {
            let seen = Rc::new(RefCell::new(Vec::new()));
            let listing = Listing::new("standard::name", None);
            let log = seen.clone();
            listing.connect_loading_notify(move |l| log.borrow_mut().push(l.is_loading()));
            listing.set_file(Some(&gio::File::for_path(&dir)));
            settle(ctx);
            assert_eq!(*seen.borrow(), [true, false]);
        });
    }
}
