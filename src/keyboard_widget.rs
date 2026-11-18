use crate::key::{Key, KeyKind};
use crate::keyboard_layout::{KeyFinger, KeyboardLayout, ResolvedKey};
use crate::keyboard_layout_manager::KeyboardLayoutManager;
use crate::utils::decompose_with_spacing_accent;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;

mod imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::KeyboardWidget)]
    pub struct KeyboardWidget {
        pub current_key: RefCell<Option<char>>,
        pub current_target: RefCell<Option<ResolvedKey>>,
        pub visible_keys: RefCell<Option<HashSet<char>>>,
        pub current_key_sequence: RefCell<Vec<char>>,
        pub sequence_index: RefCell<usize>,
        pub layout: RefCell<KeyboardLayout>,
        pub last_finger: RefCell<Option<KeyFinger>>,
        /// The key widgets, in the order of `KeyboardLayout::key_entries`.
        pub keys: RefCell<Vec<Key>>,
        pub settings: RefCell<Option<gio::Settings>>,
        /// Keyboard-layout language, set by the window.
        #[property(get, set = Self::set_layout_language_prop, builder(crate::language::Language::default()))]
        pub layout_language: Cell<crate::language::Language>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for KeyboardWidget {
        const NAME: &'static str = "TeclasKeyboardWidget";
        type Type = super::KeyboardWidget;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("keyboard");
            klass.set_layout_manager_type::<KeyboardLayoutManager>();
            klass.set_accessible_role(gtk::AccessibleRole::Presentation);
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for KeyboardWidget {
        fn constructed(&self) {
            self.parent_constructed();
            // Safe default; the window sets the real layout via `layout-language`.
            *self.layout.borrow_mut() = KeyboardLayout::default();
            self.rebuild_keys();

            // Colour the keys by finger when the preference is on.
            let settings = gio::Settings::new(crate::config::APPLICATION_ID);
            self.sync_finger_colors(&settings);
            settings.connect_changed(
                Some("use-finger-colors"),
                glib::clone!(
                    #[weak(rename_to = this)]
                    self,
                    move |settings, _| {
                        this.sync_finger_colors(settings);
                    }
                ),
            );
            // Keep the settings object alive so the signal stays connected.
            self.settings.replace(Some(settings));
        }

        fn dispose(&self) {
            for key in self.keys.take() {
                key.unparent();
            }
        }
    }

    impl WidgetImpl for KeyboardWidget {}

    impl KeyboardWidget {
        /// `layout-language` setter: load the matching layout JSON directly.
        fn set_layout_language_prop(&self, language: crate::language::Language) {
            let layout = KeyboardLayout::load_from_json(language).unwrap_or_default();
            let current = *self.current_key.borrow();
            *self.current_target.borrow_mut() = current.and_then(|c| layout.resolve_char(c));
            *self.layout.borrow_mut() = layout;
            self.layout_language.set(language);
            self.rebuild_keys();
        }

        fn sync_finger_colors(&self, settings: &gio::Settings) {
            let obj = self.obj();
            if settings.boolean("use-finger-colors") {
                obj.add_css_class("finger-colors");
            } else {
                obj.remove_css_class("finger-colors");
            }
        }

        /// Replace the key widgets with the ones for the current layout.
        fn rebuild_keys(&self) {
            let obj = self.obj();
            for key in self.keys.take() {
                key.unparent();
            }

            let layout = self.layout.borrow();
            let mut keys = Vec::new();
            for entry in layout.key_entries() {
                let key = match entry.kind {
                    KeyKind::Character => {
                        let Some(info) = entry
                            .col
                            .and_then(|col| layout.keys.get(entry.row)?.get(col))
                        else {
                            continue;
                        };
                        Key::character(entry, info)
                    }
                    KeyKind::Modifier(name) => {
                        let Some(modifier) = layout.modifiers.get(name) else {
                            continue;
                        };
                        Key::modifier(entry, &modifier.label)
                    }
                    KeyKind::Space => Key::space(
                        entry,
                        layout.space.label.as_deref().unwrap_or("SPACE"),
                        layout.space.finger,
                    ),
                };
                key.set_parent(&*obj);
                keys.push(key);
            }
            drop(layout);
            *self.keys.borrow_mut() = keys;

            self.update_key_states();
        }

        /// Mark the key(s) to press next and dim the keys the lesson does not use.
        pub fn update_key_states(&self) {
            let layout = self.layout.borrow();
            let visible = self.visible_keys.borrow();
            let current = *self.current_key.borrow();
            let target = *self.current_target.borrow();
            let required_modifiers = target
                .map(|t| layout.modifier_keys_for(&t))
                .unwrap_or_default();

            for key in self.keys.borrow().iter() {
                let entry = key.entry();
                match entry.kind {
                    KeyKind::Character => {
                        let Some(col) = entry.col else { continue };
                        key.set_current(target.is_some_and(|t| t.row == entry.row && t.col == col));
                        if let Some(info) = layout.keys.get(entry.row).and_then(|r| r.get(col)) {
                            key.set_dimmed(!layout.is_key_visible(info, visible.as_ref()));
                        }
                    }
                    KeyKind::Modifier(name) => {
                        key.set_current(required_modifiers.contains(&name));
                    }
                    KeyKind::Space => {
                        key.set_current(current == Some(' '));
                        key.set_dimmed(visible.as_ref().is_some_and(|v| !v.contains(&' ')));
                    }
                }
            }
        }
    }
}

glib::wrapper! {
    pub struct KeyboardWidget(ObjectSubclass<imp::KeyboardWidget>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl KeyboardWidget {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_current_key(&self, key: Option<char>) {
        let imp = self.imp();
        let (sequence, current) = match key {
            Some(ch) if !imp.layout.borrow().contains_character(ch) => {
                match decompose_with_spacing_accent(ch) {
                    Some((spacing_accent, base)) => {
                        (vec![spacing_accent, base], Some(spacing_accent))
                    }
                    None => (Vec::new(), key),
                }
            }
            _ => (Vec::new(), key),
        };
        *imp.current_key_sequence.borrow_mut() = sequence;
        *imp.sequence_index.borrow_mut() = 0;
        self.set_target_char(current);

        if let Some(ch) = key
            && ch != ' '
            && let Some(finger) = imp.layout.borrow().get_finger_for_char(ch)
            && finger != KeyFinger::BothThumbs
        {
            *imp.last_finger.borrow_mut() = Some(finger);
        }
        imp.update_key_states();
    }

    pub fn advance_sequence(&self) {
        let imp = self.imp();
        let next = {
            let sequence = imp.current_key_sequence.borrow();
            let mut index = imp.sequence_index.borrow_mut();
            if sequence.is_empty() || *index >= sequence.len() - 1 {
                return;
            }
            *index += 1;
            sequence[*index]
        };
        self.set_target_char(Some(next));

        if let Some(finger) = imp.layout.borrow().get_finger_for_char(next)
            && finger != KeyFinger::BothThumbs
        {
            *imp.last_finger.borrow_mut() = Some(finger);
        }
        imp.update_key_states();
    }

    fn set_target_char(&self, ch: Option<char>) {
        let imp = self.imp();
        *imp.current_key.borrow_mut() = ch;
        *imp.current_target.borrow_mut() = ch.and_then(|c| imp.layout.borrow().resolve_char(c));
    }

    pub fn current_finger(&self) -> Option<KeyFinger> {
        let current = *self.imp().current_key.borrow();
        current.and_then(|ch| self.get_finger_for_char(ch))
    }

    pub fn current_modifier_finger(&self) -> Option<KeyFinger> {
        let imp = self.imp();
        let target = *imp.current_target.borrow();
        target.and_then(|t| imp.layout.borrow().modifier_finger_for(&t))
    }

    pub fn set_visible_keys(&self, keys: Option<HashSet<char>>) {
        let imp = self.imp();
        *imp.visible_keys.borrow_mut() = keys;
        imp.update_key_states();
    }

    pub fn get_finger_for_char(&self, ch: char) -> Option<KeyFinger> {
        let imp = self.imp();

        // Special handling for space - use opposite thumb based on last finger
        if ch == ' ' {
            let last_finger = imp.last_finger.borrow();
            return Some(
                last_finger
                    .as_ref()
                    .map_or(KeyFinger::RightThumb, |f| f.opposite_thumb()),
            );
        }

        imp.layout.borrow().get_finger_for_char(ch)
    }
}

impl Default for KeyboardWidget {
    fn default() -> Self {
        Self::new()
    }
}
