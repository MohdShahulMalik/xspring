use std::io::{stdout, BufWriter, Write};

use clap::Parser;
use anyhow::{anyhow, Context, Result};
use xspring::cli::commands::Commands;
use xspring::cli::root::Cli;
use xspring::handlers::interactive::{pure_interactivity, quick_interactivity};
use tracing_appender::rolling;
use xspring::handlers::list::{get_lists, print_categories, print_values};
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
        let mut buf = BufWriter::new(handle);

        match command {
           Commands::Quick {maven, extended} => {
               quick_interactivity(maven, extended).await
                   .with_context(|| "Failed to run quick interactivity")?;
           }

           Commands::List { java: true, .. } => {
                let java_versions: Lists = get_lists("java").await
                    .with_context(|| "Failed to get java versions")?;  
                writeln!(&mut buf, "Available Java Versions:")?;

                print_values(&mut buf, java_versions)
                 .with_context(|| "Failed to print java versions")?;
                
           }

           Commands::List { boot: true, .. } => {
               let boot_versions: Lists = get_lists("boot").await
                   .with_context(|| "Failed to get boot version")?;
               writeln!(&mut buf, "Available Spring Boot Versions:")?;

               print_values(&mut buf, boot_versions)
                   .with_context(|| "Failed to print boot versions")?;
           }

           Commands::List { project_type: true, .. } => {
               let project_types: Lists = get_lists("project_type").await
                   .with_context(|| "Failed to get project types")?;
               writeln!(&mut buf, "Available Project Types:")?;

               print_values(&mut buf, project_types)
                   .with_context(|| "Failed to print project types")?;
           }

           Commands::List { language: true, .. } => {
               let languages = get_lists("language").await
                   .with_context(|| "Failed to get languages")?;
               writeln!(&mut buf, "Available Languages:")?;

               print_values(&mut buf, languages)
                   .with_context(|| "Failed to print languages")?;
           }

           Commands::List { deps: true, ..  } => {
               let categories = get_lists("deps").await
                   .with_context(|| "Failed to get dependencies")?;
               writeln!(&mut buf, "Available Dependencies:-")?;
               writeln!(&mut buf, " ")?;

               print_categories(&mut buf, categories)
                   .with_context(|| "Failed to print Dependencies with Categories")?;
           }

           _ => {
               return Err(anyhow!("Invalid or unimplemented command. Please use '--help' or '-h' to see available commands."));
           }
        }

    } else {
        pure_interactivity().await
            .with_context(|| "Failed to run pure interactivity")?;
    }
    
    Ok(())
}
