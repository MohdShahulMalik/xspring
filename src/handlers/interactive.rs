use inquire::{Text, Select};
use anyhow::Result;
use tracing::{debug, trace};
use crate::client::spring_initializr::get_metadata;

pub async fn pure_interactivity() -> Result<()> {
    let spring_metadata = get_metadata().await?;
    trace!("Spring Metadata: {:?}", spring_metadata);

    let boot_versions: Vec<String> = spring_metadata.boot_version.values.iter().map(|version_metadata| version_metadata.name.clone()).collect();
    trace!("Spring Boot Versions: {:?}", boot_versions);

    let boot_version = Select::new("Spring Boot Version:", boot_versions)
     .prompt()?;
    debug!("Selected Boot Version: {:?}", boot_version);
    
    let project_types = vec!["maven", "gradle"];
    let project_type = Select::new("Project Type:", project_types)
        .prompt()?;
    debug!("Project type: {}", project_type);

    let mut _gradle_type: Option<&str> = None;

    match project_type {
        "gradle" => {
            let gradle_choices = vec!["kotlin", "groovy"];
            let gradle_choice = Select::new("Gradle Type:", gradle_choices)
                .prompt()?;
            debug!("Gradle choice: {}", gradle_choice);

            _gradle_type = Some(gradle_choice);
        }

        _ => {}

    }
    let group_id = Text::new("Group ID:")
        .prompt()?;
    debug!("Group ID: {}", group_id);

    let artifact_id = Text::new("Artifact ID:")
        .prompt()?;
    debug!("Artifact ID: {}", artifact_id);

    Ok(())
}
