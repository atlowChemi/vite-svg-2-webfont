How many threads read SVG files and do glyph work at once, including in later updates of the result. Set it when several generations run side by side, such as in a monorepo build, or to leave cores free for other work. Output is identical for every value.

- Glyph work never uses more threads than CPU cores: it keeps every thread busy, so more would only add switching. Reads can go higher, which can help on slow or network storage.
- Not counted: WOFF2's Brotli compressor runs one extra worker thread, output files are written concurrently, the async runtime keeps its own threads, and each incremental cache lineage runs at most one background writer while saves are pending.
- Unset, files are read with up to one task per core, and glyph work runs on [Rayon](https://github.com/rayon-rs/rayon)'s shared thread pool, so the `RAYON_NUM_THREADS` environment variable still applies.
- Each value gets its own thread pool, kept for the life of the process so later generations and updates reuse its threads. If the system refuses to start them, generation prints a warning and runs on the shared pool instead.
