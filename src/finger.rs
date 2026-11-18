use crate::keyboard_layout::KeyFinger;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::cell::Cell;

/// Which finger of a hand a `FingerWidget` is. The kind decides its size.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, glib::Boxed)]
#[boxed_type(name = "TeclasFingerKind")]
pub enum FingerKind {
    Pinky,
    Ring,
    Middle,
    #[default]
    Index,
    Thumb,
}

impl FingerKind {
    /// The fingers of a hand, from the outside of the hand inwards.
    pub const ALL: [FingerKind; 5] = [
        FingerKind::Pinky,
        FingerKind::Ring,
        FingerKind::Middle,
        FingerKind::Index,
        FingerKind::Thumb,
    ];

    /// Natural size (width, height) in px. The thumb is drawn lying down
    /// and then rotated by the hand.
    pub fn size(&self) -> (f32, f32) {
        match self {
            FingerKind::Pinky => (17.0, 43.0),
            FingerKind::Ring | FingerKind::Index => (17.0, 51.0),
            FingerKind::Middle => (17.0, 60.0),
            FingerKind::Thumb => (30.0, 24.0),
        }
    }
}

mod imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::FingerWidget)]
    pub struct FingerWidget {
        /// Which hand slot this widget is; decides its size.
        #[property(get, set, construct_only)]
        pub kind: Cell<FingerKind>,
        /// Which of the ten fingers this widget stands for; decides its
        /// colour class.
        #[property(get, set, construct_only)]
        pub finger: Cell<KeyFinger>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FingerWidget {
        const NAME: &'static str = "TeclasFingerWidget";
        type Type = super::FingerWidget;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("finger");
            klass.set_accessible_role(gtk::AccessibleRole::Presentation);
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FingerWidget {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.set_can_target(false);
            obj.add_css_class(&self.finger.get().css_class());
            if self.kind.get() == FingerKind::Thumb {
                obj.add_css_class("thumb");
            }
        }
    }

    impl WidgetImpl for FingerWidget {
        fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            let (width, height) = self.kind.get().size();
            let size = match orientation {
                gtk::Orientation::Horizontal => width,
                _ => height,
            }
            .ceil() as i32;
            (size, size, -1, -1)
        }
    }
}

glib::wrapper! {
    /// One finger of the hand guide. Its size depends on its kind and its
    /// look comes from the stylesheet through CSS classes: `thumb`,
    /// `current`, `modifier` and `finger-<name>`.
    pub struct FingerWidget(ObjectSubclass<imp::FingerWidget>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl FingerWidget {
    pub fn new(kind: FingerKind, finger: KeyFinger) -> Self {
        glib::Object::builder()
            .property("kind", kind)
            .property("finger", finger)
            .build()
    }

    /// Whether this finger presses the next key.
    pub fn set_current(&self, current: bool) {
        self.set_css_class("current", current);
    }

    /// Whether this finger holds the modifier (Shift, AltGr) for the next key.
    pub fn set_modifier(&self, modifier: bool) {
        self.set_css_class("modifier", modifier);
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
    fn test_finger_sizes() {
        assert_eq!(FingerKind::Pinky.size(), (17.0, 43.0));
        assert_eq!(FingerKind::Ring.size(), (17.0, 51.0));
        assert_eq!(FingerKind::Middle.size(), (17.0, 60.0));
        assert_eq!(FingerKind::Index.size(), (17.0, 51.0));
        assert_eq!(FingerKind::Thumb.size(), (30.0, 24.0));
    }

    #[test]
    fn test_middle_finger_is_the_longest() {
        let (_, middle) = FingerKind::Middle.size();
        for kind in FingerKind::ALL {
            assert!(kind.size().1 <= middle, "{kind:?}");
        }
    }
}
