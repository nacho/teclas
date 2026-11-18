use crate::finger::FingerWidget;
use crate::hand::{Hand, Side};
use crate::keyboard_layout::Finger;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::cell::{Cell, RefCell};

/// The fingers to mark for the given current and modifier fingers, as
/// `(current, modifier)`. `BothThumbs` stands for both thumbs, and a finger
/// that is current is never also marked as the modifier finger.
pub fn finger_states(
    current: Option<Finger>,
    modifier: Option<Finger>,
) -> (Vec<Finger>, Vec<Finger>) {
    let expand = |finger: Option<Finger>| match finger {
        Some(Finger::BothThumbs) => vec![Finger::LeftThumb, Finger::RightThumb],
        Some(finger) => vec![finger],
        None => Vec::new(),
    };
    let current = expand(current);
    let modifier = expand(modifier)
        .into_iter()
        .filter(|finger| !current.contains(finger))
        .collect();
    (current, modifier)
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct HandWidget {
        pub current_finger: Cell<Option<Finger>>,
        pub modifier_finger: Cell<Option<Finger>>,
        pub hands: RefCell<Vec<Hand>>,
        pub settings: RefCell<Option<gtk::gio::Settings>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HandWidget {
        const NAME: &'static str = "TeclasHandWidget";
        type Type = super::HandWidget;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("hands");
            klass.set_layout_manager_type::<gtk::BoxLayout>();
            klass.set_accessible_role(gtk::AccessibleRole::Presentation);
        }
    }

    impl ObjectImpl for HandWidget {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            let hands: Vec<Hand> = [Side::Left, Side::Right]
                .into_iter()
                .map(|side| {
                    let hand = Hand::new(side);
                    hand.set_parent(&*obj);
                    hand
                })
                .collect();
            self.hands.replace(hands);

            // Colour the fingers by finger when the preference is on.
            let settings = gtk::gio::Settings::new(crate::config::APPLICATION_ID);
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
            for hand in self.hands.take() {
                hand.unparent();
            }
        }
    }

    impl WidgetImpl for HandWidget {}

    impl HandWidget {
        fn sync_finger_colors(&self, settings: &gtk::gio::Settings) {
            let obj = self.obj();
            if settings.boolean("use-finger-colors") {
                obj.add_css_class("finger-colors");
            } else {
                obj.remove_css_class("finger-colors");
            }
        }

        fn finger_widgets(&self) -> Vec<FingerWidget> {
            self.hands.borrow().iter().flat_map(Hand::fingers).collect()
        }

        /// Mark the finger pressing the next key and the one holding its modifier.
        pub fn update_finger_states(&self) {
            let (current, modifier) =
                finger_states(self.current_finger.get(), self.modifier_finger.get());
            for widget in self.finger_widgets() {
                let finger = widget.finger();
                widget.set_current(finger.is_some_and(|f| current.contains(&f)));
                widget.set_modifier(finger.is_some_and(|f| modifier.contains(&f)));
            }
        }
    }
}

glib::wrapper! {
    /// The hand guide: a left and a right `Hand` side by side.
    pub struct HandWidget(ObjectSubclass<imp::HandWidget>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl HandWidget {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_current_finger(&self, finger: Option<Finger>) {
        let imp = self.imp();
        imp.current_finger.set(finger);
        imp.update_finger_states();
    }

    pub fn set_modifier_finger(&self, finger: Option<Finger>) {
        let imp = self.imp();
        imp.modifier_finger.set(finger);
        imp.update_finger_states();
    }
}

impl Default for HandWidget {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finger_states_none() {
        assert_eq!(finger_states(None, None), (vec![], vec![]));
    }

    #[test]
    fn test_finger_states_both_thumbs() {
        assert_eq!(
            finger_states(Some(Finger::BothThumbs), None),
            (vec![Finger::LeftThumb, Finger::RightThumb], vec![])
        );
    }

    #[test]
    fn test_finger_states_current_wins_over_modifier() {
        assert_eq!(
            finger_states(Some(Finger::LeftPinky), Some(Finger::LeftPinky)),
            (vec![Finger::LeftPinky], vec![])
        );
        assert_eq!(
            finger_states(Some(Finger::BothThumbs), Some(Finger::RightThumb)),
            (vec![Finger::LeftThumb, Finger::RightThumb], vec![])
        );
    }

    #[test]
    fn test_finger_states_modifier_on_the_other_hand() {
        assert_eq!(
            finger_states(Some(Finger::LeftIndex), Some(Finger::RightPinky)),
            (vec![Finger::LeftIndex], vec![Finger::RightPinky])
        );
    }

    #[test]
    fn test_finger_states_modifier_only() {
        assert_eq!(
            finger_states(None, Some(Finger::RightThumb)),
            (vec![], vec![Finger::RightThumb])
        );
    }
}
