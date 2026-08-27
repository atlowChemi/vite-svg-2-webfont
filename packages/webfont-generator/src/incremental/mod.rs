#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::Path;

use crate::input::{LoadedSvgFile, finalize_generate_webfonts_options, validate_glyph_names};
use crate::output::write_generate_webfonts_result_sync;
use crate::pipeline::build_font_outputs;
use crate::svg::{prepare_svg_font_incremental, source_content_hash, svg_options_from_options};
use crate::types::{GenerateWebfontsResult, GlyphChange, RegenerateError, RegenerationState};

impl GenerateWebfontsResult {
    /// Rebuild after a batch of file changes, reusing cached glyph geometry for files whose
    /// contents are unchanged. Requires the result to have been generated with `incremental`
    /// enabled. `ordered_paths` is the complete file set after the changes, in the order a fresh
    /// build would use (e.g. the glob result); the rebuilt glyphs are ordered to match it, so
    /// auto-assigned codepoints and glyph order — and therefore the output bytes — are identical
    /// to a fresh `generate` of that set, including for additions that sort before existing
    /// glyphs. `changes` describes what to do per affected file: added/changed files are read from
    /// disk and re-parsed; any file absent from `ordered_paths` is dropped (an explicit `Removed`
    /// is optional but harmless). Every requested format is rebuilt in memory, and — matching
    /// `generate` — when the result was built with `write_files` enabled the refreshed fonts are
    /// written to disk too, while CSS/HTML companion files are skipped if their rendered bytes are
    /// unchanged from the previous write. Rendered CSS/HTML is reused when the glyph names and
    /// codepoints the templates read are unchanged (a content edit), and re-rendered otherwise.
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
    ///     &files,
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
        ordered_paths: &[String],
        changes: &[(String, GlyphChange)],
    ) -> std::io::Result<()> {
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
    ///     files.clone(),
    ///     vec![(files[0].clone(), GlyphChange::Changed { name: None })],
    /// ).await?;
    /// # let _ = result;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn regenerate_async(
        self,
        ordered_paths: Vec<String>,
        changes: Vec<(String, GlyphChange)>,
    ) -> Result<Self, RegenerateError> {
        self.regenerate_in_background(ordered_paths, Some(changes))
            .await
    }

    fn regenerate_with_state(
        &mut self,
        ordered_paths: &[String],
        changes: &[(String, GlyphChange)],
        state: &mut RegenerationState,
    ) -> std::io::Result<()> {
        let cache = &mut state.glyph_cache;
        let mut source_files = self.source_files.as_ref().clone();
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
            .map(|file| file.path.as_str())
            .ne(ordered_paths.iter().map(String::as_str));
        let mut changed_inputs = order_changed;

        for (path, change) in changes {
            match change {
                GlyphChange::Removed => {
                    state.caches_dirty = true;
                    let before = source_files.len();
                    source_files.retain(|file| &file.path != path);
                    cache.entries.remove(path);
                    cache.content_hashes.remove(path);
                    cache.processed_entries.remove(path);
                    changed_inputs |= source_files.len() != before;
                }
                GlyphChange::Added { name } => {
                    let contents = std::fs::read_to_string(path)?;
                    state.caches_dirty = true;
                    let hash = source_content_hash(&contents);
                    upsert_source_file(&mut source_files, path, contents, name.clone());
                    // Reuse identical SVG geometry if another path already parsed these bytes;
                    // otherwise prepare_svg_font_incremental will parse and populate the cache.
                    if let Some(cached) = cache.by_content_hash.get(&hash) {
                        cache.entries.insert(path.clone(), cached.clone());
                        cache.content_hashes.insert(path.clone(), hash);
                    } else {
                        cache.entries.remove(path);
                        cache.content_hashes.remove(path);
                    }
                    cache.processed_entries.remove(path);
                    changed_inputs = true;
                }
                GlyphChange::Changed { name } => {
                    let contents = std::fs::read_to_string(path)?;
                    let hash = source_content_hash(&contents);
                    let content_changed = cache.content_hashes.get(path) != Some(&hash);
                    let name_changed = name.as_ref().is_some_and(|name| {
                        source_files
                            .iter()
                            .find(|file| &file.path == path)
                            .is_none_or(|file| file.glyph_name != *name)
                    });

                    if !content_changed && !name_changed {
                        continue;
                    }

                    upsert_source_file(&mut source_files, path, contents, name.clone());
                    if content_changed {
                        state.caches_dirty = true;
                        if let Some(cached) = cache.by_content_hash.get(&hash) {
                            cache.entries.insert(path.clone(), cached.clone());
                            cache.content_hashes.insert(path.clone(), hash);
                        } else {
                            cache.entries.remove(path);
                            cache.content_hashes.remove(path);
                        }
                        cache.processed_entries.remove(path);
                    }
                    changed_inputs = true;
                }
            }
        }

        if !changed_inputs {
            return Ok(());
        }

        // Reorder to the caller's order so additions land in their fresh-build position rather than
        // at the tail. `ordered_paths` is authoritative for membership: any file not listed is
        // dropped here (handling removals), and every listed path must resolve to a known file.
        let mut by_path: std::collections::HashMap<String, LoadedSvgFile> = source_files
            .into_iter()
            .map(|file| (file.path.clone(), file))
            .collect();
        let mut reordered = Vec::with_capacity(ordered_paths.len());
        for path in ordered_paths {
            let file = by_path.remove(path).ok_or_else(|| {
                std::io::Error::new(
                    ErrorKind::InvalidInput,
                    format!("regenerate: `{path}` is in the ordered set but was not loaded; declare it as an added change."),
                )
            })?;
            reordered.push(file);
        }
        source_files = reordered;
        validate_glyph_names(&source_files)?;
        options.files = ordered_paths.to_vec();

        // Re-resolve codepoints for the (possibly changed) set from the stable explicit base, so
        // auto-assigned codepoints match what a fresh build of the new set would produce.
        finalize_generate_webfonts_options(&mut options, &source_files)?;

        let svg_options = svg_options_from_options(&options);
        state.caches_dirty = true;
        let prepared = prepare_svg_font_incremental(&svg_options, &source_files, cache)?;
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
    /// result.regenerate_all(&files)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn regenerate_all(&mut self, ordered_paths: &[String]) -> std::io::Result<()> {
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
            previous_paths.insert(source_file.path.as_str());
            if !ordered_set.contains(source_file.path.as_str()) {
                changes.push((source_file.path.clone(), GlyphChange::Removed));
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
        self.regenerate(ordered_paths, &changes)
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
    /// let result = result.regenerate_all_async(files).await?;
    /// # let _ = result;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn regenerate_all_async(
        self,
        ordered_paths: Vec<String>,
    ) -> Result<Self, RegenerateError> {
        self.regenerate_in_background(ordered_paths, None).await
    }

    async fn regenerate_in_background(
        mut self,
        ordered_paths: Vec<String>,
        changes: Option<Vec<(String, GlyphChange)>>,
    ) -> Result<Self, RegenerateError> {
        let task = tokio::task::spawn_blocking(move || {
            let outcome = match changes {
                Some(changes) => self.regenerate(&ordered_paths, &changes),
                None => self.regenerate_all(&ordered_paths),
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

/// Insert or update a source file by path: update an existing entry's contents (and name, if
/// given), or append a new entry (deriving the glyph name from the file stem when none is given).
fn upsert_source_file(
    source_files: &mut Vec<LoadedSvgFile>,
    path: &str,
    contents: String,
    name: Option<String>,
) {
    if let Some(file) = source_files.iter_mut().find(|file| file.path == path) {
        file.contents = contents.into();
        if let Some(name) = name {
            file.glyph_name = name;
        }
    } else {
        let glyph_name = name.unwrap_or_else(|| {
            Path::new(path)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default()
                .to_owned()
        });
        source_files.push(LoadedSvgFile {
            contents: contents.into(),
            glyph_name,
            path: path.to_owned(),
        });
    }
}
