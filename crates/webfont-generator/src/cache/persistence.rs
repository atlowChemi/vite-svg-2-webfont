//! One coalescing writer per incremental lineage. Encoding and I/O never hold the queue lock.
use std::io::{Error, ErrorKind, Result};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

use super::{DiskCache, glyph_set, variant_glyph_set};
use crate::incremental::RegenerationState;
use crate::input::{LoadedSvgSource, ResolvedFileGenerationOptions};
use crate::resolve_content_variant_sources;
use crate::result::{FontOutputs, GenerateWebfontsResult};
use crate::svg::{VariantGlyphCache, types::GlyphCache};

struct Snapshot {
    options: Arc<ResolvedFileGenerationOptions>,
    sources: Arc<Vec<LoadedSvgSource>>,
    fonts: FontOutputs,
    glyphs: GlyphCache,
    variants: Option<VariantGlyphCache>,
}

impl Snapshot {
    fn save(&self, disk: &mut DiskCache) -> Result<()> {
        let bytes = if let Some(variants) = &self.options.variants {
            let mut offset = 0;
            let sources: Vec<_> = variants
                .variants
                .iter()
                .map(|variant| {
                    let end = offset + variant.source_ids.len();
                    let files = &self.sources[offset..end];
                    offset = end;
                    files
                })
                .collect();
            let family = resolve_content_variant_sources(
                &mut self.options.common.clone(),
                variants,
                sources.clone(),
            )?;
            disk.state.refresh_glyph_set(variant_glyph_set(&family));
            disk.state.update_variants(
                &sources,
                self.variants.as_ref().expect("committed variant cache"),
                &self.fonts,
                &self.options.types,
            )
        } else {
            disk.state
                .refresh_glyph_set(glyph_set(&self.options, &self.sources));
            disk.state.update(
                &self.sources,
                &self.glyphs,
                &self.fonts,
                &self.options.types,
            )
        };
        let outcome = disk.save(bytes);
        disk.state.release_glyphs();
        outcome
    }
}

struct Queue {
    disk: Option<DiskCache>,
    pending: Option<Snapshot>,
    retry: Option<Snapshot>,
    running: bool,
    error: Option<(ErrorKind, String)>,
}

pub(crate) struct Persistence {
    queue: Mutex<Queue>,
    idle: Condvar,
}

impl Persistence {
    pub(crate) fn new(mut disk: DiskCache) -> Arc<Self> {
        // Incremental generation already hydrated these into its own glyph caches.
        disk.state.release_glyphs();
        Arc::new(Self {
            queue: Mutex::new(Queue {
                disk: Some(disk),
                pending: None,
                retry: None,
                running: false,
                error: None,
            }),
            idle: Condvar::new(),
        })
    }

    fn schedule(self: &Arc<Self>, snapshot: Snapshot) {
        let mut queue = self.lock();
        // An unexpected worker failure may have lost its disk state. Keep the error available
        // to flush callers; a fresh generation creates a usable lineage again.
        if !queue.running && queue.disk.is_none() {
            return;
        }
        queue.pending = Some(snapshot);
        queue.retry = None;
        self.start(&mut queue);
    }

    fn start(self: &Arc<Self>, queue: &mut Queue) {
        self.start_with(queue, |worker| {
            std::thread::Builder::new()
                .name("webfont-cache".into())
                .spawn(move || worker.run())
                .map(|_| ())
        });
    }

    fn start_with(
        self: &Arc<Self>,
        queue: &mut Queue,
        spawn: impl FnOnce(Arc<Self>) -> Result<()>,
    ) {
        if queue.running {
            return;
        }
        queue.running = true;
        if let Err(error) = spawn(Arc::clone(self)) {
            queue.running = false;
            queue.retry = queue.pending.take();
            queue.error = Some((error.kind(), error.to_string()));
        }
    }

    fn run(&self) {
        self.run_with(Snapshot::save);
    }

    fn lock(&self) -> MutexGuard<'_, Queue> {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn run_with(&self, mut save: impl FnMut(&Snapshot, &mut DiskCache) -> Result<()>) {
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.drain(&mut save))).is_err()
        {
            let mut queue = self.lock();
            queue.disk = None;
            queue.pending = None;
            queue.retry = None;
            queue.error = Some((
                ErrorKind::Other,
                "background cache worker panicked; generate a new result to resume persistence"
                    .into(),
            ));
            queue.running = false;
            self.idle.notify_all();
        }
    }

    fn drain(&self, save: &mut impl FnMut(&Snapshot, &mut DiskCache) -> Result<()>) {
        let mut disk = self
            .lock()
            .disk
            .take()
            .expect("running cache worker owns disk state");
        loop {
            let snapshot = {
                let mut queue = self.lock();
                match queue.pending.take() {
                    Some(snapshot) => snapshot,
                    None => {
                        queue.disk = Some(disk);
                        queue.running = false;
                        self.idle.notify_all();
                        return;
                    }
                }
            };
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                save(&snapshot, &mut disk)
            }))
            .unwrap_or_else(|_| {
                disk.state.loaded = None;
                Err(Error::other("background cache writer panicked"))
            });
            let mut queue = self.lock();
            queue.error = outcome.err().map(|error| (error.kind(), error.to_string()));
            queue.retry = if queue.error.is_some() && queue.pending.is_none() {
                Some(snapshot)
            } else {
                None
            };
        }
    }

    pub(crate) fn flush(self: &Arc<Self>) -> Result<()> {
        let mut queue = self.lock();
        // Retry a previous failed attempt once. Never spin on an unwritable directory.
        if !queue.running
            && let Some(snapshot) = queue.retry.take()
        {
            queue.pending = Some(snapshot);
            self.start(&mut queue);
        }
        while queue.running {
            queue = self
                .idle
                .wait(queue)
                .unwrap_or_else(PoisonError::into_inner);
        }
        match &queue.error {
            Some((kind, message)) => Err(Error::new(*kind, message.clone())),
            None => Ok(()),
        }
    }
}

impl GenerateWebfontsResult {
    pub(crate) fn retain_initial_cache_retry(&self) {
        if let Some(cache) = &self.persistence {
            let mut queue = cache.queue.lock().unwrap();
            if queue.disk.as_ref().unwrap().state.loaded.is_none() {
                let state = self.regeneration_state.lock().unwrap();
                queue.retry = Some(self.cache_snapshot(state.as_ref().unwrap()));
            }
        }
    }

    fn cache_snapshot(&self, state: &RegenerationState) -> Snapshot {
        Snapshot {
            options: Arc::clone(&self.options),
            sources: Arc::clone(&self.source_files),
            fonts: self.fonts.clone(),
            glyphs: state.glyph_cache.clone(),
            variants: state.variant_cache.clone(),
        }
    }

    /// Measure snapshot ownership separately from encoding, scheduling, and disk I/O.
    #[doc(hidden)]
    #[cfg(feature = "bench")]
    pub fn persistence_snapshot_for_bench(&self) -> impl Sized {
        let state = self.regeneration_state.lock().unwrap();
        self.cache_snapshot(state.as_ref().unwrap())
    }

    pub(crate) fn schedule_cache(&self, state: &RegenerationState) {
        if self.defer_cache {
            return;
        }
        if let Some(persistence) = &self.persistence {
            // Geometry and font payloads are Arc-backed; only the cache indexes are copied.
            persistence.schedule(self.cache_snapshot(state));
        }
    }

    /// Wait for automatic incremental cache writes, retrying a previous failed write once.
    /// No-op without an incremental filesystem cache. Errors affect only persistence, never
    /// generated outputs. Call after awaiting rebuilds and before process shutdown; dropping a
    /// result does not wait, and abrupt process termination can lose queued writes.
    /// Replaced snapshots flush the same lineage without publishing their old font bytes.
    pub fn flush_cache(&self) -> Result<()> {
        self.persistence
            .as_ref()
            .map_or(Ok(()), |cache| cache.flush())
    }

    /// Equivalent to [`Self::flush_cache`], waiting off the async executor.
    pub async fn flush_cache_async(&self) -> Result<()> {
        let Some(cache) = self.persistence.clone() else {
            return Ok(());
        };
        tokio::task::spawn_blocking(move || cache.flush())
            .await
            .map_err(Error::other)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{webfont_fixture, write_temp_template};
    use crate::{FontType, FontVariant, GenerateWebfontsOptions, generate_sync};
    use std::sync::{Barrier, mpsc};
    use std::time::Duration;

    fn incremental_result() -> (GenerateWebfontsResult, String) {
        let directory = write_temp_template("cache-worker", "");
        std::fs::remove_file(&directory).unwrap();
        let result = generate_sync(
            GenerateWebfontsOptions {
                dest: directory.clone(),
                cache_location: Some(directory.clone()),
                files: vec![webfont_fixture("add.svg")],
                types: Some(vec![FontType::Ttf]),
                incremental: Some(true),
                write_files: Some(false),
                css: Some(false),
                ..Default::default()
            },
            None,
        )
        .unwrap();
        (result, directory)
    }

    fn snapshot(result: &GenerateWebfontsResult) -> Snapshot {
        result.cache_snapshot(result.regeneration_state.lock().unwrap().as_ref().unwrap())
    }

    #[test]
    fn refused_worker_start_retains_snapshot_for_flush_retry() {
        let (result, directory) = incremental_result();
        let cache = result.persistence.as_ref().unwrap();
        {
            let mut queue = cache.lock();
            queue.pending = Some(snapshot(&result));
            cache.start_with(&mut queue, |_| Err(Error::other("thread limit")));
            assert!(!queue.running);
            assert!(queue.pending.is_none());
            assert!(queue.retry.is_some());
            assert_eq!(queue.error.as_ref().unwrap().1, "thread limit");
        }
        cache.flush().unwrap();
        assert!(cache.lock().error.is_none());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn save_panic_invalidates_identity_and_flush_retries_the_snapshot() {
        let (result, directory) = incremental_result();
        let cache = result.persistence.as_ref().unwrap();
        let file = {
            let mut queue = cache.lock();
            let disk = queue.disk.as_ref().unwrap();
            let file = disk.dir.join(&disk.file_name);
            queue.pending = Some(snapshot(&result));
            queue.running = true;
            file
        };
        let before = std::fs::read(&file).unwrap();
        cache.run_with(|_, _| panic!("injected encode panic"));
        {
            let queue = cache.lock();
            assert!(!queue.running);
            assert!(queue.retry.is_some());
            assert!(queue.disk.as_ref().unwrap().state.loaded.is_none());
            assert!(queue.error.as_ref().unwrap().1.contains("writer panicked"));
        }
        // Rewriting identical bytes must still be attempted after a panic.
        std::fs::remove_file(&file).unwrap();
        std::fs::create_dir(&file).unwrap();
        assert!(cache.flush().is_err());
        std::fs::remove_dir(&file).unwrap();
        cache.flush().unwrap();
        assert_eq!(std::fs::read(file).unwrap(), before);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn blocked_writes_coalesce_to_latest_and_release_concurrent_flushes() {
        for fail_first in [false, true] {
            let (mut result, directory) = incremental_result();
            let cache = result.persistence.as_ref().unwrap().clone();
            // Prepare committed snapshots without letting ordinary regeneration start a worker.
            result.defer_cache = true;
            result
                .update(webfont_fixture("add.svg"), Some("a".into()))
                .unwrap();
            {
                let mut queue = cache.lock();
                queue.pending = Some(snapshot(&result));
                queue.running = true;
            }
            let (entered, blocked) = mpsc::channel();
            let (release, proceed) = mpsc::channel();
            let worker_cache = cache.clone();
            let worker = std::thread::spawn(move || {
                let mut saved = Vec::new();
                worker_cache.run_with(|snapshot, disk| {
                    saved.push(snapshot.sources[0].glyph_name.clone());
                    if saved.len() == 1 {
                        entered.send(()).unwrap();
                        proceed.recv_timeout(Duration::from_secs(10)).unwrap();
                        if fail_first {
                            return Err(Error::other("injected write failure"));
                        }
                    }
                    snapshot.save(disk)
                });
                saved
            });
            blocked.recv_timeout(Duration::from_secs(10)).unwrap();
            for name in ["b", "c"] {
                result
                    .update(webfont_fixture("add.svg"), Some(name.into()))
                    .unwrap();
                cache.schedule(snapshot(&result));
            }
            let ready = Arc::new(Barrier::new(3));
            let (done, completions) = mpsc::channel();
            let waiters: Vec<_> = (0..2)
                .map(|_| {
                    let cache = cache.clone();
                    let ready = ready.clone();
                    let done = done.clone();
                    std::thread::spawn(move || {
                        ready.wait();
                        done.send(cache.flush()).unwrap();
                    })
                })
                .collect();
            ready.wait();
            assert!(completions.try_recv().is_err());
            release.send(()).unwrap();
            for _ in 0..2 {
                completions
                    .recv_timeout(Duration::from_secs(10))
                    .unwrap()
                    .unwrap();
            }
            for waiter in waiters {
                waiter.join().unwrap();
            }
            assert_eq!(worker.join().unwrap(), ["a", "c"]);
            {
                let queue = cache.lock();
                assert!(!queue.running);
                assert!(queue.pending.is_none() && queue.retry.is_none() && queue.error.is_none());
                let disk = DiskCache::open(&directory, &result.options, &result.source_files);
                let hits = disk.font_hits(&result.options.types);
                assert_eq!(hits.len(), 1);
                assert_eq!(hits[0].bytes(), result.ttf_bytes().unwrap());
            }
            std::fs::remove_dir_all(directory).unwrap();
        }
    }

    #[test]
    fn unexpected_worker_panic_unblocks_flush_even_with_a_poisoned_queue() {
        let (result, directory) = incremental_result();
        let cache = result.persistence.as_ref().unwrap().clone();
        let broken = cache.clone();
        assert!(
            std::thread::spawn(move || {
                let mut queue = broken.lock();
                queue.disk = None;
                queue.running = true;
                panic!("inject a poisoned queue and missing worker state");
            })
            .join()
            .is_err()
        );
        let (done, completion) = mpsc::channel();
        let waiter_cache = cache.clone();
        let waiter = std::thread::spawn(move || done.send(waiter_cache.flush()).unwrap());
        cache.run(); // Panics before the per-save catch, so the outer worker guard must notify.
        let error = completion
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .unwrap_err();
        assert!(error.to_string().contains("worker panicked"));
        waiter.join().unwrap();
        cache.schedule(snapshot(&result));
        assert!(cache.flush().is_err());
        assert!(!cache.lock().running);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn retained_cache_releases_loaded_bytes_and_glyphs_without_losing_write_identity() {
        let directory = write_temp_template("cache-retention", "");
        std::fs::remove_file(&directory).unwrap();
        for family in [false, true] {
            let files = vec![webfont_fixture("add.svg")];
            let options = GenerateWebfontsOptions {
                dest: directory.clone(),
                files: if family { vec![] } else { files.clone() },
                variants: family.then(|| {
                    [400, 700]
                        .map(|weight| FontVariant {
                            name: format!("weight{weight}"),
                            files: files.clone(),
                            weight: Some(weight),
                            default: Some(weight == 400),
                        })
                        .into()
                }),
                cache_location: Some(directory.clone()),
                incremental: Some(true),
                write_files: Some(false),
                css: Some(false),
                types: Some(vec![FontType::Ttf]),
                ..Default::default()
            };
            generate_sync(options.clone(), None).unwrap();
            let result = generate_sync(options, None).unwrap();
            let cache = result.persistence.as_ref().unwrap();
            let (file, identity) = {
                let queue = cache.queue.lock().unwrap();
                let disk = queue.disk.as_ref().unwrap();
                assert!(disk.state.pending.is_none());
                assert!(disk.state.glyphs().unwrap().parsed.is_empty());
                assert!(disk.state.loaded.is_some());
                (disk.dir.join(&disk.file_name), disk.state.loaded)
            };
            let modified = std::fs::metadata(&file).unwrap().modified().unwrap();
            // Force encoding of identical hydrated glyphs: releasing decoded data must not
            // discard its validated checksum or turn this into an unnecessary disk write.
            result.schedule_cache(result.regeneration_state.lock().unwrap().as_ref().unwrap());
            result.flush_cache().unwrap();
            assert_eq!(
                std::fs::metadata(file).unwrap().modified().unwrap(),
                modified
            );
            let queue = cache.queue.lock().unwrap();
            let disk = queue.disk.as_ref().unwrap();
            assert_eq!(disk.state.loaded, identity);
            assert!(disk.state.pending.is_none());
            assert!(disk.state.glyphs().unwrap().parsed.is_empty());
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
}
