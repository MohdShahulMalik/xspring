use inquire::{Text, Select, MultiSelect};
use anyhow::Result;
use tracing::{debug, trace};
use crate::client::spring_initializr::get_metadata;

pub async fn pure_interactivity() -> Result<()> {
    let spring_metadata = get_metadata().await?;
    trace!("Spring Metadata: {:?}", spring_metadata);

    let project_types = vec!["maven", "gradle"];
    let project_type = Select::new("Project Type:", project_types)
        .prompt()?;
    debug!("Project type: {}", project_type);

    let mut _gradle_type: Option<&str> = None;

    if project_type == "gradle" {
        let gradle_choices = vec!["kotlin", "groovy"];
        let gradle_choice = Select::new("Gradle Type:", gradle_choices)
            .prompt()?;
        debug!("Gradle choice: {}", gradle_choice);

        _gradle_type = Some(gradle_choice);
    }

    let languages: Vec<&str> = vec!["java", "kotlin", "groovy"];
    let language = Select::new("Language:", languages)
        .prompt()?;
    debug!("Lanuage Choice: {:?}", language);

    let boot_versions: Vec<String> = spring_metadata.boot_version.values.iter().map(|version_metadata| version_metadata.name.clone()).collect();
    trace!("Spring Boot Versions: {:?}", boot_versions);

    let boot_version = Select::new("Spring Boot Version:", boot_versions)
     .prompt()?;
    debug!("Selected Boot Version: {:?}", boot_version);
    
    let group_id = Text::new("Group ID:")
        .prompt()?;
    debug!("Group ID: {}", group_id);

    let artifact_id = Text::new("Artifact ID:")
        .prompt()?;
    debug!("Artifact ID: {}", artifact_id);

    let name = Text::new("Project Name:")
        .prompt()?;
    debug!("Project Name Choice: {}", name);

    let description = Text::new("Project Description:")
        .prompt()?;
    debug!("Project Description Choice: {}", description);

    let packages = vec!["jar", "war"];
    let packaging = Select::new("Packaging", packages)
        .prompt()?;
    debug!("Project Packaging Choice: {}", packaging);

    let dependency_names: Vec<String> = spring_metadata.dependencies.values.iter()
        .flat_map(|category| category.values.iter())
        .map(|dep| dep.name.clone())
        .collect();
    trace!("All Dependency Names: {:?}", dependency_names);
    let dependencies = MultiSelect::new("Dependencies", dependency_names)    
        .with_page_size(5)
        .with_keep_filter(true)
        .with_help_message("🔍 Type to search • Space to select • ↑↓ to navigate • Enter to confirm")
        .prompt()?;
    debug!("Selected Dependencies: {:?}", dependencies);
    
    Ok(())
}
