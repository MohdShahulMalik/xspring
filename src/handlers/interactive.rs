use inquire::{Text, Select, MultiSelect};
use anyhow::{Context, Result};
use tracing::{debug, trace};
use crate::client::spring_initializr::get_metadata;

pub async fn pure_interactivity() -> Result<()> {
    let spring_metadata = get_metadata().await?;
    trace!("Spring Metadata: {:?}", spring_metadata);

    let project_types = spring_metadata.project_type.values;
    let project_type = Select::new("Project Type:", project_types)
        .with_help_message("Choose the build system for your project (Maven or Gradle)")
        .prompt()
        .with_context(|| "Failed to get input for Project Type")?;
    debug!("Project type: {:?}", project_type);


    let languages = spring_metadata.language.values;
    let language = Select::new("Language:", languages)
        .with_help_message("Choose the programming language for your project")
        .prompt()
        .with_context(|| "Failed to get input for Language")?;
    debug!("Lanuage Choice: {:?}", language);

    let mut boot_versions = spring_metadata.boot_version.values;
    trace!("Spring Boot Versions: {:?}", boot_versions);
    let default_boot_version = spring_metadata.boot_version.default;
    let default_boot_version_idx = boot_versions.iter().position(|v| v.id == default_boot_version).unwrap_or(0);
    let temp = boot_versions[default_boot_version_idx].clone();
    boot_versions[default_boot_version_idx] = boot_versions[0].clone();
    boot_versions[0] = temp;
    
    let boot_version = Select::new("Spring Boot Version:", boot_versions)
        .with_help_message("Choose the version of Spring Boot for your project")
        .prompt()
        .with_context(|| "Failed to get input for Spring Boot Version")?;
    debug!("Selected Boot Version: {:?}", boot_version);
    
    let group_id = Text::new("Group ID:")
        .with_help_message("e.g. com.example")
        .prompt()
        .with_context(|| "Failed to get input for Group ID")?;
    debug!("Group ID: {}", group_id);

    let artifact_id = Text::new("Artifact ID:")
        .with_help_message("e.g. my-awesome-project")
        .prompt()
        .with_context(|| "Failed to get input for Artifact ID")?;
    debug!("Artifact ID: {}", artifact_id);

    let name = Text::new("Project Name:")
        .with_help_message("This will be the display name for your project")
        .prompt()
        .with_context(|| "Failed to get input for Project Name")?;
    debug!("Project Name Choice: {}", name);

    let description = Text::new("Project Description:")
        .with_help_message("A brief description of your project.")
        .prompt()
        .with_context(|| "Failed to get input for Description")?;
    debug!("Project Description Choice: {}", description);

    let packages = spring_metadata.packaging.values;
    let packaging = Select::new("Package Type:", packages)
        .with_help_message("Choose how your project will be packaged")
        .prompt()
        .with_context(|| "Failed to get input for Package Type")?;
    debug!("Project Packaging Choice: {:?}", packaging);

    let mut java_versions = spring_metadata.java_version.values;
    let default_java_version = spring_metadata.java_version.default;
    let default_java_version_idx = java_versions.iter().position(|v| v.id == default_java_version).unwrap_or(0);
    java_versions.swap(0, default_java_version_idx);
    let java_version = Select::new("Java Version", java_versions)
        .with_help_message("Select java version for your project")
        .prompt()
        .with_context(|| "Failed to get input for Java Version")?;
    debug!("Select Java Version: {:?}", java_version);

    let dependency_names = spring_metadata.dependencies.values.iter()
        .flat_map(|category| category.values.iter())
        .collect();
    trace!("All Dependency Names: {:?}", dependency_names);
    let dependencies = MultiSelect::new("Dependencies", dependency_names)   
        .with_page_size(5)
        .with_keep_filter(true)
        .with_help_message("🔍 Type to search • Space to select • ↑↓ to navigate • Enter to confirm")
        .prompt()
        .with_context(|| "Failed to get input for Dependencies")?;
    debug!("Selected Dependencies: {:?}", dependencies);
    
    Ok(())
}
