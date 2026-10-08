use std::sync::Arc;

use indexmap::IndexSet;
use kurbo::BezPath;

/// Select logical glyphs whose solid SVG fills are preserved as COLR v1 paint.
/// Names refer to the final names after rename hooks, across all variants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColorGlyphSelection {
    /// Preserve paint for every logical glyph.
    All,
    /// Preserve paint for these names. An empty list disables color.
    Named(Vec<String>),
}

// Omission uses the field default; an explicitly supplied null must not disable color.
#[cfg(feature = "cli")]
pub(super) fn deserialize_selection<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<ColorGlyphSelection>, D::Error> {
    <ColorGlyphSelection as serde::Deserialize>::deserialize(deserializer).map(Some)
}

#[cfg(feature = "cli")]
impl<'de> serde::Deserialize<'de> for ColorGlyphSelection {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Input {
            All(bool),
            Named(Vec<String>),
        }
        match Input::deserialize(deserializer)? {
            Input::All(true) => Ok(Self::All),
            Input::Named(names) => Ok(Self::Named(names)),
            Input::All(false) => Err(serde::de::Error::custom(
                "expected true or an array of glyph names; omit colorGlyphs or use [] to disable color",
            )),
        }
    }
}

impl From<ColorGlyphSelection> for ResolvedColorSelection {
    fn from(selection: ColorGlyphSelection) -> Self {
        match selection {
            ColorGlyphSelection::All => Self::All,
            ColorGlyphSelection::Named(names) => Self::Named(names.into_iter().collect()),
        }
    }
}

/// Compiled membership for final logical names, preserving input order for errors.
#[derive(Clone)]
pub(crate) enum ResolvedColorSelection {
    All,
    Named(IndexSet<String>),
}

impl ResolvedColorSelection {
    pub fn contains(&self, name: &str) -> bool {
        match self {
            Self::All => true,
            Self::Named(names) => names.contains(name),
        }
    }
}

impl serde::Serialize for ResolvedColorSelection {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::All => serializer.serialize_bool(true),
            Self::Named(names) => {
                let mut names: Vec<_> = names.iter().map(String::as_str).collect();
                names.sort_unstable();
                serde::Serialize::serialize(&names, serializer)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ResolvedLayerPaint {
    Foreground {
        alpha: f32,
    },
    Solid {
        red: u8,
        green: u8,
        blue: u8,
        alpha: f32,
    },
}

#[derive(Debug, PartialEq)]
pub(crate) struct ProcessedColorLayer {
    pub outline: Arc<BezPath>,
    pub outline_hash: u64,
    pub paint: ResolvedLayerPaint,
}
