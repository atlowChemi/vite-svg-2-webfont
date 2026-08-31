use std::error::Error;

use crate::input::{finalize_generate_webfonts_options, resolve_generate_webfonts_options};
use crate::pipeline::generate_webfonts_sync;
use crate::test_helpers::{fixture_source_files, webfont_fixture};
use crate::types::{FontType, FormatOptions, GenerateWebfontsOptions, TtfFormatOptions};

use super::{GenerateWebfontsResult, RegenerateError};

pub(super) fn incremental_result() -> GenerateWebfontsResult {
    let mut options = resolve_generate_webfonts_options(GenerateWebfontsOptions {
        css: Some(true),
        dest: "artifacts".to_owned(),
        files: vec![webfont_fixture("add.svg")],
        font_name: Some("result-test".to_owned()),
        format_options: Some(FormatOptions {
            ttf: Some(TtfFormatOptions {
                copyright: None,
                description: None,
                ts: Some(1_700_000_000),
                url: None,
                version: None,
            }),
            ..Default::default()
        }),
        html: Some(true),
        incremental: Some(true),
        ligature: Some(false),
        types: Some(vec![
            FontType::Svg,
            FontType::Ttf,
            FontType::Eot,
            FontType::Woff,
            FontType::Woff2,
        ]),
        write_files: Some(false),
        ..Default::default()
    })
    .unwrap();
    let source_files = fixture_source_files(&options);
    finalize_generate_webfonts_options(&mut options, &source_files).unwrap();
    generate_webfonts_sync(options, source_files).unwrap()
}

#[test]
fn regenerate_error_exposes_recoverable_result_and_source() {
    let error = RegenerateError::new(
        Some(incremental_result()),
        std::io::Error::new(std::io::ErrorKind::InvalidData, "regeneration failed"),
    );

    assert_eq!(error.to_string(), "regeneration failed");
    assert_eq!(error.source().unwrap().to_string(), "regeneration failed");
    assert!(format!("{error:?}").contains("recoverable: true"));

    let (result, source) = error.into_parts();
    assert!(result.is_some());
    assert_eq!(source.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn regenerate_error_can_be_unrecoverable() {
    let error = RegenerateError::new(None, std::io::Error::other("task cancelled"));

    assert!(format!("{error:?}").contains("recoverable: false"));
    assert!(error.into_result().is_none());
}
