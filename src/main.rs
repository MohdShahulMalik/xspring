use std::io::{stdout, BufWriter, Write};

use clap::Parser;
use anyhow::{Context, Result};
use xspring::cli::commands::Commands;
use xspring::cli::root::Cli;
use xspring::handlers::interactive::{pure_interactivity, quick_interactivity};
use tracing_appender::rolling;
use xspring::handlers::list::{get_versions, print_values};
use xspring::models::list::Lists;

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

    if let Some(command) = cli.command {

        let stdout = stdout();
        let handle = stdout.lock();
        let buf = BufWriter::new(handle);

        match command {
           Commands::Quick {maven, extended} => {
               quick_interactivity(maven, extended);
           }

           Commands::List { java: true, .. } => {
                let java_versions: Lists = get_versions("java").await
                    .with_context(|| "Failed to get java versions")?;  
                writeln!(buf, "Available Java Versions:");

                print_values(&mut buf, java_versions)
                 .with_context(|| "Failed to print java versions")?;
                
           }

           Commands::List { boot: true, .. } => {
               let boot_versions: Lists = get_versions("boot").await
                   .with_context(|| "Failed to get boot version")?;
               writeln!(buf, "Available Spring Boot Versions:");

               print_values(&mut buf, boot_versions)
                   .with_context(|| "Failed to print boot versions")?;
           }

           Commands::List { project_type: true, .. } => {
               let project_types: Lists = get_versions("project_type").await
                   .with_context(|| "Failed to get project types")?;
               writeln!(buf, "Available Project Types:")?;

               print_values(&mut buf, project_types)
                   .with_context(|| "Failed to print project types")?;
           }

           Commands::List { language: true, .. } => {
               let languages = get_versions("language").await
                   .with_context(|| "Failed to get languages")?;
               writeln!(buf, "Available Languages:")?;

               print_values(&mut buf, languages)
                   .with_context(|| "Failed to print languages")?;
           }

        }
    }
    
    pure_interactivity().await?;
    Ok(())
}
