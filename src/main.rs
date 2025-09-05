use clap::Parser;
use anyhow::Result;
use xspring::cli::root::Cli;
use xspring::handlers::interactive::pure_interactivity;
use tracing_appender::rolling;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let logfile = rolling::daily("logs", "xspring.log");
    let level = cli.verbose.tracing_level_filter();
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_writer(logfile)
        .with_ansi(true)
        .init();

    pure_interactivity().await?;
    Ok(())
}
