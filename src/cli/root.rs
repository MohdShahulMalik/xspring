use crate::cli::commands::Commands;
use clap::Parser;
use std::path::PathBuf;
use clap_verbosity_flag::Verbosity;

#[derive(Parser, Debug)]
#[command(name = "xspring")]
#[command(version = "1.0")]
#[command(about = "An interactive Spring Boot project generator")]
#[command(author = "Shahul Malik")]
pub struct Cli{
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short = 'o', long, global = true)]
    pub output: Option<PathBuf>,

    #[command(flatten)]
    pub verbose: Verbosity,
}
