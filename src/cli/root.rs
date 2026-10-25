use crate::cli::commands::Commands;
use clap::Parser;
use clap_verbosity_flag::Verbosity;
use std::ffi::OsString;
use std::path::PathBuf;

/// The shorthand for `--no-git`.
const NO_GIT_SHORT: &str = "-ng";

#[derive(Parser, Debug)]
#[command(name = "xspring")]
#[command(version)]
#[command(about = "An interactive Spring Boot project generator")]
#[command(author = "Shahul Malik")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short = 'o', long, global = true)]
    pub output: Option<PathBuf>,

    /// Skip the git repository that is normally created for a new project.
    ///
    /// The shorthand is `-ng`.
    #[arg(long, global = true)]
    pub no_git: bool,

    #[command(flatten)]
    pub verbose: Verbosity,
}

/// Expands the `-ng` shorthand into `--no-git`.
///
/// Clap accepts single-character short flags only, so a two-character
/// shorthand has to be rewritten before the arguments reach the parser.
pub fn expand_aliases(args: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    args.into_iter()
        .map(|arg| {
            if arg == NO_GIT_SHORT {
                OsString::from("--no-git")
            } else {
                arg
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::expand_aliases;

    fn expand(args: &[&str]) -> Vec<String> {
        expand_aliases(args.iter().map(OsString::from))
            .into_iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn the_shorthand_becomes_the_long_flag() {
        assert_eq!(expand(&["xspring", "-ng"]), ["xspring", "--no-git"]);
    }

    #[test]
    fn other_arguments_are_left_alone() {
        assert_eq!(
            expand(&["xspring", "quick", "-ng", "-o", "out"]),
            ["xspring", "quick", "--no-git", "-o", "out"]
        );
    }

    #[test]
    fn a_longer_flag_that_merely_starts_the_same_way_is_not_touched() {
        assert_eq!(
            expand(&["xspring", "--no-git-extra"]),
            ["xspring", "--no-git-extra"]
        );
        assert_eq!(expand(&["xspring", "-ngx"]), ["xspring", "-ngx"]);
    }
}
