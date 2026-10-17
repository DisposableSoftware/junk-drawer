use crate::state;
use gtk::{gio, glib, prelude::*};
use std::cmp::Ordering;
use std::iter::Peekable;
use std::str::Chars;

/// Case-insensitive, with digit runs compared by value ("file2" < "file10").
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        let ord = match (x.peek(), y.peek()) {
            (None, None) => return a.cmp(b),
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(c), Some(d)) if c.is_ascii_digit() && d.is_ascii_digit() => {
                take_number(&mut x).cmp(&take_number(&mut y))
            }
            _ => {
                let (c, d) = (x.next().unwrap(), y.next().unwrap());
                c.to_lowercase().cmp(d.to_lowercase())
            }
        };
        if ord != Ordering::Equal {
            return ord;
        }
    }
}

fn take_number(it: &mut Peekable<Chars>) -> u128 {
    let mut n: u128 = 0;
    while let Some(d) = it.peek().and_then(|c| c.to_digit(10)) {
        n = n.saturating_mul(10).saturating_add(d.into());
        it.next();
    }
    n
}

fn sorter(cmp: fn(&gio::FileInfo, &gio::FileInfo) -> Ordering) -> gtk::CustomSorter {
    gtk::CustomSorter::new(move |a, b| {
        let a = a.downcast_ref::<gio::FileInfo>().unwrap();
        let b = b.downcast_ref::<gio::FileInfo>().unwrap();
        cmp(a, b).into()
    })
}

fn is_dir(info: &gio::FileInfo) -> bool {
    crate::dnd::is_dir(info)
}

pub fn dirs_first() -> gtk::CustomSorter {
    sorter(|a, b| is_dir(b).cmp(&is_dir(a)))
}

pub fn by_name() -> gtk::CustomSorter {
    sorter(|a, b| natural_cmp(&a.display_name(), &b.display_name()))
}

pub fn by_size() -> gtk::CustomSorter {
    sorter(|a, b| a.size().cmp(&b.size()))
}

pub fn by_modified() -> gtk::CustomSorter {
    sorter(|a, b| {
        let t = |i: &gio::FileInfo| i.attribute_uint64("time::modified");
        t(a).cmp(&t(b))
    })
}

pub fn by_type() -> gtk::CustomSorter {
    sorter(|a, b| a.content_type().cmp(&b.content_type()))
}

/// The `sort-by` key of a details-view column, from its title.
fn key_of(column: &gtk::ColumnViewColumn) -> Option<String> {
    column.title().map(|t| t.to_lowercase())
}

/// Adds `win.sort-by` (`name`, `size`, `modified`, `type`) and `win.sort-reversed`,
/// which drive `view`'s column sort and follow header clicks (`VIEW-13`).
pub fn attach(window: &gtk::ApplicationWindow, view: &gtk::ColumnView) {
    let Some(sorter) = view.sorter().and_downcast::<gtk::ColumnViewSorter>() else {
        return;
    };
    let column = |view: &gtk::ColumnView, key: &str| {
        let columns = view.columns();
        (0..columns.n_items())
            .filter_map(|i| columns.item(i).and_downcast::<gtk::ColumnViewColumn>())
            .find(|c| key_of(c).as_deref() == Some(key))
    };
    let order = |reversed: bool| {
        if reversed {
            gtk::SortType::Descending
        } else {
            gtk::SortType::Ascending
        }
    };

    let by = gio::SimpleAction::new_stateful(
        "sort-by",
        Some(glib::VariantTy::STRING),
        &"name".to_variant(),
    );
    let reversed = gio::SimpleAction::new_stateful("sort-reversed", None, &false.to_variant());
    by.connect_activate(glib::clone!(
        #[weak]
        view,
        #[weak]
        reversed,
        move |_, key| {
            let key = key.and_then(|k| k.get::<String>()).unwrap_or_default();
            let reversed = reversed.state().and_then(|s| s.get::<bool>()) == Some(true);
            if let Some(col) = column(&view, &key) {
                view.sort_by_column(Some(&col), order(reversed));
            }
        }
    ));
    reversed.connect_activate(glib::clone!(
        #[weak]
        view,
        #[weak]
        sorter,
        move |action, _| {
            let now = action.state().and_then(|s| s.get::<bool>()) != Some(true);
            let col = sorter.primary_sort_column();
            view.sort_by_column(col.as_ref(), order(now));
        }
    ));
    sorter.connect_changed(glib::clone!(
        #[weak]
        by,
        #[weak]
        reversed,
        move |sorter, _| {
            if let Some(key) = sorter.primary_sort_column().as_ref().and_then(key_of) {
                by.set_state(&key.to_variant());
            }
            let descending = sorter.primary_sort_order() == gtk::SortType::Descending;
            reversed.set_state(&descending.to_variant());
            if let Some(key) = sorter.primary_sort_column().as_ref().and_then(key_of)
                && (key.clone(), descending) != state::get().sort()
            {
                state::update(|s| {
                    s.sort_column = Some(key);
                    s.sort_reversed = Some(descending);
                });
            }
        }
    ));
    window.add_action(&by);
    window.add_action(&reversed);

    let (key, descending) = state::get().sort();
    if let Some(col) = column(view, &key) {
        view.sort_by_column(Some(&col), order(descending));
    }
}

#[cfg(test)]
mod tests {
    use super::natural_cmp;
    use std::cmp::Ordering::*;

    #[test]
    fn numbers_by_value() {
        assert_eq!(natural_cmp("file2", "file10"), Less);
        assert_eq!(natural_cmp("v1.10", "v1.9"), Greater);
    }

    #[test]
    fn case_insensitive_with_stable_tiebreak() {
        assert_eq!(natural_cmp("apple", "Banana"), Less);
        assert_ne!(natural_cmp("a", "A"), Equal);
    }

    #[test]
    fn prefix_sorts_first() {
        assert_eq!(natural_cmp("abc", "abcd"), Less);
        assert_eq!(natural_cmp("", "a"), Less);
    }
}
