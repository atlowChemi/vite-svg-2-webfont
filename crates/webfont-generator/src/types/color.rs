#[cfg(any(feature = "fs", feature = "in_memory"))]
use std::sync::Arc;

#[cfg(any(feature = "fs", feature = "in_memory"))]
use indexmap::IndexSet;
#[cfg(any(feature = "fs", feature = "in_memory"))]
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

#[cfg(any(feature = "fs", feature = "in_memory"))]
pub(crate) fn resolve_slots(
    slots: Option<std::collections::HashMap<String, String>>,
    selection: Option<&ResolvedColorSelection>,
) -> std::io::Result<std::collections::BTreeMap<String, [u8; 3]>> {
    use std::io::{Error, ErrorKind};
    let slots = slots.unwrap_or_default();
    let invalid = |message: &str| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("options.colorSlots: {message}"),
        )
    };
    if !slots.is_empty()
        && !selection.is_some_and(|selection| match selection {
            ResolvedColorSelection::All => true,
            ResolvedColorSelection::Named(names) => !names.is_empty(),
        })
    {
        return Err(invalid("requires active colorGlyphs"));
    }
    if slots.len() > usize::from(u16::MAX) {
        return Err(invalid("exceeds 65,535 palette entries"));
    }
    let mut colors = std::collections::HashSet::new();
    slots
        .into_iter()
        .map(|(name, value)| {
            if name.is_empty()
                || !name
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
            {
                return Err(invalid(
                    "names must contain only ASCII letters, digits, hyphens or underscores",
                ));
            }
            if value.len() != 7
                || !value.starts_with('#')
                || !value.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
            {
                return Err(invalid("colors must use #RRGGBB"));
            }
            let rgb =
                [1, 3, 5].map(|start| u8::from_str_radix(&value[start..start + 2], 16).unwrap());
            if !colors.insert(rgb) {
                return Err(invalid("each slot must have a distinct RGB color"));
            }
            Ok((name, rgb))
        })
        .collect()
}

// Omission uses the field default; an explicitly supplied null must not disable color.
#[cfg(any(feature = "cli", feature = "in_memory"))]
pub(crate) fn deserialize_selection<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<ColorGlyphSelection>, D::Error> {
    <ColorGlyphSelection as serde::Deserialize>::deserialize(deserializer).map(Some)
}

#[cfg(any(feature = "cli", feature = "in_memory"))]
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

#[cfg(any(feature = "fs", feature = "in_memory"))]
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
#[cfg(any(feature = "fs", feature = "in_memory"))]
pub(crate) enum ResolvedColorSelection {
    All,
    Named(IndexSet<String>),
}

#[cfg(any(feature = "fs", feature = "in_memory"))]
impl ResolvedColorSelection {
    pub fn contains(&self, name: &str) -> bool {
        match self {
            Self::All => true,
            Self::Named(names) => names.contains(name),
        }
    }
}

#[cfg(any(feature = "fs", feature = "in_memory"))]
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
#[cfg(any(feature = "fs", feature = "in_memory"))]
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
#[cfg(any(feature = "fs", feature = "in_memory"))]
pub(crate) struct ProcessedColorLayer {
    pub outline: Arc<BezPath>,
    pub outline_hash: u64,
    pub paint: ResolvedLayerPaint,
}
