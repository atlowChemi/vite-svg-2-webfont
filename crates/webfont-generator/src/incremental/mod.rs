#[cfg(all(test, feature = "fs"))]
mod tests;

#[cfg(feature = "fs")]
mod files;
pub(crate) mod sources;
#[cfg(feature = "fs")]
mod variants;

#[cfg(feature = "fs")]
use std::collections::{HashMap, HashSet};
#[cfg(feature = "fs")]
use std::io::ErrorKind;
#[cfg(feature = "fs")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "fs")]
use crate::input::{finalize_generate_webfonts_options, validate_glyph_names};
#[cfg(feature = "fs")]
use crate::output::write_generate_webfonts_result_sync;
#[cfg(feature = "fs")]
use crate::pipeline::{TtfGlyphCache, build_font_outputs};
#[cfg(feature = "fs")]
use crate::result::{GenerateWebfontsResult, RegenerateError};
#[cfg(feature = "fs")]
use crate::svg::types::GlyphCache;
#[cfg(feature = "fs")]
use crate::svg::{prepare_svg_font_incremental, source_content_hash, svg_options_from_options};
#[cfg(feature = "fs")]
use crate::types::{GlyphChange, RegenerationFiles};
#[cfg(feature = "fs")]
use files::load_source_updates;
#[cfg(feature = "fs")]
use sources::{ordered_sources, updated_sources};

#[cfg(feature = "fs")]
pub(crate) struct RegenerationState {
    pub(crate) variant_cache: Option<crate::svg::VariantGlyphCache>,
    pub(crate) variant_write_pending: bool,
    pub(crate) caches_dirty: bool,
    pub(crate) glyph_cache: GlyphCache,
    pub(crate) ttf_cache: Option<TtfGlyphCache>,
    pub(crate) written_outputs: HashMap<String, [u8; 16]>,
}

#[cfg(feature = "fs")]
struct RegenerationStateLease {
    slot: Arc<Mutex<Option<RegenerationState>>>,
    state: Option<RegenerationState>,
    keep_caches: bool,
}

#[cfg(feature = "fs")]
impl RegenerationStateLease {
    fn state_mut(&mut self) -> &mut RegenerationState {
        self.state.as_mut().unwrap()
    }

    fn commit(mut self) {
        self.keep_caches = true;
    }
}

#[cfg(feature = "fs")]
impl Drop for RegenerationStateLease {
    fn drop(&mut self) {
        let mut state = self.state.take().unwrap();
        if !self.keep_caches && state.caches_dirty {
            state.variant_cache = None;
            state.glyph_cache = GlyphCache::default();
            if state.ttf_cache.is_some() {
                state.ttf_cache = Some(TtfGlyphCache::default());
            }
        }
        state.caches_dirty = false;
        *self.slot.lock().unwrap() = Some(state);
    }
}

#[cfg(feature = "fs")]
impl GenerateWebfontsResult {
    fn take_regeneration_state_lease(&self) -> std::io::Result<RegenerationStateLease> {
        Ok(RegenerationStateLease {
            slot: Arc::clone(&self.regeneration_state),
            state: Some(self.take_regeneration_state()?),
            keep_caches: false,
        })
    }

    /// Rebuild after a batch of file changes, reusing cached glyph geometry for files whose
    /// contents are unchanged. Requires the result to have been generated with `incremental`
    /// enabled. `files` selects the matching ordinary or variant input mode. Each contained list
    /// is the complete file set after the changes, in the order a fresh
    /// build would use (e.g. the glob result); the rebuilt glyphs are ordered to match it, so
    /// auto-assigned codepoints and glyph order — and therefore the output bytes — are identical
    /// to a fresh `generate` of that set, including for additions that sort before existing
    /// glyphs. `changes` describes what to do per affected file: added/changed files are read from
    /// disk and re-parsed; any file absent from its list is dropped (an explicit `Removed`
    /// is optional but harmless). Every requested format is rebuilt in memory, and — matching
    /// `generate` — when the result was built with `write_files` enabled the refreshed fonts are
    /// written to disk too, while CSS/HTML companion files are skipped if their rendered bytes are
    /// unchanged from the previous write. Rendered CSS/HTML is reused when the glyph names and
    /// codepoints the templates read are unchanged (a content edit), and re-rendered otherwise.
    ///
    /// For families, supply every configured variant exactly once. Each design's file order is
    /// independent. Hints describe family-wide paths: `Added` is new to the family, `Changed`
    /// applies to every final consumer, and `Removed` must be absent from every final list.
    /// Membership-only changes need no hint; newly assigned paths are loaded automatically.
    /// Invalid batches and font-build failures preserve the previous result. Variant writes
    /// occur after the new state commits, so write errors leave retryable new in-memory output.
    ///
    /// ```rust,no_run
    /// use webfont_generator::{
    ///     generate_sync, FontType, GenerateWebfontsOptions, GlyphChange,
    /// };
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let files = vec![
    ///     "icons/add.svg".to_owned(),
    ///     "icons/remove.svg".to_owned(),
    /// ];
    /// let mut result = generate_sync(
    ///     GenerateWebfontsOptions {
    ///         dest: "dist".to_owned(),
    ///         files: files.clone(),
    ///         incremental: Some(true),
    ///         types: Some(vec![FontType::Woff2]),
    ///         write_files: Some(false),
    ///         ..Default::default()
    ///     },
    ///     None,
    /// )?;
    ///
    /// result.regenerate(
    ///     &webfont_generator::RegenerationFiles::Single(files),
    ///     &[(
    ///         "icons/add.svg".to_owned(),
    ///         GlyphChange::Changed { name: None },
    ///     )],
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn regenerate(
        &mut self,
        files: &RegenerationFiles,
        changes: &[(String, GlyphChange)],
    ) -> std::io::Result<()> {
        let ordered_paths = match files {
            RegenerationFiles::Single(files) => files.as_slice(),
            RegenerationFiles::Variants(sets) => {
                return self.regenerate_variant_files(sets, Some(changes));
            }
        };
        self.require_ordinary_regeneration()?;
        if self.css_context.is_some() || self.html_context.is_some() {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "regenerate is not supported for results generated with cssContext/htmlContext callbacks.",
            ));
        }

        let mut state = self.take_regeneration_state_lease()?;
        let result = self.regenerate_with_state(ordered_paths, changes, state.state_mut());
        if result.is_ok() {
            state.commit();
        }
        result
    }

    /// Asynchronously rebuild on Tokio's blocking pool and return the next result generation.
    /// This consumes `self`, so Rust prevents reuse of a stale result after success. On ordinary
    /// failure, [`RegenerateError::into_result`] returns the input result for retry. This future is
    /// not cancellation-safe: dropping it lets already-started blocking work finish and drops the
    /// consumed result; filesystem writes may continue.
    ///
    /// ```rust,no_run
    /// use webfont_generator::{generate_sync, GenerateWebfontsOptions, GlyphChange};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let files = vec!["icons/add.svg".to_owned()];
    /// let result = generate_sync(GenerateWebfontsOptions {
    ///     files: files.clone(),
    ///     dest: "dist".to_owned(),
    ///     incremental: Some(true),
    ///     ..Default::default()
    /// }, None)?;
    /// let result = result.regenerate_async(
    ///     webfont_generator::RegenerationFiles::Single(files.clone()),
    ///     vec![(files[0].clone(), GlyphChange::Changed { name: None })],
    /// ).await?;
    /// # let _ = result;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn regenerate_async(
        self,
        files: RegenerationFiles,
        changes: Vec<(String, GlyphChange)>,
    ) -> Result<Self, RegenerateError> {
        self.regenerate_in_background(files, Some(changes)).await
    }

    fn regenerate_with_state(
        &mut self,
        ordered_paths: &[String],
        changes: &[(String, GlyphChange)],
        state: &mut RegenerationState,
    ) -> std::io::Result<()> {
        let mut options = self.options.as_ref().clone();

        // Snapshot the inputs the CSS/HTML templates depend on (glyph names + codepoints) so we
        // can tell, after rebuilding, whether the rendered output could have changed.
        let prev_names: Vec<String> = self
            .source_files
            .iter()
            .map(|f| f.glyph_name.clone())
            .collect();
        let prev_codepoints = self.options.codepoints.clone();

        let order_changed = self
            .source_files
            .iter()
            .map(|file| file.id.as_str())
            .ne(ordered_paths.iter().map(String::as_str));
        let (updates, changed_inputs) = load_source_updates(&self.source_files, changes, state)?;
        if !order_changed && !changed_inputs {
            return Ok(());
        }

        // Reorder to the caller's order so additions land in their fresh-build position rather than
        // at the tail. `ordered_paths` is authoritative for membership: any file not listed is
        // dropped here (handling removals), and every listed path must resolve to a known file.
        let definitions = updated_sources(&self.source_files, &updates);
        let source_files = ordered_sources(&definitions, ordered_paths).map_err(|path| {
                std::io::Error::new(
                    ErrorKind::InvalidInput,
                    format!("regenerate: `{path}` is in the ordered set but was not loaded; declare it as an added change."),
                )
            })?;
        validate_glyph_names(&source_files)?;
        options.files = ordered_paths.to_vec();

        // Re-resolve codepoints for the (possibly changed) set from the stable explicit base, so
        // auto-assigned codepoints match what a fresh build of the new set would produce.
        finalize_generate_webfonts_options(&mut options, &source_files)?;

        let svg_options = svg_options_from_options(&options);
        state.caches_dirty = true;
        let prepared =
            prepare_svg_font_incremental(&svg_options, &source_files, &mut state.glyph_cache)?;
        let fonts =
            build_font_outputs(&options, &svg_options, &prepared, state.ttf_cache.as_mut())?;

        // Rebuild the template data fresh (the font hash changed), but keep the rendered CSS/HTML
        // that can't have changed: only when the glyph names and codepoints the templates read are
        // unchanged (e.g. a pure content edit). Carry those safe entries into the new result.
        let names_unchanged = source_files
            .iter()
            .map(|file| file.glyph_name.as_str())
            .eq(prev_names.iter().map(String::as_str));
        let codepoints_unchanged = options.codepoints == prev_codepoints;
        let carried = self.reusable_render_cache(names_unchanged, codepoints_unchanged);
        let previous_source_files =
            std::mem::replace(&mut self.source_files, std::sync::Arc::new(source_files));
        let previous_options = std::mem::replace(&mut self.options, std::sync::Arc::new(options));
        let previous_fonts = std::mem::replace(&mut self.fonts, fonts);
        let previous_cached = std::mem::take(&mut self.cached);
        let previous_carried = std::mem::replace(&mut self.carried_render, carried);
        let previous_css_context = self.css_context.take();
        let previous_html_context = self.html_context.take();

        // Match `generate`'s write behavior: refresh font files, and skip unchanged CSS/HTML.
        if self.options.write_files
            && let Err(error) =
                write_generate_webfonts_result_sync(self, &mut state.written_outputs)
        {
            self.source_files = previous_source_files;
            self.options = previous_options;
            self.fonts = previous_fonts;
            self.cached = previous_cached;
            self.carried_render = previous_carried;
            self.css_context = previous_css_context;
            self.html_context = previous_html_context;
            return Err(error);
        }
        Ok(())
    }

    /// Rebuild after re-diffing the complete ordered file set against the retained incremental
    /// cache. This is useful when the caller has a fresh file list but not a reliable watcher
    /// change batch: every current path is re-read and hashed, missing prior paths are treated as
    /// removed, new paths as added, and paths whose content hash changed as changed. Existing glyph
    /// names are preserved; added paths derive their glyph name from the file stem.
    ///
    /// ```rust,no_run
    /// use webfont_generator::{generate_sync, GenerateWebfontsOptions};
    ///
    /// # fn run() -> std::io::Result<()> {
    /// let files = vec!["icons/add.svg".to_owned()];
    /// let mut result = generate_sync(GenerateWebfontsOptions {
    ///     files: files.clone(),
    ///     dest: "dist".to_owned(),
    ///     incremental: Some(true),
    ///     ..Default::default()
    /// }, None)?;
    /// result.regenerate_all(&webfont_generator::RegenerationFiles::Single(files))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn regenerate_all(&mut self, files: &RegenerationFiles) -> std::io::Result<()> {
        let ordered_paths = match files {
            RegenerationFiles::Single(files) => files.as_slice(),
            RegenerationFiles::Variants(sets) => return self.regenerate_variant_files(sets, None),
        };
        self.require_ordinary_regeneration()?;
        if !self.options.incremental {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "regenerate requires the font to be generated with `incremental` enabled.",
            ));
        }
        let state = self.regeneration_state.lock().unwrap();
        let cache = &state
            .as_ref()
            .ok_or_else(|| {
                std::io::Error::new(
                    ErrorKind::WouldBlock,
                    "This result is already regenerating or has been replaced.",
                )
            })?
            .glyph_cache;
        let ordered_set: HashSet<&str> = ordered_paths.iter().map(String::as_str).collect();
        let mut previous_paths = HashSet::with_capacity(self.source_files.len());
        let mut changes = Vec::new();

        for source_file in self.source_files.iter() {
            previous_paths.insert(source_file.id.as_str());
            if !ordered_set.contains(source_file.id.as_str()) {
                changes.push((source_file.id.clone(), GlyphChange::Removed));
            }
        }

        for path in ordered_paths {
            if !previous_paths.contains(path.as_str()) {
                changes.push((path.clone(), GlyphChange::Added { name: None }));
                continue;
            }

            let contents = std::fs::read_to_string(path)?;
            let hash = source_content_hash(&contents);
            if cache.content_hashes.get(path) != Some(&hash) {
                changes.push((path.clone(), GlyphChange::Changed { name: None }));
            }
        }

        drop(state);
        self.regenerate(files, &changes)
    }

    /// Asynchronously re-diff and rebuild on Tokio's blocking pool, consuming the old result and
    /// returning the next result generation. This future is not cancellation-safe: dropping it
    /// lets already-started blocking work finish and drops the consumed result.
    ///
    /// ```rust,no_run
    /// use webfont_generator::{generate_sync, GenerateWebfontsOptions};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let files = vec!["icons/add.svg".to_owned()];
    /// let result = generate_sync(GenerateWebfontsOptions {
    ///     files: files.clone(),
    ///     dest: "dist".to_owned(),
    ///     incremental: Some(true),
    ///     ..Default::default()
    /// }, None)?;
    /// let result = result.regenerate_all_async(webfont_generator::RegenerationFiles::Single(files)).await?;
    /// # let _ = result;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn regenerate_all_async(
        self,
        files: RegenerationFiles,
    ) -> Result<Self, RegenerateError> {
        self.regenerate_in_background(files, None).await
    }

    async fn regenerate_in_background(
        mut self,
        files: RegenerationFiles,
        changes: Option<Vec<(String, GlyphChange)>>,
    ) -> Result<Self, RegenerateError> {
        let task = tokio::task::spawn_blocking(move || {
            let outcome = match changes {
                Some(changes) => self.regenerate(&files, &changes),
                None => self.regenerate_all(&files),
            };
            match outcome {
                Ok(()) => Ok(self),
                Err(source) => Err(RegenerateError::new(Some(self), source)),
            }
        });
        match task.await {
            Ok(result) => result,
            Err(error) if error.is_panic() => std::panic::resume_unwind(error.into_panic()),
            Err(error) => Err(RegenerateError::new(
                None,
                std::io::Error::other(format!("asynchronous regeneration task failed: {error}")),
            )),
        }
    }
}
