use std::io::{BufWriter, Write};

use anyhow::{anyhow, Context, Result};
use clap::parser::Values;
use crate::{client::spring_initializr::get_metadata, models::spring::{Value, DependencyCategories} };

pub enum Lists {
    Values(Vec<Value>),
    Categories(Vec<DependencyCategories>),
}

pub async fn get_versions(item: &str) -> Result<Lists>{
    let spring_metadata = get_metadata().await
        .with_context(|| "Failed to get metadata for listing out versions")?;

    match item {
        "java" => Ok(Lists::Values(spring_metadata.java_version.values)),
        "boot" => Ok(Lists::Values(spring_metadata.boot_version.values)),
        "type" => Ok(Lists::Values(spring_metadata.project_type.values)),
        "language" => Ok(Lists::Values(spring_metadata.language.values)),
        "deps" => Ok(Lists::Categories(spring_metadata.dependencies.values)),
        _ => Err(anyhow!("Unknown item '{}'", item))
    }
}

pub fn print_versions<W: Write>(buf: &mut BufWriter<W>, versions: Lists) -> Result<()>{
    if let Lists::Values(vers) = versions {
        for version in vers {
            writeln!(buf, "{}", version)?;
        }
    }

    Ok(())
}
