use super::*;

#[test]
fn config_is_complete_and_excludes_explicit_generation_arguments() {
    assert!(Cli::try_parse_from(["webfont-generator", "--config", "icons.json"]).is_ok());
    for extra in [
        vec!["icon.svg"],
        vec!["--dest", "out"],
        vec!["--font-name", "iconfont"],
        vec!["--css"],
        vec!["--no-css"],
        vec!["--types", "woff2"],
        vec!["--no-write"],
        vec!["--write"],
        vec!["--html"],
        vec!["--no-html"],
        vec!["--ligature"],
        vec!["--no-ligature"],
        vec!["--css-template", "css.hbs"],
        vec!["--html-template", "html.hbs"],
        vec!["--css-fonts-url", "/fonts"],
        vec!["--font-height", "1000"],
        vec!["--ascent", "800"],
        vec!["--descent", "200"],
        vec!["--start-codepoint", "0xF101"],
    ] {
        let mut args = vec!["webfont-generator", "--config", "icons.json"];
        args.extend(extra);
        assert_eq!(
            Cli::try_parse_from(args).err().unwrap().kind(),
            clap::error::ErrorKind::ArgumentConflict
        );
    }
    for (flag, kind) in [
        ("--help", clap::error::ErrorKind::DisplayHelp),
        ("--version", clap::error::ErrorKind::DisplayVersion),
    ] {
        assert_eq!(
            Cli::try_parse_from(["webfont-generator", "--config", "icons.json", flag])
                .err()
                .unwrap()
                .kind(),
            kind
        );
    }
}
