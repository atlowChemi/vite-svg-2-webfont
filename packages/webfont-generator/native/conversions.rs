//! Explicit ownership transfer across the binding boundary.
use crate::types::*;

macro_rules! fields {
    ($name:ident { $($field:ident),* $(,)? }) => {
        impl From<$name> for webfont_generator::$name {
            fn from(value: $name) -> Self {
                Self { $($field: value.$field),* }
            }
        }
    };
}

fields!(VariantFileSet { variant, files });
fields!(FontVariant {
    name,
    files,
    weight,
    default
});
fields!(SvgFormatOptions {
    center_vertically,
    font_id,
    metadata,
    optimize_output,
    preserve_aspect_ratio
});
fields!(TtfFormatOptions {
    copyright,
    description,
    ts,
    url,
    version
});
fields!(WoffFormatOptions { metadata });
fields!(Woff2FormatOptions {
    compression_quality
});

impl From<FontType> for webfont_generator::FontType {
    fn from(value: FontType) -> Self {
        match value {
            FontType::Svg => Self::Svg,
            FontType::Ttf => Self::Ttf,
            FontType::Eot => Self::Eot,
            FontType::Woff => Self::Woff,
            FontType::Woff2 => Self::Woff2,
        }
    }
}

impl From<MissingGlyphOptions> for webfont_generator::MissingGlyphOptions {
    fn from(value: MissingGlyphOptions) -> Self {
        Self {
            behavior: match value.behavior {
                MissingGlyphBehavior::Blank => webfont_generator::MissingGlyphBehavior::Blank,
                MissingGlyphBehavior::Error => webfont_generator::MissingGlyphBehavior::Error,
                MissingGlyphBehavior::Fallback => webfont_generator::MissingGlyphBehavior::Fallback,
            },
            variant: value.variant,
        }
    }
}

impl From<FormatOptions> for webfont_generator::FormatOptions {
    fn from(value: FormatOptions) -> Self {
        Self {
            svg: value.svg.map(Into::into),
            ttf: value.ttf.map(Into::into),
            woff: value.woff.map(Into::into),
            woff2: value.woff2.map(Into::into),
        }
    }
}

impl From<GenerateWebfontsOptions> for webfont_generator::GenerateWebfontsOptions {
    fn from(value: GenerateWebfontsOptions) -> Self {
        Self {
            ascent: value.ascent,
            center_horizontally: value.center_horizontally,
            center_vertically: value.center_vertically,
            css: value.css,
            css_dest: value.css_dest,
            css_template: value.css_template,
            codepoints: value.codepoints,
            css_fonts_url: value.css_fonts_url,
            descent: value.descent,
            dest: value.dest,
            files: value.files,
            fixed_width: value.fixed_width,
            format_options: value.format_options.map(Into::into),
            html: value.html,
            html_dest: value.html_dest,
            html_template: value.html_template,
            incremental: value.incremental,
            font_height: value.font_height,
            font_name: value.font_name,
            font_style: value.font_style,
            font_weight: value.font_weight,
            ligature: value.ligature,
            missing_glyphs: value.missing_glyphs.map(Into::into),
            normalize: value.normalize,
            order: value
                .order
                .map(|values| values.into_iter().map(Into::into).collect()),
            optimize_output: value.optimize_output,
            preserve_aspect_ratio: value.preserve_aspect_ratio,
            round: value.round,
            start_codepoint: value.start_codepoint,
            template_options: value.template_options,
            types: value
                .types
                .map(|values| values.into_iter().map(Into::into).collect()),
            variant_class_prefix: value.variant_class_prefix,
            variants: value
                .variants
                .map(|values| values.into_iter().map(Into::into).collect()),
            write_files: value.write_files,
        }
    }
}
