use crate::config::Config;
use gtk::{gdk, glib, prelude::*};

const ACCENTS: &[&str] = &[
    "rosewater",
    "flamingo",
    "pink",
    "mauve",
    "red",
    "maroon",
    "peach",
    "yellow",
    "green",
    "teal",
    "sky",
    "sapphire",
    "blue",
    "lavender",
];

fn palette(flavor: &str) -> &'static str {
    match flavor {
        "latte" => include_str!("../data/themes/latte.css"),
        "frappe" => include_str!("../data/themes/frappe.css"),
        "macchiato" => include_str!("../data/themes/macchiato.css"),
        _ => include_str!("../data/themes/mocha.css"),
    }
}

/// `flavor = gtk` (`LOOK-15`): the shades come from the theme's window colours, mixed
/// in Catppuccin's proportions; the accents keep mocha's values for user CSS.
fn gtk_palette() -> String {
    let mocha = include_str!("../data/themes/mocha.css");
    let accents: String = mocha
        .lines()
        .filter(|l| {
            ACCENTS
                .iter()
                .any(|a| l.starts_with(&format!("@define-color {a} ")))
        })
        .map(|l| format!("{l}\n"))
        .collect();
    // Fractions of the way from base to text, measured on mocha and latte.
    let shades: String = [
        ("surface0", 0.11),
        ("surface1", 0.22),
        ("surface2", 0.33),
        ("overlay0", 0.45),
        ("overlay1", 0.55),
        ("overlay2", 0.67),
        ("subtext0", 0.78),
        ("subtext1", 0.89),
    ]
    .iter()
    .map(|(name, f)| format!("@define-color {name} mix(@base, @text, {f});\n"))
    .collect();
    // Catppuccin's mantle and crust are 3 and 6 points of lightness below base, in
    // dark and light flavors alike.
    format!(
        "@define-color base @window_bg_color;\n\
         @define-color text @window_fg_color;\n\
         @define-color mantle hsl(from @base h s calc(l - 3));\n\
         @define-color crust hsl(from @base h s calc(l - 6));\n\
         {shades}{accents}"
    )
}

/// Defines the libadwaita names `LOOK-15` reads, for themes that only have GTK's
/// older names. The theme's own definitions take precedence over these.
const GTK_FALLBACK: &str = "\
@define-color window_bg_color @theme_bg_color;
@define-color window_fg_color @theme_fg_color;
@define-color accent_bg_color @theme_selected_bg_color;
";

/// The palette with `@accent` and `@bg`, `data/style.css` and `user_css`, in one
/// provider so user CSS can reference palette names like `@mauve`.
fn css(cfg: &Config, user_css: &str) -> String {
    let (palette, accent) = if cfg.flavor == "gtk" {
        (gtk_palette(), "accent_bg_color".to_owned())
    } else {
        let accent = if ACCENTS.contains(&cfg.accent.as_str()) {
            &cfg.accent
        } else {
            "mauve"
        };
        (palette(&cfg.flavor).to_owned(), accent.to_owned())
    };
    format!(
        "{palette}\n@define-color accent @{accent};\n@define-color bg alpha(@base, {});\n{}\n{user_css}",
        cfg.opacity,
        include_str!("../data/style.css"),
    )
}

pub fn load(cfg: &Config) {
    let user_css = glib::user_config_dir()
        .join("junk-drawer")
        .join("style.css");
    let display = gdk::Display::default().expect("no display");
    if cfg.flavor == "gtk" {
        let fallback = gtk::CssProvider::new();
        fallback.load_from_string(GTK_FALLBACK);
        gtk::style_context_add_provider_for_display(
            &display,
            &fallback,
            gtk::STYLE_PROVIDER_PRIORITY_FALLBACK,
        );
    }
    let provider = gtk::CssProvider::new();
    provider.load_from_string(&css(
        cfg,
        &if crate::screenshot::enabled() {
            String::new()
        } else {
            std::fs::read_to_string(user_css).unwrap_or_default()
        },
    ));
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_USER,
    );
}

/// Whether text should use hinted metrics: only when every monitor has scale 1, where
/// logical px are device px (`LOOK-20`).
fn hint_metrics(scales: &[f64]) -> bool {
    !scales.is_empty() && scales.iter().all(|&s| s == 1.0)
}

fn set_font_rendering(display: &gdk::Display) {
    let monitors = display.monitors();
    let scales: Vec<f64> = (0..monitors.n_items())
        .filter_map(|i| monitors.item(i).and_downcast::<gdk::Monitor>())
        .map(|m| m.scale())
        .collect();
    let settings = gtk::Settings::for_display(display);
    if hint_metrics(&scales) {
        settings.set_gtk_font_rendering(gtk::FontRendering::Manual);
        settings.set_gtk_hint_font_metrics(true);
    } else {
        settings.set_gtk_font_rendering(gtk::FontRendering::Automatic);
    }
}

/// Sets the font rendering for the display's scales now and whenever monitors change.
pub fn follow_scale(display: &gdk::Display) {
    let watch = |display: &gdk::Display, monitor: &gdk::Monitor| {
        monitor.connect_scale_notify(glib::clone!(
            #[weak]
            display,
            move |_| set_font_rendering(&display)
        ));
    };
    let monitors = display.monitors();
    for monitor in (0..monitors.n_items()).filter_map(|i| monitors.item(i).and_downcast()) {
        watch(display, &monitor);
    }
    monitors.connect_items_changed(glib::clone!(
        #[weak]
        display,
        move |monitors, position, _, added| {
            for i in position..position + added {
                if let Some(monitor) = monitors.item(i).and_downcast() {
                    watch(&display, &monitor);
                }
            }
            set_font_rendering(&display);
        }
    ));
    set_font_rendering(display);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn hints_metrics_only_when_every_monitor_is_scale_1() {
        assert!(hint_metrics(&[1.0]));
        assert!(hint_metrics(&[1.0, 1.0]));
        assert!(!hint_metrics(&[]));
        assert!(!hint_metrics(&[2.0]));
        assert!(!hint_metrics(&[1.25]));
        assert!(!hint_metrics(&[1.0, 2.0]));
    }

    fn defined(css: &str) -> HashSet<String> {
        css.lines()
            .filter_map(|l| l.trim().strip_prefix("@define-color "))
            .filter_map(|l| l.split_whitespace().next())
            .map(str::to_owned)
            .collect()
    }

    fn used(css: &str) -> HashSet<String> {
        css.split('@')
            .skip(1)
            .map(|s| {
                s.chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect::<String>()
            })
            .filter(|s| !s.is_empty())
            .collect()
    }

    #[test]
    fn every_flavor_defines_the_colours_used() {
        let mut wanted = used(include_str!("../data/style.css"));
        wanted.extend(ACCENTS.iter().map(|a| a.to_string()));
        for flavor in ["latte", "frappe", "macchiato", "mocha", "gtk"] {
            let cfg = Config {
                flavor: flavor.into(),
                ..Config::default()
            };
            let mut names = defined(&css(&cfg, ""));
            if flavor == "gtk" {
                names.extend(defined(GTK_FALLBACK));
            }
            for name in &wanted {
                assert!(names.contains(name), "{flavor}: @{name} is not defined");
            }
        }
    }

    #[test]
    fn gtk_flavor_uses_the_theme_accent() {
        let red = Config {
            accent: "red".into(),
            ..Config::default()
        };
        assert!(css(&red, "").contains("@define-color accent @red;"));
        let gtk = Config {
            flavor: "gtk".into(),
            ..red
        };
        let css = css(&gtk, "");
        assert!(css.contains("@define-color accent @accent_bg_color;"));
        assert!(css.contains("@define-color base @window_bg_color;"));
        assert!(css.contains("@define-color mauve #cba6f7;"));
    }

    fn rgb(css: &str, name: &str) -> [f64; 3] {
        let hex = css
            .lines()
            .find_map(|l| l.strip_prefix(&format!("@define-color {name} #")))
            .unwrap_or_else(|| panic!("no @{name}"));
        [0, 2, 4].map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap() as f64)
    }

    /// WCAG relative luminance.
    fn luminance(c: [f64; 3]) -> f64 {
        let [r, g, b] = c.map(|v| {
            let v = v / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        });
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    #[test]
    fn selected_text_contrasts_with_the_selection() {
        for flavor in ["latte", "frappe", "macchiato", "mocha"] {
            let css = palette(flavor);
            let (base, text) = (rgb(css, "base"), rgb(css, "text"));
            for accent in ACCENTS {
                let a = rgb(css, accent);
                let selection = [0, 1, 2].map(|i| base[i] * 0.7 + a[i] * 0.3);
                let (l1, l2) = (luminance(text), luminance(selection));
                let ratio = (l1.max(l2) + 0.05) / (l1.min(l2) + 0.05);
                assert!(ratio >= 3.0, "{flavor} {accent}: {ratio:.2}:1");
            }
        }
    }
}
