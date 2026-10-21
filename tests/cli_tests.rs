//! Tests for the command line surface.

use std::ffi::OsString;

use clap::Parser;
use xspring::cli::root::{Cli, expand_aliases};

/// Parses a command line the way `main` does.
fn parse(args: &[&str]) -> Cli {
    let args = expand_aliases(args.iter().map(OsString::from));
    Cli::parse_from(args)
}

#[test]
fn the_shorthand_turns_off_the_git_repository() {
    let cli = parse(&["xspring", "-ng"]);

    assert!(cli.no_git);
}

#[test]
fn the_long_flag_turns_off_the_git_repository() {
    let cli = parse(&["xspring", "--no-git"]);

    assert!(cli.no_git);
}

#[test]
fn the_git_repository_is_on_by_default() {
    let cli = parse(&["xspring"]);

    assert!(!cli.no_git);
}

#[test]
fn the_shorthand_works_after_a_subcommand() {
    let cli = parse(&["xspring", "quick", "-ng"]);

    assert!(cli.no_git);
    assert!(matches!(
        cli.command,
        Some(xspring::cli::commands::Commands::Quick { .. })
    ));
}

#[test]
fn the_shorthand_does_not_swallow_the_other_flags() {
    let cli = parse(&["xspring", "quick", "-ng", "-m", "-e"]);

    assert!(cli.no_git);

    let Some(xspring::cli::commands::Commands::Quick {
        maven, extended, ..
    }) = cli.command
    else {
        panic!("expected the quick command");
    };

    assert!(maven);
    assert!(extended);
}
