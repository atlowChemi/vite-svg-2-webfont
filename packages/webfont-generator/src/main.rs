use std::path::Path;
use std::process::ExitCode;

use clap::{ArgAction, Parser, builder::styling};
use webfont_generator::{FontType, GenerateWebfontsOptions};

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
    about = "Generate webfonts from SVG icons"
)]
struct Cli {
    /// SVG files or directories containing SVG files
    #[arg(required = true)]
    files: Vec<String>,

    /// Output directory
    #[arg(short, long)]
    dest: String,

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
        css: Some(!cli.no_css),
        css_template: cli.css_template,
        css_fonts_url: cli.css_fonts_url,
        descent: cli.descent,
        dest: cli.dest,
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
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}
