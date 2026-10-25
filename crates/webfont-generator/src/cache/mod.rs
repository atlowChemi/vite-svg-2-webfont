//! Opt-in persistent generation cache (`cacheLocation`).
//!
//! One file per font identity holds:
//! - level 2: parsed geometry and processed glyphs keyed by SVG content hash, so unchanged icons
//!   skip parsing and path processing on a cold run;
//! - level 1: finished font bytes per format, keyed by everything that affects those bytes, so
//!   unchanged formats skip glyph work and encoding (including WOFF2 compression) entirely.
//!
//! Variant families use their own file (the design list is part of its identity), with processed
//! glyphs keyed by content hash and advance width instead of by source.
//!
//! Every read or write failure degrades to a miss: the cache can make generation faster, never
//! fail it or change its output.

mod codec;
#[cfg(all(test, feature = "fs"))]
mod tests;

use std::array::from_fn;
use std::collections::HashMap;
#[cfg(feature = "fs")]
use std::fs::{self, File};
#[cfg(feature = "fs")]
use std::io::{ErrorKind, Read, Result};
use std::mem::take;
#[cfg(feature = "fs")]
use std::ops::Deref;
#[cfg(feature = "fs")]
use std::path::{Path, PathBuf};
#[cfg(feature = "fs")]
use std::process;
use std::sync::Arc;
#[cfg(feature = "fs")]
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

use flate2::Crc;
use md5::compute;
use serde::Serialize;
use serde_json::{Value, json, to_vec};

use self::codec::{Reader, Writer};
use crate::input::{
    LoadedSvgSource, ResolvedGenerationOptions, ResolvedVariants, VariantFamilySources,
    VariantGlyphSource,
};
use crate::rendering::font_settings_json;
use crate::result::FontOutputs;
use crate::svg::types::{CachedGlyph, CachedProcessedGlyph, GlyphCache, SvgOptions};
use crate::svg::{VariantGlyphCache, source_content_hash};
use crate::types::FontType;

const MAGIC: &[u8; 8] = b"WFGCACHE";
/// Bump when the file layout or any persisted type's meaning changes.
const FORMAT_VERSION: u32 = 1;
const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
#[cfg(feature = "fs")]
const EXTENSION: &str = "wfcache";
/// Entries kept per format: the current one plus the most recent different one, so builds that
/// differ only in a format option (e.g. dev and production WOFF2 quality) don't evict each other.
const ENTRIES_PER_FORMAT: usize = 2;

const FONT_TYPES: [FontType; 5] = [
    FontType::Svg,
    FontType::Ttf,
    FontType::Eot,
    FontType::Woff,
    FontType::Woff2,
];

fn font_type_tag(font_type: FontType) -> u8 {
    FONT_TYPES
        .iter()
        .position(|candidate| *candidate == font_type)
        .expect("every font type is listed") as u8
}

/// One format's finished output, shared without copying with the result that uses it.
#[derive(Clone)]
pub(crate) enum CachedFont {
    Svg(Arc<String>),
    Ttf(Arc<Vec<u8>>),
    Eot(Arc<Vec<u8>>),
    Woff(Arc<Vec<u8>>),
    Woff2(Arc<Vec<u8>>),
}

impl CachedFont {
    pub(crate) fn font_type(&self) -> FontType {
        match self {
            Self::Svg(_) => FontType::Svg,
            Self::Ttf(_) => FontType::Ttf,
            Self::Eot(_) => FontType::Eot,
            Self::Woff(_) => FontType::Woff,
            Self::Woff2(_) => FontType::Woff2,
        }
    }

    fn bytes(&self) -> &[u8] {
        match self {
            Self::Svg(text) => text.as_bytes(),
            Self::Ttf(bytes) | Self::Eot(bytes) | Self::Woff(bytes) | Self::Woff2(bytes) => bytes,
        }
    }

    fn from_outputs(outputs: &FontOutputs, font_type: FontType) -> Option<Self> {
        Some(match font_type {
            FontType::Svg => Self::Svg(outputs.svg_font.clone()?),
            FontType::Ttf => Self::Ttf(outputs.ttf_font.clone()?),
            FontType::Eot => Self::Eot(outputs.eot_font.clone()?),
            FontType::Woff => Self::Woff(outputs.woff_font.clone()?),
            FontType::Woff2 => Self::Woff2(outputs.woff2_font.clone()?),
        })
    }

    fn decode(font_type: FontType, bytes: Vec<u8>) -> Option<Self> {
        Some(match font_type {
            FontType::Svg => Self::Svg(Arc::new(String::from_utf8(bytes).ok()?)),
            FontType::Ttf => Self::Ttf(Arc::new(bytes)),
            FontType::Eot => Self::Eot(Arc::new(bytes)),
            FontType::Woff => Self::Woff(Arc::new(bytes)),
            FontType::Woff2 => Self::Woff2(Arc::new(bytes)),
        })
    }

    /// Put this format into a result's outputs.
    pub(crate) fn place(self, outputs: &mut FontOutputs) {
        match self {
            Self::Svg(text) => outputs.svg_font = Some(text),
            Self::Ttf(bytes) => outputs.ttf_font = Some(bytes),
            Self::Eot(bytes) => outputs.eot_font = Some(bytes),
            Self::Woff(bytes) => outputs.woff_font = Some(bytes),
            Self::Woff2(bytes) => outputs.woff2_font = Some(bytes),
        }
    }
}

#[derive(Clone)]
struct FontEntry {
    key: [u8; 16],
    /// Identifies the ordered glyph names, codepoints, and contents the bytes were built from.
    glyph_set: [u8; 16],
    font: CachedFont,
}

/// Counts reported by [`clear_cache`].
#[cfg(feature = "fs")]
#[derive(Default)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct ClearCacheReport {
    /// Number of cache files removed.
    pub files_removed: usize,
    /// Total size of the removed files in bytes.
    pub bytes_removed: u64,
}

/// Remove every file the generation cache created in `cache_location`, including temporary
/// files left by interrupted writes. Other files in the directory are left untouched, and a
/// missing directory is not an error.
#[cfg(feature = "fs")]
pub fn clear_cache(cache_location: impl AsRef<Path>) -> Result<ClearCacheReport> {
    let entries = match fs::read_dir(cache_location) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(ClearCacheReport::default()),
        Err(error) => return Err(error),
    };
    let mut report = ClearCacheReport::default();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_file() || !created_by_cache(&path) {
            continue;
        }
        let size = entry.metadata().map(|metadata| metadata.len()).unwrap_or(0);
        match fs::remove_file(&path) {
            Ok(()) => {
                report.files_removed += 1;
                report.bytes_removed += size;
            }
            // A concurrent run may have renamed or removed it first.
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(report)
}

/// Cache contents for one generation, independent of where the bytes live: identity, decoded
/// entries, and everything needed to look up and produce updated bytes.
pub(crate) struct CacheState {
    fingerprint: [u8; 16],
    /// Body checksum and length of the bytes last loaded or returned by `update`, to skip
    /// rewriting identical bytes without keeping a second copy of them.
    loaded: Option<(u32, usize)>,
    processed_signature: Option<[u8; 16]>,
    parsed: HashMap<[u8; 16], Arc<CachedGlyph>>,
    processed: HashMap<[u8; 16], CachedProcessedGlyph>,
    /// Variant families: processed glyphs per content hash and advance width bits.
    variant_processed: HashMap<[u8; 16], Vec<(u64, CachedProcessedGlyph)>>,
    fonts: Vec<FontEntry>,
    glyph_set: [u8; 16],
    /// Digest of the settings each format depends on, in `FONT_TYPES` order, kept so keys can be
    /// recomputed when the glyphs change.
    #[cfg(feature = "in_memory")]
    settings_digests: [[u8; 16]; 5],
    /// Level 1 key per format, in `FONT_TYPES` order.
    font_keys: [[u8; 16]; 5],
}

impl CacheState {
    pub(crate) fn new(options: &ResolvedGenerationOptions, sources: &[LoadedSvgSource]) -> Self {
        let settings = font_settings_json(options);
        Self::with_identity(&settings, &settings, glyph_set(options, sources))
    }

    /// A variant family's cache: a family marker keeps its file apart from a single-design cache
    /// for the same font name, and the designs (names, weights, default) key its fonts.
    pub(crate) fn new_variant<Files: AsRef<[LoadedSvgSource]>>(
        options: &ResolvedGenerationOptions,
        variants: &ResolvedVariants,
        family: &VariantFamilySources<Files>,
    ) -> Self {
        let settings = font_settings_json(options);
        let with_variants = |value: Value| {
            let mut settings = settings.clone();
            let object = settings.as_object_mut().expect("settings are an object");
            object.insert("variants".to_owned(), value);
            settings
        };
        // Design names, weights and the default only shape the variable-font tables, so they key
        // the fonts but not the file: renaming a design keeps every saved glyph. The marker alone
        // keeps a family's file apart from a single design's.
        let identity = with_variants(json!(true));
        let designs: Vec<_> = variants
            .variants
            .iter()
            .map(|variant| json!([variant.name, variant.weight]))
            .collect();
        let keys = with_variants(json!({ "default": variants.default_index, "designs": designs }));
        Self::with_identity(&identity, &keys, variant_glyph_set(family))
    }

    /// `identity` decides which file (and so which saved glyphs) applies; `keys` adds what only
    /// the finished fonts depend on.
    fn with_identity(identity: &Value, keys: &Value, glyph_set: [u8; 16]) -> Self {
        // Level 2 validity depends on everything except per-format encoding options and palette
        // slots (only the color tables use them), which only level 1 keys need. Those stay out of
        // the file identity so formats share one file and slot changes keep saved glyphs.
        let mut glyph_settings = with_format_options(identity, &[]);
        if let Some(settings) = glyph_settings.as_object_mut() {
            settings.remove("colorSlots");
        }
        let fingerprint = digest(&(FORMAT_VERSION, ENGINE_VERSION, &glyph_settings));
        let settings_digests = format_settings_digests(keys);
        Self {
            fingerprint,
            loaded: None,
            processed_signature: None,
            parsed: HashMap::new(),
            processed: HashMap::new(),
            variant_processed: HashMap::new(),
            fonts: Vec::new(),
            glyph_set,
            #[cfg(feature = "in_memory")]
            settings_digests,
            font_keys: font_keys(&settings_digests, &glyph_set),
        }
    }

    #[cfg(feature = "in_memory")]
    /// Re-identify the glyphs (from [`glyph_set`] or [`variant_glyph_set`]) after a membership
    /// or content change that kept the settings.
    pub(crate) fn refresh_glyph_set(&mut self, glyph_set: [u8; 16]) {
        self.glyph_set = glyph_set;
        self.font_keys = font_keys(&self.settings_digests, &self.glyph_set);
    }

    /// Adopt previously saved bytes. Anything that doesn't decode for this identity is ignored.
    pub(crate) fn load(&mut self, bytes: Vec<u8>) {
        if let Some(body_checksum) = self.decode(&bytes) {
            self.loaded = Some((body_checksum, bytes.len()));
        } else {
            self.processed_signature = None;
            self.parsed.clear();
            self.processed.clear();
            self.variant_processed.clear();
            self.fonts.clear();
        }
    }

    /// Free the decoded glyphs once they seeded a glyph cache: `update` and `update_variants`
    /// rebuild them from it.
    #[cfg(feature = "in_memory")]
    pub(crate) fn release_glyphs(&mut self) {
        self.parsed = HashMap::new();
        self.processed = HashMap::new();
        self.variant_processed = HashMap::new();
    }

    fn font_key(&self, font_type: FontType) -> [u8; 16] {
        self.font_keys[usize::from(font_type_tag(font_type))]
    }

    /// Cached output for each requested format that has a current entry.
    pub(crate) fn font_hits(&self, types: &[FontType]) -> Vec<CachedFont> {
        types
            .iter()
            .filter_map(|font_type| {
                let key = self.font_key(*font_type);
                self.fonts
                    .iter()
                    .find(|entry| entry.font.font_type() == *font_type && entry.key == key)
                    .map(|entry| entry.font.clone())
            })
            .collect()
    }

    /// Seed an in-memory glyph cache so unchanged glyphs skip parse and processing.
    pub(crate) fn hydrate(
        &self,
        svg_options: &SvgOptions,
        sources: &[LoadedSvgSource],
    ) -> GlyphCache {
        let mut cache = GlyphCache::default();
        for source in sources {
            // Saved work only applies in the same color mode the glyph is selected for now.
            let color = svg_options.selects_color(&source.glyph_name);
            let hash = source_content_hash(&source.contents);
            if let Some(glyph) = self.parsed.get(&hash)
                && glyph.color_layers.is_some() == color
            {
                cache.by_content_hash.insert(hash, glyph.clone());
            }
            if let Some(glyph) = self.processed.get(&hash)
                && glyph.color_layers.is_some() == color
            {
                cache
                    .processed_entries
                    .insert(source.id.clone(), glyph.clone());
            }
        }
        cache.processed_signature = self.processed_signature;
        cache
    }

    /// Seed a variant glyph cache so unchanged glyphs skip parse and processing.
    pub(crate) fn hydrate_variants<Files: AsRef<[LoadedSvgSource]>>(
        &self,
        svg_options: &SvgOptions,
        variants: &[Files],
    ) -> VariantGlyphCache {
        let mut cache = VariantGlyphCache {
            signature: self.processed_signature,
            ..Default::default()
        };
        for (index, files) in variants.iter().enumerate() {
            let mut parsed = GlyphCache::default();
            for source in files.as_ref() {
                let color = svg_options.selects_color(&source.glyph_name);
                let hash = source_content_hash(&source.contents);
                let Some(glyph) = self
                    .parsed
                    .get(&hash)
                    .filter(|glyph| glyph.color_layers.is_some() == color)
                else {
                    continue;
                };
                // A known content hash marks the entry current, so preparation keeps it.
                parsed.entries.insert(source.id.clone(), glyph.clone());
                parsed.content_hashes.insert(source.id.clone(), hash);
                parsed.by_content_hash.insert(hash, glyph.clone());
                let processed = self.variant_processed.get(&hash).into_iter().flatten();
                for (advance, glyph) in
                    processed.filter(|(_, glyph)| glyph.color_layers.is_some() == color)
                {
                    cache
                        .processed
                        .insert((index, source.id.clone(), *advance), glyph.clone());
                }
            }
            cache.parsed.push(parsed);
        }
        cache
    }

    /// Replace level 2 with `glyph_cache` (current sources only) and add level 1 entries from
    /// `outputs`. Returns the new bytes, or `None` when they equal the last loaded or returned
    /// ones.
    pub(crate) fn update(
        &mut self,
        sources: &[LoadedSvgSource],
        glyph_cache: &GlyphCache,
        outputs: &FontOutputs,
        types: &[FontType],
    ) -> Option<Vec<u8>> {
        self.parsed.clear();
        self.processed.clear();
        // ponytail: one entry per content hash, so an SVG used as both a color and a monochrome
        // glyph keeps only one mode's work (the other recomputes). Key by mode if that shows up.
        for source in sources {
            let hash = source_content_hash(&source.contents);
            // Glyphs reused straight from the cache are only indexed by content hash.
            if let Some(glyph) = glyph_cache
                .entries
                .get(&source.id)
                .or_else(|| glyph_cache.by_content_hash.get(&hash))
            {
                self.parsed.insert(hash, glyph.clone());
            }
            if let Some(glyph) = glyph_cache.processed_entries.get(&source.id) {
                self.processed.insert(hash, glyph.clone());
            }
        }
        self.processed_signature = glyph_cache.processed_signature;
        self.finish(outputs, types)
    }

    /// Variant counterpart of [`Self::update`]: level 2 comes from `glyph_cache`, restricted to
    /// the current `variants` sources.
    pub(crate) fn update_variants<Files: AsRef<[LoadedSvgSource]>>(
        &mut self,
        variants: &[Files],
        glyph_cache: &VariantGlyphCache,
        outputs: &FontOutputs,
        types: &[FontType],
    ) -> Option<Vec<u8>> {
        self.parsed.clear();
        self.variant_processed.clear();
        let mut hashes = HashMap::new();
        for (index, files) in variants.iter().enumerate() {
            let Some(parsed) = glyph_cache.parsed.get(index) else {
                continue;
            };
            for source in files.as_ref() {
                let hash = source_content_hash(&source.contents);
                if parsed.content_hashes.get(&source.id) != Some(&hash) {
                    continue;
                }
                if let Some(glyph) = parsed.entries.get(&source.id) {
                    self.parsed.insert(hash, glyph.clone());
                    hashes.insert((index, source.id.as_str()), hash);
                }
            }
        }
        // `processed` iterates in arbitrary order: keeping only glyphs in the same color mode as
        // the saved parsed glyph makes every candidate identical, so the bytes are deterministic.
        for ((index, id, advance), glyph) in &glyph_cache.processed {
            if let Some(hash) = hashes.get(&(*index, id.as_str()))
                && self.parsed.get(hash).is_some_and(|parsed| {
                    parsed.color_layers.is_some() == glyph.color_layers.is_some()
                })
            {
                let entries = self.variant_processed.entry(*hash).or_default();
                if entries.iter().all(|(known, _)| known != advance) {
                    entries.push((*advance, glyph.clone()));
                }
            }
        }
        self.processed_signature = glyph_cache.signature;
        self.finish(outputs, types)
    }

    /// Add level 1 entries from `outputs` and encode. Returns the new bytes, or `None` when they
    /// equal the last loaded or returned ones.
    fn finish(&mut self, outputs: &FontOutputs, types: &[FontType]) -> Option<Vec<u8>> {
        let mut fonts: Vec<FontEntry> = types
            .iter()
            .filter_map(|font_type| {
                Some(FontEntry {
                    key: self.font_key(*font_type),
                    glyph_set: self.glyph_set,
                    font: CachedFont::from_outputs(outputs, *font_type)?,
                })
            })
            .collect();
        for entry in take(&mut self.fonts) {
            let same_type = fonts
                .iter()
                .filter(|kept| kept.font.font_type() == entry.font.font_type())
                .collect::<Vec<_>>();
            // Older entries only survive for the same glyphs (an option-only difference); once the
            // icons change they can never match again.
            if entry.glyph_set == self.glyph_set
                && same_type.len() < ENTRIES_PER_FORMAT
                && same_type.iter().all(|kept| kept.key != entry.key)
            {
                fonts.push(entry);
            }
        }
        self.fonts = fonts;

        let (encoded, body_checksum) = self.encode();
        let identity = Some((body_checksum, encoded.len()));
        // ponytail: a CRC32 collision skips one write; the old bytes stay valid for their own
        // keys, so output is unaffected and the next change rewrites them.
        if self.loaded == identity {
            return None;
        }
        self.loaded = identity;
        Some(encoded)
    }

    /// The file bytes and their body checksum.
    fn encode(&self) -> (Vec<u8>, u32) {
        // The checksum catches corruption the structure checks can't, such as a flipped byte
        // inside stored font bytes. The body is written straight after the header and the
        // checksum filled in afterwards, so the bytes are never copied.
        let mut out = Writer::default();
        out.0.extend_from_slice(MAGIC);
        out.u32(FORMAT_VERSION);
        out.bytes(ENGINE_VERSION.as_bytes());
        out.array(&self.fingerprint);
        let checksum_at = out.0.len();
        out.u32(0);
        let body_start = out.0.len();

        // Always set after a store: preparation computes it before anything is persisted.
        out.array(&self.processed_signature.unwrap_or_default());

        // Sorted so identical content always encodes identically (lets unchanged runs skip writes).
        let mut parsed: Vec<_> = self.parsed.iter().collect();
        parsed.sort_unstable_by_key(|(hash, _)| **hash);
        out.len(parsed.len());
        for (hash, glyph) in parsed {
            out.array(hash);
            out.f64(glyph.height);
            out.f64(glyph.width);
            out.len(glyph.paths.len());
            for path in &glyph.paths {
                out.tiny_path(path);
            }
            out.parsed_color_layers(glyph.color_layers.as_deref());
        }

        let mut processed: Vec<_> = self.processed.iter().collect();
        processed.sort_unstable_by_key(|(hash, _)| **hash);
        out.len(processed.len());
        for (hash, glyph) in processed {
            out.array(hash);
            out.processed_glyph(glyph);
        }

        let mut variant_processed: Vec<_> = self.variant_processed.iter().collect();
        variant_processed.sort_unstable_by_key(|(hash, _)| **hash);
        out.len(variant_processed.len());
        for (hash, entries) in variant_processed {
            out.array(hash);
            let mut entries: Vec<_> = entries.iter().collect();
            entries.sort_unstable_by_key(|(advance, _)| *advance);
            out.len(entries.len());
            for (advance, glyph) in entries {
                out.u64(*advance);
                out.processed_glyph(glyph);
            }
        }

        out.len(self.fonts.len());
        for entry in &self.fonts {
            out.u8(font_type_tag(entry.font.font_type()));
            out.array(&entry.key);
            out.array(&entry.glyph_set);
            out.bytes(entry.font.bytes());
        }
        let body_checksum = checksum(&out.0[body_start..]);
        out.0[checksum_at..body_start].copy_from_slice(&body_checksum.to_le_bytes());
        (out.0, body_checksum)
    }

    /// Decode `bytes` into this state, returning the verified body checksum.
    fn decode(&mut self, bytes: &[u8]) -> Option<u32> {
        let mut input = Reader(bytes);
        (input.take(MAGIC.len())? == MAGIC).then_some(())?;
        (input.u32()? == FORMAT_VERSION).then_some(())?;
        (input.bytes()? == ENGINE_VERSION.as_bytes()).then_some(())?;
        (input.array::<16>()? == self.fingerprint).then_some(())?;
        let expected = input.u32()?;
        (checksum(input.0) == expected).then_some(())?;

        self.processed_signature = Some(input.array()?);

        for _ in 0..input.len(16)? {
            let hash = input.array()?;
            let height = input.f64()?;
            let width = input.f64()?;
            let mut paths = Vec::new();
            for _ in 0..input.len(8)? {
                paths.push(input.tiny_path()?);
            }
            let color_layers = input.parsed_color_layers(paths.len())?;
            self.parsed.insert(
                hash,
                Arc::new(CachedGlyph {
                    color_layers,
                    height,
                    paths,
                    width,
                }),
            );
        }

        for _ in 0..input.len(16)? {
            let hash = input.array()?;
            self.processed.insert(hash, input.processed_glyph()?);
        }

        for _ in 0..input.len(16)? {
            let hash = input.array()?;
            let mut entries = Vec::new();
            for _ in 0..input.len(8)? {
                entries.push((input.u64()?, input.processed_glyph()?));
            }
            self.variant_processed.insert(hash, entries);
        }

        for _ in 0..input.len(33)? {
            let font_type = *FONT_TYPES.get(input.u8()? as usize)?;
            let key = input.array()?;
            let glyph_set = input.array()?;
            let font = CachedFont::decode(font_type, input.bytes()?.to_vec())?;
            self.fonts.push(FontEntry {
                key,
                glyph_set,
                font,
            });
        }
        input.0.is_empty().then_some(expected)
    }
}

/// Whether `path` is a cache file (checked by its name and header) or a temporary file left by an
/// interrupted cache write (checked by its exact generated name).
#[cfg(feature = "fs")]
fn created_by_cache(path: &Path) -> bool {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    if let Some(stem) = name.strip_suffix(&format!(".{EXTENSION}")) {
        cache_stem(stem).is_some() && has_cache_header(path)
    } else {
        temp_file_stem(&name).is_some_and(|stem| cache_stem(stem).is_some())
    }
}

#[cfg(feature = "fs")]
/// The font prefix of a cache file stem shaped `<prefix>.<16 hex digits>`.
fn cache_stem(stem: &str) -> Option<&str> {
    let (prefix, hash) = stem.rsplit_once('.')?;
    (hash.len() == 16 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())).then_some(prefix)
}

#[cfg(feature = "fs")]
/// The cache file stem of a temporary file named `<stem>.wfcache.<pid>-<write>.tmp`, as `write`
/// creates them.
fn temp_file_stem(name: &str) -> Option<&str> {
    let (stem, suffix) = name
        .strip_suffix(".tmp")?
        .rsplit_once(&format!(".{EXTENSION}."))?;
    let (pid, write) = suffix.split_once('-')?;
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    (digits(pid) && digits(write)).then_some(stem)
}

#[cfg(feature = "fs")]
/// Whether `path` starts with the cache magic. Only the header is read, however large the file is.
fn has_cache_header(path: &Path) -> bool {
    let mut header = [0; MAGIC.len()];
    File::open(path)
        .and_then(|mut file| file.read_exact(&mut header))
        .is_ok_and(|()| &header == MAGIC)
}

/// The `cacheLocation` file for one generation.
#[cfg(feature = "fs")]
pub(crate) struct DiskCache {
    dir: PathBuf,
    file_name: String,
    font_prefix: String,
    state: CacheState,
}

#[cfg(feature = "fs")]
impl Deref for DiskCache {
    type Target = CacheState;

    fn deref(&self) -> &CacheState {
        &self.state
    }
}

#[cfg(feature = "fs")]
impl DiskCache {
    pub(crate) fn open(
        dir: &str,
        options: &ResolvedGenerationOptions,
        sources: &[LoadedSvgSource],
    ) -> Self {
        Self::load(dir, &options.font_name, CacheState::new(options, sources))
    }

    pub(crate) fn open_variant<Files: AsRef<[LoadedSvgSource]>>(
        dir: &str,
        options: &ResolvedGenerationOptions,
        variants: &ResolvedVariants,
        family: &VariantFamilySources<Files>,
    ) -> Self {
        let state = CacheState::new_variant(options, variants, family);
        Self::load(dir, &options.font_name, state)
    }

    fn load(dir: &str, font_name: &str, mut state: CacheState) -> Self {
        let dir = PathBuf::from(dir);
        let font_prefix = sanitize(font_name);
        let file_name = format!(
            "{font_prefix}.{}.{EXTENSION}",
            &hex(&state.fingerprint)[..16]
        );
        if let Ok(bytes) = fs::read(dir.join(&file_name)) {
            state.load(bytes);
        }
        Self {
            dir,
            file_name,
            font_prefix,
            state,
        }
    }

    /// Persist the updated cache. Best effort: an unwritable location only costs the next run its
    /// warm start.
    pub(crate) fn store(
        mut self,
        sources: &[LoadedSvgSource],
        glyph_cache: &GlyphCache,
        outputs: &FontOutputs,
        types: &[FontType],
    ) {
        let bytes = self.state.update(sources, glyph_cache, outputs, types);
        self.save(bytes);
    }

    /// Variant counterpart of [`Self::store`].
    pub(crate) fn store_variants<Files: AsRef<[LoadedSvgSource]>>(
        mut self,
        variants: &[Files],
        glyph_cache: &VariantGlyphCache,
        outputs: &FontOutputs,
        types: &[FontType],
    ) {
        let bytes = self
            .state
            .update_variants(variants, glyph_cache, outputs, types);
        self.save(bytes);
    }

    fn save(self, bytes: Option<Vec<u8>>) {
        let Self {
            dir,
            file_name,
            font_prefix,
            ..
        } = self;
        if let Some(bytes) = bytes {
            let path = dir.join(&file_name);
            match write(&dir, &file_name, &bytes) {
                Ok(()) => prune_siblings(&dir, &file_name, &font_prefix),
                // The cache never removes what it didn't create, so this stays cold until the user
                // removes the directory; say so instead of failing silently on every run.
                Err(_) if path.is_dir() => eprintln!(
                    "warning: webfont-generator: {} is a directory, so the generation cache can't be saved there; remove it to re-enable caching",
                    path.display()
                ),
                Err(_) => {}
            }
        }
    }
}

#[cfg(feature = "fs")]
fn write(dir: &Path, file_name: &str, bytes: &[u8]) -> Result<()> {
    fs::create_dir_all(dir)?;
    // Unique per write within this process, so concurrent generations never share a file.
    static WRITES: AtomicU64 = AtomicU64::new(0);
    let write = WRITES.fetch_add(1, Relaxed);
    let tmp = dir.join(format!("{}.{}-{write}.tmp", file_name, process::id()));
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, dir.join(file_name)).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

/// Drop older files for the same font name: they belong to superseded glyph settings.
// ponytail: two configs sharing a font name but differing in glyph settings evict each other
// (each run is then cold). Key the prefix by more than the font name if that shows up.
#[cfg(feature = "fs")]
fn prune_siblings(dir: &Path, file_name: &str, font_prefix: &str) {
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let sibling = name
            .strip_suffix(&format!(".{EXTENSION}"))
            .and_then(cache_stem)
            .is_some_and(|prefix| prefix == font_prefix);
        if sibling
            && name != file_name
            && entry.file_type().is_ok_and(|kind| kind.is_file())
            && has_cache_header(&entry.path())
        {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Digest of the font-affecting settings each format depends on, in `FONT_TYPES` order: the SVG
/// options (part of every glyph), plus the TTF options for formats built from the TTF, plus the
/// format's own options.
fn format_settings_digests(settings: &Value) -> [[u8; 16]; 5] {
    FONT_TYPES.map(|font_type| {
        let own: &[&str] = match font_type {
            FontType::Svg => &[],
            FontType::Ttf | FontType::Eot => &["ttf"],
            FontType::Woff => &["ttf", "woff"],
            FontType::Woff2 => &["ttf", "woff2"],
        };
        digest(&with_format_options(settings, own))
    })
}

/// `settings` with only the SVG options plus the `own` format options. Empty option objects are
/// dropped, so leaving `formatOptions` (or one format's options) unset and setting it to nothing
/// produce the same identity.
fn with_format_options(settings: &Value, own: &[&str]) -> Value {
    let mut settings = settings.clone();
    if let Some(object) = settings.as_object_mut()
        && let Some(formats) = object
            .get_mut("formatOptions")
            .and_then(Value::as_object_mut)
    {
        formats.retain(|name, options| {
            (name == "svg" || own.contains(&name.as_str()))
                && options
                    .as_object()
                    .is_none_or(|options| !options.is_empty())
        });
        if formats.is_empty() {
            object.remove("formatOptions");
        }
    }
    settings
}

/// Key for each format's bytes: the settings that format depends on, plus the ordered glyph names,
/// codepoints, and source content hashes.
fn font_keys(settings_digests: &[[u8; 16]; 5], glyph_set: &[u8; 16]) -> [[u8; 16]; 5] {
    from_fn(|index| {
        digest(&(
            FORMAT_VERSION,
            ENGINE_VERSION,
            FONT_TYPES[index].as_extension(),
            settings_digests[index],
            glyph_set,
        ))
    })
}

/// CRC32 of the file body, verified before decoding.
fn checksum(bytes: &[u8]) -> u32 {
    let mut crc = Crc::new();
    crc.update(bytes);
    crc.sum()
}

/// MD5 of a value's JSON serialization, for identities and keys.
fn digest(value: &impl Serialize) -> [u8; 16] {
    compute(to_vec(value).expect("cache identity serializes")).0
}

/// Identifies the ordered glyph names, codepoints, and source contents.
pub(crate) fn glyph_set(
    options: &ResolvedGenerationOptions,
    sources: &[LoadedSvgSource],
) -> [u8; 16] {
    let glyphs: Vec<Value> = sources
        .iter()
        .map(|source| {
            json!([
                source.glyph_name,
                options.codepoints.get(&source.glyph_name),
                hex(&source_content_hash(&source.contents)),
            ])
        })
        .collect();
    digest(&glyphs)
}

/// Identifies a variant family's ordered glyph names and codepoints, and the source contents
/// (or blank) each design uses for them, which also captures missing-glyph resolution.
pub(crate) fn variant_glyph_set<Files: AsRef<[LoadedSvgSource]>>(
    family: &VariantFamilySources<Files>,
) -> [u8; 16] {
    let glyphs: Vec<Value> = family
        .glyphs
        .iter()
        .map(|glyph| {
            let designs: Vec<Option<String>> = glyph
                .sources
                .iter()
                .map(|source| match source {
                    Some(VariantGlyphSource::Source {
                        variant_index,
                        source_index,
                    }) => Some(hex(&source_content_hash(
                        &family.variants[*variant_index].as_ref()[*source_index].contents,
                    ))),
                    _ => None,
                })
                .collect();
            json!([glyph.name, glyph.codepoint, designs])
        })
        .collect();
    digest(&glyphs)
}

#[cfg(feature = "fs")]
fn sanitize(font_name: &str) -> String {
    font_name
        .chars()
        .map(|char| {
            if char.is_ascii_alphanumeric() || char == '-' || char == '_' {
                char
            } else {
                '_'
            }
        })
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
