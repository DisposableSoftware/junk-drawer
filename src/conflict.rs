use crate::dialog;
use crate::ops::{Answer, Ask, Choice, Conflict};
use gtk::{gio, glib, prelude::*};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::task::{Poll, Waker};

/// Asks about each name clash in a modal dialog over `parent` (`OPS-11`).
pub fn asker(parent: Option<gtk::Window>) -> Ask {
    Box::new(move |conflict| Box::pin(ask(parent.clone(), conflict)))
}

/// The dialog's answer, handed from its buttons to the waiting transfer.
#[derive(Default)]
struct Reply {
    value: RefCell<Option<Answer>>,
    sent: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

impl Reply {
    fn send(&self, answer: Answer) {
        if self.sent.replace(true) {
            return;
        }
        *self.value.borrow_mut() = Some(answer);
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }
}

fn name_of(file: &gio::File) -> String {
    file.basename()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| file.parse_name().to_string())
}

async fn ask(parent: Option<gtk::Window>, conflict: Conflict) -> Answer {
    let name = name_of(&conflict.dest);
    let folder = conflict
        .dest
        .parent()
        .map(|d| name_of(&d))
        .unwrap_or_default();
    let (heading, detail, replace) = if conflict.both_dirs {
        (
            format!("Merge folder “{name}”?"),
            format!(
                "A folder with the same name already exists in “{folder}”. \
                 Merging asks before replacing any files in it."
            ),
            "Merge",
        )
    } else {
        (
            format!("Replace “{name}”?"),
            format!("An item with the same name already exists in “{folder}”."),
            "Replace",
        )
    };

    let window = dialog::window_for(parent.as_ref(), "Name Conflict", "conflict");
    let label = |text: &str, class: &str| {
        gtk::Label::builder()
            .label(text)
            .xalign(0.0)
            .wrap(true)
            .max_width_chars(48)
            .css_classes([class])
            .build()
    };
    let entry = gtk::Entry::builder().text(&conflict.free_name).build();
    let all = gtk::CheckButton::with_label("Apply to all");
    let buttons = gtk::Box::builder()
        .spacing(8)
        .halign(gtk::Align::End)
        .build();
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(18)
        .margin_bottom(18)
        .margin_start(18)
        .margin_end(18)
        .build();
    content.append(&label(&heading, "heading"));
    content.append(&label(&detail, "detail"));
    content.append(&entry);
    content.append(&all);
    content.append(&buttons);
    window.set_child(Some(&content));

    let reply = Rc::new(Reply::default());
    let answer = glib::clone!(
        #[strong]
        reply,
        #[weak]
        window,
        #[weak]
        all,
        #[weak]
        entry,
        move |choice: Choice| {
            let name = entry.text().trim().to_owned();
            if choice == Choice::Rename && (name.is_empty() || name.contains('/')) {
                entry.add_css_class("error");
                return;
            }
            reply.send(Answer {
                choice,
                name,
                all: all.is_active(),
            });
            window.close();
        }
    );
    let mut default = None;
    for (text, choice) in [
        ("Cancel", Choice::Cancel),
        ("Skip", Choice::Skip),
        ("Rename", Choice::Rename),
        (replace, Choice::Replace),
    ] {
        let button = gtk::Button::with_label(text);
        let answer = answer.clone();
        button.connect_clicked(move |_| answer(choice));
        buttons.append(&button);
        default = Some(button);
    }
    entry.connect_activate(glib::clone!(
        #[strong]
        answer,
        move |_| answer(Choice::Rename)
    ));
    // Escape and the close button cancel.
    window.connect_close_request(glib::clone!(
        #[strong]
        reply,
        move |_| {
            reply.send(Answer {
                choice: Choice::Cancel,
                name: String::new(),
                all: false,
            });
            glib::Propagation::Proceed
        }
    ));
    let default = default.expect("four buttons");
    default.add_css_class("suggested-action");
    window.set_default_widget(Some(&default));
    window.present();
    default.grab_focus();

    std::future::poll_fn(|cx| match reply.value.take() {
        Some(answer) => Poll::Ready(answer),
        None => {
            *reply.waker.borrow_mut() = Some(cx.waker().clone());
            Poll::Pending
        }
    })
    .await
}
