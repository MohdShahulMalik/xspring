use std::io::{BufWriter, Write}; 
use anyhow::{anyhow, Context, Result};
use crate::{client::spring_initializr::get_metadata, models::list::Lists::{self, Categories, Values}};

pub async fn get_versions(item: &str) -> Result<Lists>{
    let spring_metadata = get_metadata().await
        .with_context(|| "Failed to get metadata for listing out versions")?;

    match item {
        "java" => Ok(Values(spring_metadata.java_version.values)),
        "boot" => Ok(Values(spring_metadata.boot_version.values)),
        "project_type" => Ok(Values(spring_metadata.project_type.values)),
        "language" => Ok(Values(spring_metadata.language.values)),
        "deps" => Ok(Categories(spring_metadata.dependencies.values)),
        _ => Err(anyhow!("Unknown item '{}'", item))
    }
}

pub fn print_values<W: Write>(buf: &mut BufWriter<W>, versions: Lists) -> Result<()>{
    if let Values(vers) = versions {
        for version in vers {
            writeln!(buf, "{}", version)?;
        }
    }else {
        return Err(anyhow!("Provided 'Lists' enum type is not a 'Values' variant"));
    }

    Ok(())
}

pub fn print_categories<W: Write>(buf: &mut BufWriter<W>, categories: Lists) -> Result<()> {
    if let Categories(cats) = categories {
        for category in cats {
            writeln!(buf, "{}:", category.name)?;
            for dep in category.values {
                writeln!(buf, "-{}", dep)?;
            }
            writeln!(buf, " ")?;
        }
    }else {
        return Err(anyhow!("Provided 'Lists' enum type is not a 'Categories' variant"));
    }

    Ok(())
}
