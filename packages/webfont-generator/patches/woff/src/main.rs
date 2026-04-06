use std::fs::{read, write};
use std::path::PathBuf;

#[allow(unused_variables)]
fn main() {
    let arguments::Arguments {
        options, orphans, ..
    } = arguments::parse(std::env::args()).expect("failed to parse arguments");
    #[allow(clippy::get_first)]
    let source = match orphans.get(0) {
        Some(value) => PathBuf::from(value),
        _ => usage(),
    };
    let destination = match orphans.get(1) {
        Some(value) => PathBuf::from(value),
        _ => usage(),
    };
    #[allow(unused_mut)]
    let mut data = read(&source).expect("failed to read the source");
    match (
        source.extension().and_then(|value| value.to_str()),
        destination.extension().and_then(|value| value.to_str()),
    ) {
        #[cfg(feature = "version1")]
        (_, Some("woff")) => {
            data = woff::version1::compress(
                &data,
                options.get::<usize>("major-version").unwrap_or(1),
                options.get::<usize>("minor-version").unwrap_or(0),
            )
            .expect("failed to compress");
        }
        #[cfg(feature = "version1")]
        (Some("woff"), _) => {
            data = woff::version1::decompress(&data).expect("failed to decompress");
        }
        #[cfg(feature = "version2")]
        (_, Some("woff2")) => {
            data = woff::version2::compress(
                &data,
                options.get::<String>("metadata").unwrap_or_default(),
                options.get::<usize>("quality").unwrap_or(8),
                options.get::<bool>("transform").unwrap_or(true),
            )
            .expect("failed to compress");
        }
        #[cfg(feature = "version2")]
        (Some("woff2"), _) => {
            data = woff::version2::decompress(&data).expect("failed to decompress");
        }
        _ => usage(),
    }
    #[allow(unreachable_code)]
    write(destination, data).expect("failed to write to the destination");
}

fn usage() -> ! {
    eprintln!(
        r#"Usage: woff <source> <destination> [options]

Either the source or destination should end with either .woff or .woff2.

Options for WOFF:
    --major-version <number> — set the major version (1 by default)
    --minor-version <number> — set the minor version (0 by default)

Options for WOFF2:
    --metadata <string> — append metadata (empty by default)
    --quality <number>  — set the compression quality (8 by default)
    --no-transform      — disallow transforms"#
    );
    std::process::exit(1);
}
