use std::path::{Component, Path};
use std::process::ExitCode;

use clap::{ArgAction, Parser, Subcommand, builder::styling};
use webfont_generator::{FontType, GenerateWebfontsOptions};
mod manifest;

#[cfg(test)]
mod cli_tests;

const STYLES: styling::Styles = styling::Styles::styled()
    .header(styling::AnsiColor::Green.on_default().bold())
    .usage(styling::AnsiColor::Green.on_default().bold())
    .literal(styling::AnsiColor::Cyan.on_default().bold())
    .placeholder(styling::AnsiColor::White.on_default());

#[derive(Parser)]
#[command(
    name = "webfont-generator",
    version,
    styles = STYLES,
    about = "Generate webfonts from SVG icons",
    args_conflicts_with_subcommands = true,
    after_help = "Inputs are explicit paths (./icons, ../icons, /abs/icons); bare names are reserved for subcommands."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Complete JSON configuration; paths are relative to this file
    #[arg(long, exclusive = true)]
    config: Option<String>,
    /// SVG files or directories containing SVG files, as explicit paths (`./icons`)
    #[arg(required_unless_present = "config", value_parser = explicit_path)]
    files: Vec<String>,

    /// Output directory
    #[arg(short, long, required_unless_present = "config")]
    dest: Option<String>,

    /// Font name
    #[arg(short = 'n', long, default_value = "iconfont")]
    font_name: String,

    /// Font types to generate
    #[arg(short, long, value_delimiter = ',')]
    types: Option<Vec<FontType>>,

    /// Generate CSS (default)
    #[arg(long, overrides_with = "no_css", action = ArgAction::SetTrue)]
    css: bool,

    /// Skip CSS generation
    #[arg(long = "no-css", overrides_with = "css", action = ArgAction::SetTrue)]
    no_css: bool,

    /// Generate HTML preview
    #[arg(long, overrides_with = "no_html", action = ArgAction::SetTrue)]
    html: bool,

    /// Skip HTML generation (default)
    #[arg(long = "no-html", overrides_with = "html", action = ArgAction::SetTrue)]
    no_html: bool,

    /// Custom CSS template path
    #[arg(long)]
    css_template: Option<String>,

    /// Custom HTML template path
    #[arg(long)]
    html_template: Option<String>,

    /// CSS fonts URL prefix
    #[arg(long)]
    css_fonts_url: Option<String>,

    /// Write output files to disk (default)
    #[arg(long, overrides_with = "no_write", action = ArgAction::SetTrue)]
    write: bool,

    /// Do not write output files (dry run)
    #[arg(long = "no-write", overrides_with = "write", action = ArgAction::SetTrue)]
    no_write: bool,

    /// Enable ligatures (default)
    #[arg(long, overrides_with = "no_ligature", action = ArgAction::SetTrue)]
    ligature: bool,

    /// Disable ligatures
    #[arg(long = "no-ligature", overrides_with = "ligature", action = ArgAction::SetTrue)]
    no_ligature: bool,

    /// Font height
    #[arg(long)]
    font_height: Option<f64>,

    /// Ascent value
    #[arg(long)]
    ascent: Option<f64>,

    /// Descent value
    #[arg(long)]
    descent: Option<f64>,

    /// Start codepoint (hex, e.g. 0xF101)
    #[arg(long)]
    start_codepoint: Option<String>,

    /// Directory for the persistent generation cache; later runs reuse unchanged work
    #[arg(long)]
    cache_location: Option<String>,
}

#[derive(Subcommand)]
enum Command {
    /// Remove the files the generation cache created in a directory
    ClearCache {
        /// Cache directory passed to `--cache-location` (or `cacheLocation`) when generating
        #[arg(long)]
        cache_location: String,
    },
}

/// Accept inputs that start with `.`, `..`, a root, or a Windows prefix. Bare names stay free for
/// subcommands, so adding one never changes what an existing command line means.
fn explicit_path(value: &str) -> Result<String, String> {
    match Path::new(value).components().next() {
        Some(
            Component::CurDir | Component::ParentDir | Component::RootDir | Component::Prefix(_),
        ) => Ok(value.to_owned()),
        _ => Err(format!(
            "inputs must be explicit paths so they can't be mistaken for subcommands; use `./{value}`"
        )),
    }
}

fn collect_svg_files(paths: &[String]) -> std::io::Result<Vec<String>> {
    let mut result = Vec::new();
    for path in paths {
        let p = Path::new(path);
        if p.is_dir() {
            let mut dir_files: Vec<String> = std::fs::read_dir(p)?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("svg"))
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            dir_files.sort();
            result.extend(dir_files);
        } else {
            result.push(path.clone());
        }
    }
    Ok(result)
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Some(Command::ClearCache { cache_location }) = &cli.command {
        return match webfont_generator::clear_cache(cache_location) {
            Ok(report) => {
                println!(
                    "Removed {} cache file(s), {} bytes.",
                    report.files_removed, report.bytes_removed
                );
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("Error: {cache_location}: {error}");
                ExitCode::FAILURE
            }
        };
    }

    if let Some(path) = &cli.config {
        return match manifest::load(Path::new(path)) {
            Ok(options) => generate(options, Some(path)),
            Err(error) => {
                eprintln!("Error: {path}: {error}");
                ExitCode::FAILURE
            }
        };
    }

    let files = match collect_svg_files(&cli.files) {
        Ok(files) => files,
        Err(error) => {
            eprintln!("Error: failed to read input path: {error}");
            return ExitCode::FAILURE;
        }
    };
    if files.is_empty() {
        eprintln!("Error: no SVG files found in the specified paths.");
        return ExitCode::FAILURE;
    }

    let start_codepoint = cli.start_codepoint.and_then(|s| {
        let s = s.trim();
        if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
            u32::from_str_radix(hex, 16).ok()
        } else {
            s.parse::<u32>().ok()
        }
    });

    // For --flag / --no-flag pairs: when neither is passed, both are false,
    // so !no_flag = true (matching the library defaults). When one is passed,
    // overrides_with ensures only the last one is active.
    let options = GenerateWebfontsOptions {
        ascent: cli.ascent,
        cache_location: cli.cache_location,
        css: Some(!cli.no_css),
        css_template: cli.css_template,
        css_fonts_url: cli.css_fonts_url,
        descent: cli.descent,
        dest: cli.dest.unwrap(),
        files,
        font_height: cli.font_height,
        font_name: Some(cli.font_name),
        html: Some(cli.html && !cli.no_html),
        html_template: cli.html_template,
        ligature: Some(!cli.no_ligature),
        start_codepoint,
        types: cli.types,
        write_files: Some(!cli.no_write),
        ..Default::default()
    };

    generate(options, None)
}

fn generate(options: GenerateWebfontsOptions, manifest: Option<&str>) -> ExitCode {
    match webfont_generator::generate_sync(options, None) {
        Ok(result) => {
            let mut generated = Vec::new();
            if result.svg_string().is_some() {
                generated.push("SVG");
            }
            if result.ttf_bytes().is_some() {
                generated.push("TTF");
            }
            if result.eot_bytes().is_some() {
                generated.push("EOT");
            }
            if result.woff_bytes().is_some() {
                generated.push("WOFF");
            }
            if result.woff2_bytes().is_some() {
                generated.push("WOFF2");
            }
            println!("Generated: {}", generated.join(", "));
            ExitCode::SUCCESS
        }
        Err(error) => {
            if let Some(path) = manifest {
                eprintln!("Error: {path}: {error}");
            } else {
                eprintln!("Error: {error}");
            }
            ExitCode::FAILURE
        }
    }
}
