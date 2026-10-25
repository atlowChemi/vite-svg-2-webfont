//! The `threads` option: how many threads read SVG files and do glyph work at once.

use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::available_parallelism;

use rayon::{ThreadPool, ThreadPoolBuilder};

/// Pools by size (at most one per core count), kept for the life of the process so repeated
/// generations and rebuilds reuse their threads. `None` marks a size whose threads the OS refused.
static POOLS: Mutex<BTreeMap<NonZeroUsize, Option<Arc<ThreadPool>>>> = Mutex::new(BTreeMap::new());

/// How many files may be read at once: `threads`, or one per core when unset. Reads wait on
/// storage, so values above the core count still raise concurrency (slow or network disks).
#[cfg(feature = "fs")]
pub(crate) fn limit(threads: Option<NonZeroUsize>) -> usize {
    threads.map_or_else(
        || available_parallelism().map_or(1, usize::from),
        usize::from,
    )
}

/// Glyph work is CPU-bound: threads beyond the core count only add switching.
fn pool_size(threads: NonZeroUsize) -> NonZeroUsize {
    available_parallelism().map_or(threads, |cores| threads.min(cores))
}

/// Run `work` with Rayon parallelism capped at `threads` (and the core count), or on Rayon's global
/// pool when unset.
pub(crate) fn install<R: Send>(
    threads: Option<NonZeroUsize>,
    work: impl FnOnce() -> R + Send,
) -> R {
    let Some(size) = threads.map(pool_size) else {
        return work();
    };
    run_in(cached_pool(&POOLS, size, build_pool), work)
}

fn build_pool(size: NonZeroUsize) -> Option<ThreadPool> {
    ThreadPoolBuilder::new()
        .num_threads(size.get())
        .thread_name(move |index| format!("webfont-{size}-{index}"))
        .build()
        .ok()
}

/// The pool for `size` from `pools`, built on first use. A failed build is remembered, so it
/// warns once and isn't retried.
fn cached_pool(
    pools: &Mutex<BTreeMap<NonZeroUsize, Option<Arc<ThreadPool>>>>,
    size: NonZeroUsize,
    build: impl FnOnce(NonZeroUsize) -> Option<ThreadPool>,
) -> Option<Arc<ThreadPool>> {
    let cached = pools
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&size)
        .cloned();
    cached.unwrap_or_else(|| {
        // Built outside the lock: starting threads takes time, and other sizes shouldn't wait.
        let built = build(size).map(Arc::new);
        if built.is_none() {
            eprintln!(
                "warning: webfont-generator: couldn't start {size} threads, so `threads: {size}` runs on the shared thread pool in this process"
            );
        }
        pools
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(size)
            .or_insert(built)
            .clone()
    })
}

/// Output is the same either way; only the cap is lost when the OS refused the threads.
fn run_in<R: Send>(pool: Option<Arc<ThreadPool>>, work: impl FnOnce() -> R + Send) -> R {
    match pool {
        Some(pool) => pool.install(work),
        None => work(),
    }
}

/// Glyph parses that ran in the single-thread pool. Only `tests::every_entry_point_runs_in_the_pool`
/// uses that size, so a missing `install` on any entry point leaves the count unchanged.
#[cfg(test)]
pub(crate) static SINGLE_THREAD_PARSES: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[cfg(test)]
pub(crate) fn record_parse() {
    if std::thread::current().name() == Some("webfont-1-0") {
        SINGLE_THREAD_PARSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cores() -> usize {
        available_parallelism().unwrap().get()
    }

    #[test]
    fn runs_inside_a_reused_pool_of_the_requested_size() {
        let two = NonZeroUsize::new(2);
        let pool = || {
            let pools = POOLS.lock().unwrap();
            Arc::as_ptr(pools[&pool_size(two.unwrap())].as_ref().unwrap())
        };
        assert_eq!(install(two, rayon::current_num_threads), 2.min(cores()));
        let first = pool();
        assert_eq!(
            install(NonZeroUsize::new(3), rayon::current_num_threads),
            3.min(cores())
        );
        install(two, || ());
        assert_eq!(pool(), first);
    }

    #[test]
    fn glyph_pools_stop_at_the_core_count() {
        let many = NonZeroUsize::new(cores() + 5);
        assert_eq!(install(many, rayon::current_num_threads), cores());
    }

    #[test]
    fn a_refused_pool_warns_once_and_runs_on_the_caller() {
        // A local map: marking a size as refused in `POOLS` would affect other tests.
        let pools = Mutex::new(BTreeMap::new());
        let size = NonZeroUsize::new(2).unwrap();
        let mut builds = 0;
        assert!(
            cached_pool(&pools, size, |_| {
                builds += 1;
                None
            })
            .is_none()
        );
        assert!(
            cached_pool(&pools, size, |_| {
                builds += 1;
                None
            })
            .is_none()
        );
        assert_eq!(builds, 1, "a refused size isn't retried");
        let caller = std::thread::current().id();
        assert_eq!(run_in(None, || std::thread::current().id()), caller);
    }

    #[test]
    fn unset_keeps_the_global_pool() {
        assert_eq!(
            install(None, rayon::current_num_threads),
            rayon::current_num_threads()
        );
    }

    #[test]
    #[cfg(feature = "fs")]
    fn reads_follow_the_requested_value_even_above_the_core_count() {
        assert_eq!(limit(None), available_parallelism().map_or(1, usize::from));
        assert_eq!(limit(NonZeroUsize::new(5)), 5);
        assert_eq!(limit(NonZeroUsize::new(cores() + 5)), cores() + 5);
    }

    /// Each entry point runs glyph work inside the pool: without `install`, parses run on the caller
    /// or the global pool and the single-thread pool's count doesn't move.
    #[test]
    #[cfg(all(feature = "fs", feature = "in_memory"))]
    fn every_entry_point_runs_in_the_pool() {
        use crate::{
            FontType, FontVariant, GenerateWebfontsFromSourcesOptions, GenerateWebfontsOptions,
            GlyphChange, RegenerationFiles, SvgSource, VariantFileSet,
        };
        use std::sync::atomic::Ordering::Relaxed;

        struct Folder(std::path::PathBuf);
        impl Drop for Folder {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let folder = Folder(
            std::env::temp_dir().join(format!("webfont-threads-paths-{}", std::process::id())),
        );
        std::fs::create_dir_all(&folder.0).unwrap();
        let svg = |width: u32| {
            format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M0 0H{width}V24H0Z"/></svg>"#
            )
        };
        let files: Vec<String> = ["a", "b"]
            .iter()
            .map(|name| {
                let path = folder.0.join(format!("{name}.svg"));
                std::fs::write(&path, svg(10)).unwrap();
                path.to_string_lossy().into_owned()
            })
            .collect();
        let one = NonZeroUsize::new(1);
        let parses_in_pool = |step: &str, run: &mut dyn FnMut()| {
            let before = SINGLE_THREAD_PARSES.load(Relaxed);
            run();
            assert!(SINGLE_THREAD_PARSES.load(Relaxed) > before, "{step}");
        };
        let options = || GenerateWebfontsOptions {
            css: Some(false),
            dest: folder.0.to_string_lossy().into_owned(),
            files: files.clone(),
            incremental: Some(true),
            threads: one,
            types: Some(vec![FontType::Ttf]),
            write_files: Some(false),
            ..Default::default()
        };
        let edit = |width| std::fs::write(&files[1], svg(width)).unwrap();
        let changed = [(files[1].clone(), GlyphChange::Changed { name: None })];

        let mut result = None;
        parses_in_pool("generate", &mut || {
            result = Some(crate::generate_sync(options(), None).unwrap());
        });
        let mut result = result.unwrap();
        parses_in_pool("regenerate", &mut || {
            edit(12);
            let single = RegenerationFiles::Single(files.clone());
            result.regenerate(&single, &changed).unwrap();
        });
        parses_in_pool("regenerate_all", &mut || {
            edit(14);
            let single = RegenerationFiles::Single(files.clone());
            result.regenerate_all(&single).unwrap();
        });

        let variant = |name: &str, weight, default| FontVariant {
            name: name.to_owned(),
            files: files.clone(),
            weight: Some(weight),
            default,
        };
        let mut family = None;
        parses_in_pool("variant generate", &mut || {
            let options = GenerateWebfontsOptions {
                files: vec![],
                types: Some(vec![FontType::Woff2]),
                variants: Some(vec![
                    variant("light", 300, Some(true)),
                    variant("bold", 700, None),
                ]),
                ..options()
            };
            family = Some(crate::generate_sync(options, None).unwrap());
        });
        let mut family = family.unwrap();
        parses_in_pool("variant regenerate", &mut || {
            edit(16);
            let sets = ["light", "bold"].map(|name| VariantFileSet {
                variant: name.to_owned(),
                files: files.clone(),
            });
            family
                .regenerate(&RegenerationFiles::Variants(sets.into()), &changed)
                .unwrap();
        });
        parses_in_pool("variant regenerate_all", &mut || {
            edit(18);
            let sets = ["light", "bold"].map(|name| VariantFileSet {
                variant: name.to_owned(),
                files: files.clone(),
            });
            family
                .regenerate_all(&RegenerationFiles::Variants(sets.into()))
                .unwrap();
        });

        let source = |svg: String| SvgSource {
            id: "icon".to_owned(),
            name: "icon".to_owned(),
            svg,
        };
        let mut sources = None;
        parses_in_pool("in-memory generate", &mut || {
            sources = Some(
                crate::generate_from_sources_sync(GenerateWebfontsFromSourcesOptions {
                    sources: vec![source(svg(10))],
                    incremental: Some(true),
                    threads: one,
                    types: Some(vec![FontType::Ttf]),
                    ..Default::default()
                })
                .unwrap(),
            );
        });
        let mut sources = sources.unwrap();
        parses_in_pool("in-memory update", &mut || {
            sources
                .update_sync("icon".to_owned(), svg(12), None)
                .unwrap();
        });
    }
}
