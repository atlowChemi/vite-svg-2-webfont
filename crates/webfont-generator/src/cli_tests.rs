use super::*;

#[test]
fn config_is_complete_and_excludes_explicit_generation_arguments() {
    assert!(Cli::try_parse_from(["webfont-generator", "--config", "icons.json"]).is_ok());
    let cli = Cli::try_parse_from([
        "webfont-generator",
        "--config",
        "icons.json",
        "--threads",
        "2",
    ])
    .unwrap();
    assert_eq!(cli.threads, std::num::NonZeroUsize::new(2));
    for extra in [
        vec!["./icon.svg"],
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
        vec!["--cache-location", "cache"],
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

#[test]
fn config_conflicts_with_every_generation_argument_but_threads() {
    use clap::CommandFactory;
    let mut command = Cli::command();
    command.build();
    let ids: Vec<_> = command
        .get_arguments()
        .map(|arg| arg.get_id().as_str())
        .filter(|id| !["config", "threads", "help", "version"].contains(id))
        .collect();
    assert_eq!(ids, GENERATION_ARGS);
}

#[test]
fn clear_cache_is_a_subcommand_and_generation_accepts_a_cache_location() {
    let cli = Cli::try_parse_from([
        "webfont-generator",
        "clear-cache",
        "--cache-location",
        "cache",
    ])
    .unwrap();
    assert!(
        matches!(cli.command, Some(Command::ClearCache { cache_location }) if cache_location == "cache")
    );
    assert_eq!(
        Cli::try_parse_from(["webfont-generator", "clear-cache"])
            .err()
            .unwrap()
            .kind(),
        clap::error::ErrorKind::MissingRequiredArgument
    );

    let cli = Cli::try_parse_from([
        "webfont-generator",
        "./icons",
        "-d",
        "out",
        "--cache-location",
        "cache",
    ])
    .unwrap();
    assert!(cli.command.is_none());
    assert_eq!(cli.cache_location.as_deref(), Some("cache"));

    // A directory named like the subcommand still works as an input when written as a path.
    let cli = Cli::try_parse_from(["webfont-generator", "./clear-cache", "-d", "out"]).unwrap();
    assert!(cli.command.is_none());
    assert_eq!(cli.files, ["./clear-cache"]);
}

#[test]
fn inputs_must_be_explicit_paths() {
    for input in ["./icons", "../icons", "/abs/icons", ".", "./plus.svg"] {
        let cli = Cli::try_parse_from(["webfont-generator", input, "-d", "out"]).unwrap();
        assert_eq!(cli.files, [input]);
    }
    // Bare names, even later in the list, are rejected rather than read as files.
    for args in [
        vec!["icons"],
        vec!["plus.svg"],
        vec!["icons/"],
        vec![".hidden"],
        vec!["./icons", "help"],
    ] {
        let mut argv = vec!["webfont-generator"];
        argv.extend(args);
        argv.extend(["-d", "out"]);
        let error = Cli::try_parse_from(argv).err().unwrap();
        assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
        assert!(error.to_string().contains("use `./"));
    }
}
