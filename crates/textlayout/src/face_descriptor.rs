//! The finite static face facts admitted by this oracle.
//!
//! These types carry already-decided values. They do not parse source syntax,
//! round numeric inputs, or describe variable-font state. The environment
//! orders their exact field values during static matching; tuple equality
//! identifies the resource or resources at the winner.

/// One exact static font weight in the inclusive range 1 through 1000.
///
/// The private representation prevents an out-of-profile weight from entering
/// either an attributed request or a declared font resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FontWeight(u16);

impl FontWeight {
    pub const MIN: Self = Self(1);
    pub const NORMAL: Self = Self(400);
    pub const MAX: Self = Self(1000);

    /// Construct a representable exact static weight.
    pub const fn new(value: u16) -> Result<Self, InvalidFontWeight> {
        if value >= Self::MIN.0 && value <= Self::MAX.0 {
            Ok(Self(value))
        } else {
            Err(InvalidFontWeight { value })
        }
    }

    pub const fn value(self) -> u16 {
        self.0
    }
}

/// A numeric weight that cannot enter the static face profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidFontWeight {
    value: u16,
}

impl InvalidFontWeight {
    pub const fn value(self) -> u16 {
        self.value
    }
}

impl std::fmt::Display for InvalidFontWeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "font weight {} is outside the supported static range 1..=1000",
            self.value
        )
    }
}

impl std::error::Error for InvalidFontWeight {}

/// One exact static stretch at an admitted named point.
///
/// Arbitrary percentages and ranges have no representation here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FontStretch {
    UltraCondensed,
    ExtraCondensed,
    Condensed,
    SemiCondensed,
    #[default]
    Normal,
    SemiExpanded,
    Expanded,
    ExtraExpanded,
    UltraExpanded,
}

/// One exact static face style admitted by this oracle.
///
/// Oblique angles and synthetic posture have no representation here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
}

/// The complete tuple used for static same-family face selection.
///
/// Every request and every declared resource supplies all three fields. There
/// is no missing field and no normalization between tuples.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StaticFaceDescriptor {
    weight: FontWeight,
    stretch: FontStretch,
    style: FontStyle,
}

impl StaticFaceDescriptor {
    pub const NORMAL: Self = Self {
        weight: FontWeight::NORMAL,
        stretch: FontStretch::Normal,
        style: FontStyle::Normal,
    };

    pub const fn new(weight: FontWeight, stretch: FontStretch, style: FontStyle) -> Self {
        Self {
            weight,
            stretch,
            style,
        }
    }

    pub const fn weight(self) -> FontWeight {
        self.weight
    }

    pub const fn stretch(self) -> FontStretch {
        self.stretch
    }

    pub const fn style(self) -> FontStyle {
        self.style
    }
}

impl Default for StaticFaceDescriptor {
    fn default() -> Self {
        Self::NORMAL
    }
}

#[cfg(test)]
mod tests {
    use super::{FontStretch, FontStyle, FontWeight, InvalidFontWeight, StaticFaceDescriptor};

    #[test]
    fn static_weight_boundaries_are_exact_and_out_of_range_values_refuse() {
        assert_eq!(FontWeight::new(1), Ok(FontWeight::MIN));
        assert_eq!(FontWeight::new(1000), Ok(FontWeight::MAX));
        assert_eq!(FontWeight::new(0), Err(InvalidFontWeight { value: 0 }));
        assert_eq!(
            FontWeight::new(1001),
            Err(InvalidFontWeight { value: 1001 })
        );
    }

    #[test]
    fn every_admitted_static_stretch_is_a_distinct_exact_tuple_member() {
        let stretches = [
            FontStretch::UltraCondensed,
            FontStretch::ExtraCondensed,
            FontStretch::Condensed,
            FontStretch::SemiCondensed,
            FontStretch::Normal,
            FontStretch::SemiExpanded,
            FontStretch::Expanded,
            FontStretch::ExtraExpanded,
            FontStretch::UltraExpanded,
        ];

        for (index, stretch) in stretches.iter().enumerate() {
            assert!(
                stretches[..index].iter().all(|earlier| earlier != stretch),
                "stretch point {stretch:?} must be distinct"
            );
            let descriptor =
                StaticFaceDescriptor::new(FontWeight::NORMAL, *stretch, FontStyle::Normal);
            assert_eq!(descriptor.stretch(), *stretch);
        }
    }

    #[test]
    fn normal_descriptor_is_complete_and_explicit() {
        assert_eq!(
            StaticFaceDescriptor::default(),
            StaticFaceDescriptor::NORMAL
        );
        assert_eq!(StaticFaceDescriptor::NORMAL.weight(), FontWeight::NORMAL);
        assert_eq!(StaticFaceDescriptor::NORMAL.stretch(), FontStretch::Normal);
        assert_eq!(StaticFaceDescriptor::NORMAL.style(), FontStyle::Normal);
    }
}
