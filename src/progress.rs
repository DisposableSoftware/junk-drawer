use crate::ops::{Ask, Transfer};
use gtk::{gio, glib, prelude::*};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// The progress window of one transfer (`OPS-12`). It appears only if the transfer is
/// still running after a second.
#[derive(Clone)]
pub struct Window(Rc<Inner>);

struct Inner {
    window: gtk::Window,
    bar: gtk::ProgressBar,
    amount: gtk::Label,
    cancellable: gio::Cancellable,
    timer: RefCell<Option<glib::SourceId>>,
    shown: Cell<bool>,
    finished: Cell<bool>,
}

fn name_of(file: &gio::File) -> String {
    file.basename()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| file.parse_name().to_string())
}

impl Window {
    pub fn new(
        parent: Option<&gtk::Window>,
        mode: Transfer,
        sources: &[gio::File],
        dest: &gio::File,
    ) -> Self {
        let verb = match mode {
            Transfer::Copy => "Copying",
            Transfer::Move => "Moving",
        };
        let what = match sources {
            [one] => format!("{verb} “{}”", name_of(one)),
            many => format!("{verb} {} items to “{}”", many.len(), name_of(dest)),
        };
        let window = gtk::Window::builder()
            .resizable(false)
            .title(verb)
            .default_width(380)
            .css_classes(["junk", "progress"])
            .build();
        window.set_transient_for(parent);
        let label = gtk::Label::builder()
            .label(&what)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::Middle)
            .build();
        let bar = gtk::ProgressBar::new();
        let amount = gtk::Label::builder()
            .label("Preparing…")
            .xalign(0.0)
            .hexpand(true)
            .css_classes(["amount"])
            .build();
        let cancel = gtk::Button::with_label("Cancel");
        let row = gtk::Box::builder().spacing(12).build();
        row.append(&amount);
        row.append(&cancel);
        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(10)
            .margin_top(18)
            .margin_bottom(18)
            .margin_start(18)
            .margin_end(18)
            .build();
        content.append(&label);
        content.append(&bar);
        content.append(&row);
        window.set_child(Some(&content));

        let inner = Rc::new(Inner {
            window,
            bar,
            amount,
            cancellable: gio::Cancellable::new(),
            timer: RefCell::default(),
            shown: Cell::new(false),
            finished: Cell::new(false),
        });
        cancel.connect_clicked(glib::clone!(
            #[weak]
            inner,
            move |_| {
                inner.cancellable.cancel();
                inner.window.close();
            }
        ));
        let this = Self(inner);
        this.arm();
        this
    }

    /// Wraps `ask` so the window doesn't appear while a conflict dialog waits for an
    /// answer: the one-second wait starts again afterwards.
    pub fn while_asking(&self, ask: Ask) -> Ask {
        let this = self.clone();
        Box::new(move |conflict| {
            this.hold();
            let (this, answer) = (this.clone(), ask(conflict));
            Box::pin(async move {
                let answer = answer.await;
                this.arm();
                answer
            })
        })
    }

    fn hold(&self) {
        if let Some(timer) = self.0.timer.take() {
            timer.remove();
        }
    }

    fn arm(&self) {
        if self.0.shown.get() || self.0.finished.get() || self.0.timer.borrow().is_some() {
            return;
        }
        let inner = Rc::downgrade(&self.0);
        let timer = glib::timeout_add_local_once(Duration::from_secs(1), move || {
            if let Some(inner) = inner.upgrade() {
                inner.timer.take();
                inner.shown.set(true);
                inner.window.present();
            }
        });
        *self.0.timer.borrow_mut() = Some(timer);
    }

    pub fn cancellable(&self) -> gio::Cancellable {
        self.0.cancellable.clone()
    }

    pub fn update(&self, done: u64, total: u64, _file: &gio::File) {
        let inner = &self.0;
        if total > 0 {
            inner
                .bar
                .set_fraction((done as f64 / total as f64).clamp(0.0, 1.0));
        }
        inner.amount.set_label(&format!(
            "{} of {}",
            glib::format_size(done),
            glib::format_size(total)
        ));
    }

    pub fn finish(&self) {
        self.0.finished.set(true);
        self.hold();
        if self.0.shown.get() {
            self.0.window.close();
        }
    }
}
