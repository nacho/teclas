use crate::finger::{FingerKind, FingerWidget};
use crate::keyboard_layout::Finger;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, graphene, gsk};
use std::cell::{Cell, RefCell};

/// Size (in px) of the box a single hand is drawn in.
pub const HAND_WIDTH: f32 = 128.0;
pub const HAND_HEIGHT: f32 = 132.0;

/// Rotation (in degrees) of the left thumb; the right one is mirrored.
const LEFT_THUMB_ANGLE: f32 = -35.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Side {
    #[default]
    Left,
    Right,
}

impl Side {
    pub fn css_class(&self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
        }
    }

    /// The finger of this hand that `kind` stands for.
    pub fn finger(&self, kind: FingerKind) -> Finger {
        match (self, kind) {
            (Side::Left, FingerKind::Pinky) => Finger::LeftPinky,
            (Side::Left, FingerKind::Ring) => Finger::LeftRing,
            (Side::Left, FingerKind::Middle) => Finger::LeftMiddle,
            (Side::Left, FingerKind::Index) => Finger::LeftIndex,
            (Side::Left, FingerKind::Thumb) => Finger::LeftThumb,
            (Side::Right, FingerKind::Pinky) => Finger::RightPinky,
            (Side::Right, FingerKind::Ring) => Finger::RightRing,
            (Side::Right, FingerKind::Middle) => Finger::RightMiddle,
            (Side::Right, FingerKind::Index) => Finger::RightIndex,
            (Side::Right, FingerKind::Thumb) => Finger::RightThumb,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    /// Mirror the rectangle across the vertical centre line of the hand box.
    fn mirrored(self) -> Self {
        Self {
            x: HAND_WIDTH - self.x - self.width,
            ..self
        }
    }
}

/// Where the palm goes, in the hand's own coordinates.
pub fn palm_rect(side: Side) -> Rect {
    let left = Rect {
        x: 9.0,
        y: 64.0,
        width: 89.0,
        height: 68.0,
    };
    match side {
        Side::Left => left,
        Side::Right => left.mirrored(),
    }
}

/// Where a finger goes, before any rotation, in the hand's own coordinates.
pub fn finger_rect(side: Side, kind: FingerKind, (width, height): (f32, f32)) -> Rect {
    let (x, y) = match kind {
        FingerKind::Pinky => (9.0, 34.0),
        FingerKind::Ring => (30.0, 17.0),
        FingerKind::Middle => (51.0, 9.0),
        FingerKind::Index => (72.0, 17.0),
        FingerKind::Thumb => (92.0, 68.0),
    };
    let left = Rect {
        x,
        y,
        width,
        height,
    };
    match side {
        Side::Left => left,
        Side::Right => left.mirrored(),
    }
}

/// Rotation (in degrees, around the finger's centre) of a finger.
pub fn finger_angle(side: Side, kind: FingerKind) -> f32 {
    match (side, kind) {
        (Side::Left, FingerKind::Thumb) => LEFT_THUMB_ANGLE,
        (Side::Right, FingerKind::Thumb) => -LEFT_THUMB_ANGLE,
        _ => 0.0,
    }
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct HandLayoutManager;

    #[glib::object_subclass]
    impl ObjectSubclass for HandLayoutManager {
        const NAME: &'static str = "TeclasHandLayoutManager";
        type Type = super::HandLayoutManager;
        type ParentType = gtk::LayoutManager;
    }

    impl ObjectImpl for HandLayoutManager {}

    impl LayoutManagerImpl for HandLayoutManager {
        fn measure(
            &self,
            _widget: &gtk::Widget,
            orientation: gtk::Orientation,
            _for_size: i32,
        ) -> (i32, i32, i32, i32) {
            let size = match orientation {
                gtk::Orientation::Horizontal => HAND_WIDTH,
                _ => HAND_HEIGHT,
            }
            .ceil() as i32;
            (size, size, -1, -1)
        }

        fn allocate(&self, widget: &gtk::Widget, _width: i32, _height: i32, _baseline: i32) {
            let side = widget
                .downcast_ref::<super::Hand>()
                .map_or(Side::Left, |hand| hand.side());

            let mut child = widget.first_child();
            while let Some(part) = child {
                child = part.next_sibling();
                if !part.should_layout() {
                    continue;
                }

                // Fingers decide their own size; the palm has a fixed place.
                let (_, natural) = part.preferred_size();
                let size = (natural.width() as f32, natural.height() as f32);

                let (rect, angle) = match part.downcast_ref::<FingerWidget>() {
                    Some(finger) => (
                        finger_rect(side, finger.kind(), size),
                        finger_angle(side, finger.kind()),
                    ),
                    None => (palm_rect(side), 0.0),
                };

                // Rotate around the centre of the part.
                let transform = gsk::Transform::new()
                    .translate(&graphene::Point::new(
                        rect.x + rect.width / 2.0,
                        rect.y + rect.height / 2.0,
                    ))
                    .rotate(angle)
                    .translate(&graphene::Point::new(-rect.width / 2.0, -rect.height / 2.0));
                part.allocate(
                    rect.width.round() as i32,
                    rect.height.round() as i32,
                    -1,
                    Some(transform),
                );
            }
        }
    }

    #[derive(Default)]
    pub struct Hand {
        pub side: Cell<Side>,
        pub fingers: RefCell<Vec<FingerWidget>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Hand {
        const NAME: &'static str = "TeclasHand";
        type Type = super::Hand;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("hand");
            klass.set_layout_manager_type::<super::HandLayoutManager>();
            klass.set_accessible_role(gtk::AccessibleRole::Presentation);
        }
    }

    impl ObjectImpl for Hand {
        fn dispose(&self) {
            self.fingers.take();
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for Hand {}
}

glib::wrapper! {
    /// Places the palm and fingers of a `Hand`, mirrored for the right hand.
    pub struct HandLayoutManager(ObjectSubclass<imp::HandLayoutManager>)
        @extends gtk::LayoutManager;
}

glib::wrapper! {
    /// One hand of the hand guide (CSS node `hand`, class `left`/`right`):
    /// a `palm` and five `FingerWidget`s.
    pub struct Hand(ObjectSubclass<imp::Hand>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Hand {
    pub fn new(side: Side) -> Self {
        let hand: Self = glib::Object::new();
        hand.imp().side.set(side);
        hand.add_css_class(side.css_class());
        hand.set_can_target(false);

        // The palm goes first so the fingers are drawn on top of it.
        let palm = gtk::Box::builder()
            .css_name("palm")
            .accessible_role(gtk::AccessibleRole::Presentation)
            .can_target(false)
            .build();
        palm.set_parent(&hand);

        let fingers: Vec<FingerWidget> = FingerKind::ALL
            .iter()
            .map(|&kind| {
                let finger = FingerWidget::new(kind, side.finger(kind));
                finger.set_parent(&hand);
                finger
            })
            .collect();
        hand.imp().fingers.replace(fingers);

        hand
    }

    pub fn side(&self) -> Side {
        self.imp().side.get()
    }

    pub fn fingers(&self) -> Vec<FingerWidget> {
        self.imp().fingers.borrow().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIDES: [Side; 2] = [Side::Left, Side::Right];

    /// Axis-aligned bounding box of `rect` rotated by `angle` degrees
    /// around its centre.
    fn rotated_bounds(rect: Rect, angle: f32) -> Rect {
        let (sin, cos) = angle.to_radians().sin_cos();
        let width = rect.width * cos.abs() + rect.height * sin.abs();
        let height = rect.width * sin.abs() + rect.height * cos.abs();
        Rect {
            x: rect.x + (rect.width - width) / 2.0,
            y: rect.y + (rect.height - height) / 2.0,
            width,
            height,
        }
    }

    fn finger_bounds(side: Side, kind: FingerKind) -> Rect {
        rotated_bounds(
            finger_rect(side, kind, kind.size()),
            finger_angle(side, kind),
        )
    }

    fn inside_hand(rect: Rect) -> bool {
        rect.x >= 0.0
            && rect.y >= 0.0
            && rect.x + rect.width <= HAND_WIDTH
            && rect.y + rect.height <= HAND_HEIGHT
    }

    #[test]
    fn test_every_part_fits_in_the_hand() {
        for side in SIDES {
            assert!(inside_hand(palm_rect(side)), "{side:?} palm");
            for kind in FingerKind::ALL {
                let bounds = finger_bounds(side, kind);
                assert!(inside_hand(bounds), "{side:?} {kind:?}: {bounds:?}");
            }
        }
    }

    #[test]
    fn test_right_hand_mirrors_the_left_hand() {
        let mirror_x = |rect: Rect| HAND_WIDTH - rect.x - rect.width;

        let (left, right) = (palm_rect(Side::Left), palm_rect(Side::Right));
        assert_eq!(right.x, mirror_x(left));
        assert_eq!(
            (right.y, right.width, right.height),
            (left.y, left.width, left.height)
        );

        for kind in FingerKind::ALL {
            let left = finger_rect(Side::Left, kind, kind.size());
            let right = finger_rect(Side::Right, kind, kind.size());
            assert_eq!(right.x, mirror_x(left), "{kind:?}");
            assert_eq!(right.y, left.y, "{kind:?}");
            assert_eq!(
                finger_angle(Side::Right, kind),
                -finger_angle(Side::Left, kind),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn test_only_thumbs_are_rotated() {
        for side in SIDES {
            for kind in FingerKind::ALL {
                let angle = finger_angle(side, kind);
                assert_eq!(angle != 0.0, kind == FingerKind::Thumb, "{side:?} {kind:?}");
            }
        }
        assert!(finger_angle(Side::Left, FingerKind::Thumb) < 0.0);
    }

    #[test]
    fn test_fingers_do_not_overlap() {
        let overlaps = |a: Rect, b: Rect| {
            a.x < b.x + b.width
                && b.x < a.x + a.width
                && a.y < b.y + b.height
                && b.y < a.y + a.height
        };
        for side in SIDES {
            // The thumb only lies across the palm, so compare the other
            // fingers with each other.
            let fingers: Vec<Rect> = FingerKind::ALL
                .iter()
                .filter(|kind| **kind != FingerKind::Thumb)
                .map(|&kind| finger_bounds(side, kind))
                .collect();
            for (i, a) in fingers.iter().enumerate() {
                for b in &fingers[i + 1..] {
                    assert!(!overlaps(*a, *b), "{side:?}: {a:?} overlaps {b:?}");
                }
            }
        }
    }

    #[test]
    fn test_thumbs_point_inwards() {
        let left = finger_rect(Side::Left, FingerKind::Thumb, FingerKind::Thumb.size());
        let right = finger_rect(Side::Right, FingerKind::Thumb, FingerKind::Thumb.size());
        let left_palm = palm_rect(Side::Left);
        let right_palm = palm_rect(Side::Right);
        // The thumb sits on the side of the palm that faces the other hand.
        assert!(left.x + left.width > left_palm.x + left_palm.width);
        assert!(right.x < right_palm.x);
    }

    #[test]
    fn test_side_fingers() {
        assert_eq!(Side::Left.finger(FingerKind::Pinky), Finger::LeftPinky);
        assert_eq!(Side::Right.finger(FingerKind::Thumb), Finger::RightThumb);
        for kind in FingerKind::ALL {
            assert_eq!(Side::Left.finger(kind).hand(), Some("left"));
            assert_eq!(Side::Right.finger(kind).hand(), Some("right"));
        }
    }
}
