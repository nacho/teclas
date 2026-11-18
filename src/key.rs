/// Size (in px) of a regular key; also the unit of `KeyKind::width_units`.
pub const KEY_SIZE: f32 = 50.0;

/// Gap (in px) between neighbouring keys and between rows.
pub const KEY_SPACING: f32 = 5.0;

/// What a key on the on-screen keyboard is. The kind decides the key's size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyKind<'a> {
    /// A key that types a character.
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
}
