use std::sync::Arc;

use crate::input::default_output_dest;
use crate::rendering::{render_css_with_hbs_context, render_html_with_hbs_context};
use crate::result::GenerateWebfontsResult;

pub(super) enum OutputContents {
    Bytes(Arc<Vec<u8>>),
    Text(Arc<String>),
    Owned(Vec<u8>),
}

impl OutputContents {
    pub(super) fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Bytes(bytes) => bytes.as_slice(),
            Self::Text(text) => text.as_bytes(),
            Self::Owned(bytes) => bytes.as_slice(),
        }
    }
}

impl AsRef<[u8]> for OutputContents {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

pub(super) struct OutputFile {
    pub(super) path: String,
    pub(super) contents: OutputContents,
    pub(super) skip_unchanged: bool,
}

pub(super) fn collect_write_outputs(
    result: &GenerateWebfontsResult,
) -> std::io::Result<Vec<OutputFile>> {
    let mut outputs = Vec::new();
    let font_name = result.options.font_name.clone();
    let dest = result.options.dest.clone();

    if let Some(svg_font) = &result.fonts.svg_font {
        outputs.push(OutputFile {
            path: default_output_dest(&dest, &font_name, "svg"),
            contents: OutputContents::Text(Arc::clone(svg_font)),
            skip_unchanged: false,
        });
    }
    if let Some(ttf_font) = &result.fonts.ttf_font {
        outputs.push(OutputFile {
            path: default_output_dest(&dest, &font_name, "ttf"),
            contents: OutputContents::Bytes(Arc::clone(ttf_font)),
            skip_unchanged: false,
        });
    }
    if let Some(woff_font) = &result.fonts.woff_font {
        outputs.push(OutputFile {
            path: default_output_dest(&dest, &font_name, "woff"),
            contents: OutputContents::Bytes(Arc::clone(woff_font)),
            skip_unchanged: false,
        });
    }
    if let Some(woff2_font) = &result.fonts.woff2_font {
        outputs.push(OutputFile {
            path: default_output_dest(&dest, &font_name, "woff2"),
            contents: OutputContents::Bytes(Arc::clone(woff2_font)),
            skip_unchanged: false,
        });
    }
    if let Some(eot_font) = &result.fonts.eot_font {
        outputs.push(OutputFile {
            path: default_output_dest(&dest, &font_name, "eot"),
            contents: OutputContents::Bytes(Arc::clone(eot_font)),
            skip_unchanged: false,
        });
    }

    // Only render CSS/HTML templates when those files need to be written.
    if result.options.css || result.options.html {
        let cached = result.get_cached_io()?;
        if result.options.css {
            let ctx = cached.css_hbs_context.lock().unwrap();
            let css = render_css_with_hbs_context(&cached.shared, &ctx, &cached.css_context)?;
            drop(ctx);
            outputs.push(OutputFile {
                path: result.options.css_dest.clone(),
                contents: OutputContents::Owned(css.into_bytes()),
                skip_unchanged: true,
            });
        }
        if result.options.html {
            let ctx = cached.html_hbs_context.lock().unwrap();
            let html = render_html_with_hbs_context(
                cached.html_registry.as_ref(),
                &ctx,
                &cached.html_context,
            )?;
            drop(ctx);
            outputs.push(OutputFile {
                path: result.options.html_dest.clone(),
                contents: OutputContents::Owned(html.into_bytes()),
                skip_unchanged: true,
            });
        }
    }

    Ok(outputs)
}
