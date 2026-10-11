use std::sync::Arc;

use crate::FontType;
use crate::result::FontOutputs;

pub(crate) enum OutputContents {
    Bytes(Arc<Vec<u8>>),
    Text(Arc<String>),
}

impl OutputContents {
    pub(crate) fn as_bytes(&self) -> &[u8] {
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

pub(crate) enum OutputKind {
    Font(FontType),
    Css,
    Html,
}

pub(crate) struct CollectedOutput {
    pub(crate) kind: OutputKind,
    pub(crate) contents: OutputContents,
}

/// Collect already-produced contents without assigning destinations or copying payloads.
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
