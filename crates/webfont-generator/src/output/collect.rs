use std::sync::Arc;

use crate::FontType;
#[cfg(any(feature = "fs", feature = "in_memory"))]
use crate::result::FontOutputs;

/// Shared output payload. Cloning it does not copy font bytes or rendered text.
#[derive(Clone)]
pub enum OutputContents {
    /// Binary font payload.
    Bytes(Arc<Vec<u8>>),
    /// SVG font or rendered companion text.
    Text(Arc<String>),
}

impl OutputContents {
    /// Borrow the encoded output without copying it.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Bytes(bytes) => bytes.as_slice(),
            Self::Text(text) => text.as_bytes(),
        }
    }
}

impl AsRef<[u8]> for OutputContents {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

/// Format of a destination-free generated output.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OutputKind {
    /// A generated font format.
    Font(FontType),
    /// CSS stylesheet.
    Css,
    /// HTML preview.
    Html,
}

/// A generated payload with no filesystem destination.
#[derive(Clone)]
pub struct CollectedOutput {
    /// Identifies the format.
    pub kind: OutputKind,
    /// Reference-counted output data.
    pub contents: OutputContents,
}

/// Collect already-produced contents without assigning destinations or copying payloads.
#[cfg(any(feature = "fs", feature = "in_memory"))]
pub(crate) fn collect_outputs(
    fonts: &FontOutputs,
    css: Option<Arc<String>>,
    html: Option<Arc<String>>,
) -> Vec<CollectedOutput> {
    let mut outputs = Vec::new();

    if let Some(svg_font) = &fonts.svg_font {
        outputs.push(CollectedOutput {
            kind: OutputKind::Font(FontType::Svg),
            contents: OutputContents::Text(Arc::clone(svg_font)),
        });
    }
    if let Some(ttf_font) = &fonts.ttf_font {
        outputs.push(CollectedOutput {
            kind: OutputKind::Font(FontType::Ttf),
            contents: OutputContents::Bytes(Arc::clone(ttf_font)),
        });
    }
    if let Some(woff_font) = &fonts.woff_font {
        outputs.push(CollectedOutput {
            kind: OutputKind::Font(FontType::Woff),
            contents: OutputContents::Bytes(Arc::clone(woff_font)),
        });
    }
    if let Some(woff2_font) = &fonts.woff2_font {
        outputs.push(CollectedOutput {
            kind: OutputKind::Font(FontType::Woff2),
            contents: OutputContents::Bytes(Arc::clone(woff2_font)),
        });
    }
    if let Some(eot_font) = &fonts.eot_font {
        outputs.push(CollectedOutput {
            kind: OutputKind::Font(FontType::Eot),
            contents: OutputContents::Bytes(Arc::clone(eot_font)),
        });
    }

    if let Some(css) = css {
        outputs.push(CollectedOutput {
            kind: OutputKind::Css,
            contents: OutputContents::Text(css),
        });
    }
    if let Some(html) = html {
        outputs.push(CollectedOutput {
            kind: OutputKind::Html,
            contents: OutputContents::Text(html),
        });
    }
    outputs
}
