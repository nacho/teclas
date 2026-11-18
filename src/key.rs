use crate::keyboard_layout::{Finger, KeyInfo};
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

/// Size (in px) of a regular key; also the unit of `KeyKind::width_units`.
pub const KEY_SIZE: f32 = 50.0;

/// Gap (in px) between neighbouring keys and between rows.
pub const KEY_SPACING: f32 = 5.0;

/// What a key on the on-screen keyboard is. The kind decides the key's size.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum KeyKind<'a> {
    /// A key that types a character.
    #[default]
    Character,
    /// A modifier or control key, by its name in the layout JSON
    /// (`tab`, `shift_left`, `enter`, ...).
    Modifier(&'a str),
    /// The space bar.
    Space,
}

impl KeyKind<'_> {
    /// Natural width in key units (one unit is `KEY_SIZE` px).
    pub fn width_units(&self) -> f32 {
        match self {
            KeyKind::Character => 1.0,
            KeyKind::Space => 6.0,
            KeyKind::Modifier(name) => match *name {
                "backspace" => 2.0,
                "tab" | "ctrl_left" | "ctrl_right" => 1.5,
                "caps_lock" | "enter" => 1.75,
                "shift_left" => 1.25,
                "shift_right" => 3.0,
                "super_left" | "super_right" | "menu" => 1.2,
                "alt_left" | "alt_right" => 1.3,
                _ => 1.0,
            },
        }
    }

    /// Natural width in px.
    pub fn width(&self) -> f32 {
        self.width_units() * KEY_SIZE
    }
}

/// Height in px of a key spanning `rows` keyboard rows.
pub fn key_height(rows: usize) -> f32 {
    let rows = rows.max(1);
    rows as f32 * KEY_SIZE + (rows - 1) as f32 * KEY_SPACING
}

/// A key of the on-screen keyboard and where it goes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyEntry {
    pub kind: KeyKind<'static>,
    /// Keyboard row the key starts in.
    pub row: usize,
    /// Number of keyboard rows the key covers (`0` is treated as `1`).
    pub row_span: usize,
    /// The key stretches to the right edge of the keyboard.
    pub fill: bool,
    /// For character keys, the column in `KeyboardLayout::keys[row]`.
    pub col: Option<usize>,
}

impl KeyEntry {
    pub fn character(row: usize, col: usize) -> Self {
        Self {
            kind: KeyKind::Character,
            row,
            row_span: 1,
            fill: false,
            col: Some(col),
        }
    }

    pub fn modifier(name: &'static str, row: usize) -> Self {
        Self {
            kind: KeyKind::Modifier(name),
            row,
            row_span: 1,
            fill: false,
            col: None,
        }
    }

    pub fn space(row: usize) -> Self {
        Self {
            kind: KeyKind::Space,
            row,
            row_span: 1,
            fill: false,
            col: None,
        }
    }

    pub fn spanning(mut self, rows: usize) -> Self {
        self.row_span = rows;
        self
    }

    pub fn filling(mut self) -> Self {
        self.fill = true;
        self
    }
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Key {
        pub entry: std::cell::Cell<KeyEntry>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Key {
        const NAME: &'static str = "TeclasKey";
        type Type = super::Key;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("key");
            klass.set_accessible_role(gtk::AccessibleRole::Presentation);
        }
    }

    impl ObjectImpl for Key {
        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for Key {
        fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            let entry = self.entry.get();
            let size = match orientation {
                gtk::Orientation::Horizontal => entry.kind.width(),
                _ => key_height(entry.row_span),
            }
            .ceil() as i32;
            (size, size, -1, -1)
        }

        fn size_allocate(&self, width: i32, height: i32, _baseline: i32) {
            // Every label gets the whole key and positions itself with
            // halign/valign and the margins from the stylesheet.
            let mut child = self.obj().first_child();
            while let Some(label) = child {
                label.allocate(width, height, -1, None);
                child = label.next_sibling();
            }
        }
    }
}

glib::wrapper! {
    /// A single key of the on-screen keyboard. Its size depends on its kind
    /// and its look comes from the stylesheet through CSS classes:
    /// `modifier`, `current`, `dimmed` and `finger-<name>`.
    pub struct Key(ObjectSubclass<imp::Key>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Key {
    fn from_entry(entry: KeyEntry) -> Self {
        let key: Self = glib::Object::new();
        key.imp().entry.set(entry);
        key.set_can_target(false);
        key
    }

    /// A key that types a character, with its finger colour class.
    pub fn character(entry: KeyEntry, info: &KeyInfo) -> Self {
        let key = Self::from_entry(entry);
        key.add_css_class(&info.finger.css_class());

        if info.base.chars().next().is_some_and(char::is_alphabetic) {
            key.add_label(
                &info.base.to_uppercase(),
                "letter",
                gtk::Align::Center,
                gtk::Align::Center,
            );
        } else {
            key.add_label(&info.base, "symbol", gtk::Align::Start, gtk::Align::End);
            if let Some(shift) = &info.shift {
                key.add_label(shift, "symbol", gtk::Align::Start, gtk::Align::Start);
            }
            if let Some(altgr) = info.altgr.as_deref().filter(|s| !s.is_empty()) {
                key.add_label(altgr, "symbol", gtk::Align::End, gtk::Align::End);
            }
        }
        key
    }

    /// A modifier or control key with a text label.
    pub fn modifier(entry: KeyEntry, label: &str) -> Self {
        let key = Self::from_entry(entry);
        key.add_css_class("modifier");
        key.add_label(label, "caption", gtk::Align::Center, gtk::Align::Center);
        key
    }

    /// The space bar, coloured by the thumb that presses it.
    pub fn space(entry: KeyEntry, label: &str, finger: Finger) -> Self {
        let key = Self::from_entry(entry);
        key.add_css_class(&finger.css_class());
        key.add_label(label, "caption", gtk::Align::Center, gtk::Align::Center);
        key
    }

    fn add_label(&self, text: &str, css_class: &str, halign: gtk::Align, valign: gtk::Align) {
        let label = gtk::Label::builder()
            .label(text)
            .css_classes([css_class])
            .halign(halign)
            .valign(valign)
            .build();
        // Long modifier names (e.g. "Retour arrière") must not overflow the key.
        if css_class == "caption" {
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        }
        label.set_parent(self);
    }

    pub fn entry(&self) -> KeyEntry {
        self.imp().entry.get()
    }

    /// Whether this key is the one (or one of the ones) to press next.
    pub fn set_current(&self, current: bool) {
        self.set_css_class("current", current);
    }

    /// Hide the key's text, e.g. for keys the lesson does not use.
    pub fn set_dimmed(&self, dimmed: bool) {
        self.set_css_class("dimmed", dimmed);
    }

    fn set_css_class(&self, name: &str, enabled: bool) {
        if enabled {
            self.add_css_class(name);
        } else {
            self.remove_css_class(name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_character_and_space_widths() {
        assert_eq!(KeyKind::Character.width_units(), 1.0);
        assert_eq!(KeyKind::Space.width_units(), 6.0);
        assert_eq!(KeyKind::Character.width(), KEY_SIZE);
    }

    #[test]
    fn test_modifier_widths() {
        let expected = [
            ("backspace", 2.0),
            ("tab", 1.5),
            ("caps_lock", 1.75),
            ("enter", 1.75),
            ("shift_left", 1.25),
            ("shift_right", 3.0),
            ("ctrl_left", 1.5),
            ("ctrl_right", 1.5),
            ("super_left", 1.2),
            ("super_right", 1.2),
            ("menu", 1.2),
            ("alt_left", 1.3),
            ("alt_right", 1.3),
        ];
        for (name, units) in expected {
            assert_eq!(KeyKind::Modifier(name).width_units(), units, "{name}");
        }
    }

    #[test]
    fn test_unknown_modifier_is_one_unit() {
        assert_eq!(KeyKind::Modifier("fn").width_units(), 1.0);
    }

    #[test]
    fn test_key_height() {
        assert_eq!(key_height(0), KEY_SIZE);
        assert_eq!(key_height(1), KEY_SIZE);
        assert_eq!(key_height(2), 2.0 * KEY_SIZE + KEY_SPACING);
    }
}
