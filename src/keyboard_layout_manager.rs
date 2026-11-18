use crate::key::{KEY_SIZE, KEY_SPACING, Key, KeyEntry, key_height};
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

/// What the flow needs to know about a key. `KeyEntry`s convert into it;
/// at runtime the sizes come from the `Key` widgets themselves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeySpec {
    pub row: usize,
    pub width: f32,
    pub row_span: usize,
    pub fill: bool,
}

impl From<&KeyEntry> for KeySpec {
    fn from(entry: &KeyEntry) -> Self {
        Self {
            row: entry.row,
            width: entry.kind.width(),
            row_span: entry.row_span,
            fill: entry.fill,
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

/// Where each key goes and how big the whole keyboard is.
#[derive(Debug, Clone, PartialEq)]
pub struct Flow {
    /// One rectangle per input key, in input order.
    pub rects: Vec<Rect>,
    pub width: f32,
    pub height: f32,
}

/// Place keys row by row, left to right, in the order given.
///
/// * A key covering several rows is placed after the last key of every row
///   it covers, so it never overlaps the keys of the lower rows.
/// * A `fill` key takes whatever is left up to the right edge of the
///   keyboard, which is as wide as the widest row.
pub fn flow(keys: &[KeySpec]) -> Flow {
    let rows = keys
        .iter()
        .map(|key| key.row + key.row_span.max(1))
        .max()
        .unwrap_or(0);
    let row_y = |row: usize| row as f32 * (KEY_SIZE + KEY_SPACING);

    let mut rects = vec![Rect::default(); keys.len()];
    // Right edge of the last key placed in each row.
    let mut row_end: Vec<Option<f32>> = vec![None; rows];

    for (rect, key) in rects.iter_mut().zip(keys).filter(|(_, k)| k.row_span <= 1) {
        let x = row_end[key.row].map_or(0.0, |end| end + KEY_SPACING);
        *rect = Rect {
            x,
            y: row_y(key.row),
            width: key.width,
            height: key_height(1),
        };
        row_end[key.row] = Some(x + key.width);
    }

    for (rect, key) in rects.iter_mut().zip(keys).filter(|(_, k)| k.row_span > 1) {
        let covered = key.row..key.row + key.row_span;
        let x = covered
            .clone()
            .filter_map(|row| row_end[row])
            .reduce(f32::max)
            .map_or(0.0, |end| end + KEY_SPACING);
        *rect = Rect {
            x,
            y: row_y(key.row),
            width: key.width,
            height: key_height(key.row_span),
        };
        for row in covered {
            row_end[row] = Some(x + key.width);
        }
    }

    let width = row_end.iter().flatten().copied().fold(0.0, f32::max);

    for (rect, key) in rects.iter_mut().zip(keys).filter(|(_, k)| k.fill) {
        rect.width = (width - rect.x).max(key.width);
    }

    Flow {
        rects,
        width,
        height: if rows == 0 { 0.0 } else { key_height(rows) },
    }
}

fn keys_of(widget: &gtk::Widget) -> Vec<Key> {
    let mut keys = Vec::new();
    let mut child = widget.first_child();
    while let Some(widget) = child {
        child = widget.next_sibling();
        if let Ok(key) = widget.downcast::<Key>() {
            keys.push(key);
        }
    }
    keys
}

fn flow_of(keys: &[Key]) -> Flow {
    let specs: Vec<KeySpec> = keys.iter().map(|key| (&key.entry()).into()).collect();
    flow(&specs)
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct KeyboardLayoutManager;

    #[glib::object_subclass]
    impl ObjectSubclass for KeyboardLayoutManager {
        const NAME: &'static str = "TeclasKeyboardLayoutManager";
        type Type = super::KeyboardLayoutManager;
        type ParentType = gtk::LayoutManager;
    }

    impl ObjectImpl for KeyboardLayoutManager {}

    impl LayoutManagerImpl for KeyboardLayoutManager {
        fn measure(
            &self,
            widget: &gtk::Widget,
            orientation: gtk::Orientation,
            _for_size: i32,
        ) -> (i32, i32, i32, i32) {
            let flow = flow_of(&keys_of(widget));
            let size = match orientation {
                gtk::Orientation::Horizontal => flow.width,
                _ => flow.height,
            }
            .ceil() as i32;
            (size, size, -1, -1)
        }

        fn allocate(&self, widget: &gtk::Widget, _width: i32, _height: i32, _baseline: i32) {
            let keys = keys_of(widget);
            let flow = flow_of(&keys);

            for (key, rect) in keys.iter().zip(&flow.rects) {
                // GTK requires every child to be measured before it is
                // allocated. The flow already knows the sizes from the key
                // kinds, so the result is not needed.
                key.preferred_size();

                // Round both edges so neighbouring keys keep the same gap.
                let left = rect.x.round() as i32;
                let top = rect.y.round() as i32;
                let right = (rect.x + rect.width).round() as i32;
                let bottom = (rect.y + rect.height).round() as i32;
                key.size_allocate(
                    &gtk::gdk::Rectangle::new(left, top, right - left, bottom - top),
                    -1,
                );
            }
        }
    }
}

glib::wrapper! {
    /// Lays out `Key` children like a keyboard, see `flow`.
    pub struct KeyboardLayoutManager(ObjectSubclass<imp::KeyboardLayoutManager>)
        @extends gtk::LayoutManager;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyboard_layout::KeyboardLayout;
    use crate::language::Language;

    const ALL_LANGUAGES: [Language; 9] = [
        Language::Us,
        Language::Es,
        Language::De,
        Language::Fr,
        Language::Gl,
        Language::It,
        Language::Pl,
        Language::Pt,
        Language::PtBr,
    ];

    fn flow_for(language: Language) -> (Vec<KeyEntry>, Flow) {
        let layout = KeyboardLayout::load_from_json(language).unwrap();
        let entries = layout.key_entries();
        let specs: Vec<KeySpec> = entries.iter().map(KeySpec::from).collect();
        let flow = flow(&specs);
        (entries, flow)
    }

    fn overlaps(a: &Rect, b: &Rect) -> bool {
        a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
    }

    fn spec(row: usize, width: f32) -> KeySpec {
        KeySpec {
            row,
            width,
            row_span: 1,
            fill: false,
        }
    }

    #[test]
    fn test_flow_empty() {
        let result = flow(&[]);
        assert!(result.rects.is_empty());
        assert_eq!(result.width, 0.0);
        assert_eq!(result.height, 0.0);
    }

    #[test]
    fn test_flow_places_keys_left_to_right_with_spacing() {
        let result = flow(&[spec(0, 50.0), spec(0, 100.0), spec(1, 50.0)]);
        assert_eq!(result.rects[0].x, 0.0);
        assert_eq!(result.rects[1].x, 55.0);
        assert_eq!(result.rects[2].x, 0.0);
        assert_eq!(result.rects[2].y, 55.0);
        assert_eq!(result.width, 155.0);
        assert_eq!(result.height, 105.0);
    }

    #[test]
    fn test_flow_spanning_key_goes_after_the_widest_covered_row() {
        let tall = KeySpec {
            row: 0,
            width: 80.0,
            row_span: 2,
            fill: false,
        };
        let result = flow(&[spec(0, 50.0), tall, spec(1, 50.0), spec(1, 50.0)]);
        // Row 1 ends at 105, which is further than row 0 (50).
        assert_eq!(result.rects[1].x, 110.0);
        assert_eq!(result.rects[1].height, 105.0);
        assert_eq!(result.width, 190.0);
    }

    #[test]
    fn test_flow_fill_key_reaches_the_widest_row() {
        let fill = KeySpec {
            row: 1,
            width: 40.0,
            row_span: 1,
            fill: true,
        };
        let result = flow(&[spec(0, 200.0), spec(1, 50.0), fill]);
        assert_eq!(result.rects[2].x, 55.0);
        assert_eq!(result.rects[2].width, 145.0);
        assert_eq!(result.width, 200.0);
    }

    #[test]
    fn test_flow_fill_key_never_shrinks_below_its_width() {
        let fill = KeySpec {
            row: 1,
            width: 40.0,
            row_span: 1,
            fill: true,
        };
        let result = flow(&[spec(0, 60.0), spec(1, 50.0), fill]);
        assert_eq!(result.rects[2].width, 40.0);
        assert_eq!(result.width, 95.0);
    }

    #[test]
    fn test_us_keyboard_size() {
        let (_, result) = flow_for(Language::Us);
        assert_eq!(result.width, 815.0);
        assert_eq!(result.height, 270.0);
    }

    #[test]
    fn test_no_keys_overlap_in_any_layout() {
        for language in ALL_LANGUAGES {
            let (_, result) = flow_for(language);
            for (i, a) in result.rects.iter().enumerate() {
                for b in &result.rects[i + 1..] {
                    assert!(!overlaps(a, b), "{language:?}: {a:?} overlaps {b:?}");
                }
            }
        }
    }

    #[test]
    fn test_keys_stay_inside_the_keyboard() {
        for language in ALL_LANGUAGES {
            let (_, result) = flow_for(language);
            assert_eq!(result.height, 270.0, "{language:?}");
            for rect in &result.rects {
                assert!(rect.x >= 0.0 && rect.x + rect.width <= result.width + 0.001);
                assert!(rect.y >= 0.0 && rect.y + rect.height <= result.height + 0.001);
            }
        }
    }

    #[test]
    fn test_enter_shape_in_each_layout() {
        for language in ALL_LANGUAGES {
            let (entries, result) = flow_for(language);
            let index = entries
                .iter()
                .position(|e| e.kind == crate::key::KeyKind::Modifier("enter"))
                .unwrap();
            let enter = result.rects[index];
            if language == Language::Us {
                // Single row: stretches to the right edge of the keyboard.
                assert_eq!(enter.height, KEY_SIZE);
                assert!((enter.x + enter.width - result.width).abs() < 0.001);
            } else {
                // Two rows tall, at the right of both rows.
                assert_eq!(enter.height, 2.0 * KEY_SIZE + KEY_SPACING, "{language:?}");
                assert_eq!(enter.width, 87.5);
            }
        }
    }
}
