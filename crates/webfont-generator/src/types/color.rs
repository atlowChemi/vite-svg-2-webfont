use std::collections::HashSet;
use std::sync::Arc;

use kurbo::BezPath;

/// Internal selection of final logical names; public option resolution lands later.
#[derive(Clone, Default)]
#[allow(
    dead_code,
    reason = "internal color selection is exposed in the public API stack layer"
)]
pub(crate) enum ColorSelection {
    All,
    #[default]
    None,
    Named(HashSet<String>),
}

impl ColorSelection {
    pub fn contains(&self, name: &str) -> bool {
        match self {
            Self::All => true,
            Self::None => false,
            Self::Named(names) => names.contains(name),
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
