use crate::{
    client::spring_initializr::get_metadata,
    models::list::Lists::{self, Categories, Values},
};
use anyhow::{Context, Result, anyhow};
use std::io::{BufWriter, Write};
use tracing::debug;

pub async fn get_lists(item: &str) -> Result<Lists> {
    debug!("list: fetching metadata for '{}'", item);

    let spring_metadata = get_metadata()
        .await
        .with_context(|| "Failed to get metadata for listing out versions")?;

    match item {
        "java" => Ok(Values(spring_metadata.java_version.values)),
        "boot" => Ok(Values(spring_metadata.boot_version.values)),
        "project_type" => Ok(Values(spring_metadata.project_type.values)),
        "language" => Ok(Values(spring_metadata.language.values)),
        "deps" => Ok(Categories(spring_metadata.dependencies.values)),
        _ => Err(anyhow!("Unknown item '{}'", item)),
    }
}

pub fn print_values<W: Write>(buf: &mut BufWriter<W>, versions: Lists) -> Result<()> {
    if let Values(vers) = versions {
        for version in vers {
            writeln!(buf, "{}", version)?;
        }
    } else {
        return Err(anyhow!(
            "Provided 'Lists' enum type is not a 'Values' variant"
        ));
    }

    Ok(())
}

pub fn print_categories<W: Write>(
    buf: &mut BufWriter<W>,
    categories: Lists,
    search: Option<&str>,
) -> Result<()> {
    if let Categories(cats) = categories {
        let query = search.map(|s| s.trim().to_lowercase());
        let mut shown = 0usize;

        for category in cats {
            let deps: Vec<_> = match &query {
                Some(q) => category
                    .values
                    .into_iter()
                    .filter(|dep| {
                        dep.id.to_lowercase().contains(q) || dep.name.to_lowercase().contains(q)
                    })
                    .collect(),
                None => category.values,
            };

            if deps.is_empty() {
                continue;
            }

            writeln!(buf, "{}:", category.name)?;
            for dep in deps {
                writeln!(buf, "-{} ({})", dep.name, dep.id)?;
                shown += 1;
            }
            writeln!(buf, " ")?;
        }

        if let Some(q) = query
            && shown == 0
        {
            writeln!(buf, "No dependencies match '{q}'")?;
        }
    } else {
        return Err(anyhow!(
            "Provided 'Lists' enum type is not a 'Categories' variant"
        ));
    }

    Ok(())
}
