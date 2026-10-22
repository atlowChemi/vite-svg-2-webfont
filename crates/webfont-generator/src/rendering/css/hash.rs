use md5::Context;
#[cfg(feature = "fs")]
use serde::Serialize;

use crate::input::LoadedSvgSource;
#[cfg(feature = "fs")]
use crate::input::ResolvedFileGenerationOptions;
#[cfg(feature = "in_memory")]
use crate::input::{ResolvedGenerationOptions, ResolvedVariants};
use crate::rendering::fingerprint::FontFingerprint;

/// Wraps md5::Context as an io::Write so serde_json can stream directly into
/// the hash without allocating an intermediate String.
struct Md5Writer<'a>(&'a mut Context);

impl std::io::Write for Md5Writer<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.consume(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "fs")]
pub(in crate::rendering) fn calc_hash(
    options: &ResolvedFileGenerationOptions,
    source_files: &[LoadedSvgSource],
) -> String {
    let mut hash = Context::new();

    for source_file in source_files {
        hash.consume(source_file.contents.as_bytes());
    }

    let hashable = FontFingerprint::from(options);
    serde_json::to_writer(Md5Writer(&mut hash), &hashable).expect("hash options should serialize");
    if let Some(variants) = &options.variants {
        let variant_hashable = HashableVariantOptions {
            default_index: variants.default_index,
            missing_glyph_behavior: match options.missing_glyphs.behavior {
                crate::types::MissingGlyphBehavior::Blank => "blank",
                crate::types::MissingGlyphBehavior::Error => "error",
                crate::types::MissingGlyphBehavior::Fallback => "fallback",
            },
            missing_glyph_variant: options.missing_glyphs.variant.as_deref(),
            variants: variants
                .variants
                .iter()
                .enumerate()
                .map(|(index, variant)| HashableVariant {
                    name: &variant.name,
                    files: &variant.source_ids,
                    weight: variant.weight,
                    default: index == variants.default_index,
                    class_name: &variant.class_name,
                    selector: &variant.selector,
                })
                .collect(),
        };
        serde_json::to_writer(Md5Writer(&mut hash), &variant_hashable)
            .expect("variant hash options should serialize");
    }

    format!("{:x}", hash.finalize())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg(feature = "fs")]
struct HashableVariantOptions<'a> {
    default_index: usize,
    missing_glyph_behavior: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    missing_glyph_variant: Option<&'a str>,
    variants: Vec<HashableVariant<'a>>,
}

/// Text-mode fingerprint: framed JSON values, ordered memberships, and no file provenance.
#[cfg(feature = "in_memory")]
pub(crate) fn calc_content_hash(
    options: &ResolvedGenerationOptions,
    variants: Option<&ResolvedVariants>,
    sources: &[LoadedSvgSource],
    css_template: Option<&str>,
    html_template: Option<&str>,
) -> String {
    let mut hash = Context::new();
    let mut settings = FontFingerprint::from(options);
    settings.css_template = css_template;
    settings.html_template = html_template;
    let mut settings = serde_json::to_value(settings).expect("settings serialize");
    settings.sort_all_objects();
    serde_json::to_writer(Md5Writer(&mut hash), &settings).expect("settings serialize");
    for source in sources {
        serde_json::to_writer(
            Md5Writer(&mut hash),
            &(&source.id, &source.glyph_name, source.contents.as_ref()),
        )
        .expect("source serializes");
    }
    if let Some(variants) = variants {
        for (index, variant) in variants.variants.iter().enumerate() {
            serde_json::to_writer(
                Md5Writer(&mut hash),
                &(
                    &variant.name,
                    variant.weight,
                    index == variants.default_index,
                    &variant.class_name,
                    &variant.source_ids,
                ),
            )
            .expect("variant serializes");
        }
    }
    let missing = match options.missing_glyphs.behavior {
        crate::MissingGlyphBehavior::Blank => "blank",
        crate::MissingGlyphBehavior::Error => "error",
        crate::MissingGlyphBehavior::Fallback => "fallback",
    };
    serde_json::to_writer(
        Md5Writer(&mut hash),
        &(missing, &options.missing_glyphs.variant),
    )
    .expect("settings serialize");
    format!("{:x}", hash.finalize())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg(feature = "fs")]
struct HashableVariant<'a> {
    name: &'a str,
    files: &'a [String],
    weight: u16,
    default: bool,
    class_name: &'a str,
    selector: &'a str,
}
