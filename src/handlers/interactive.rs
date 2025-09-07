use inquire::{Text, Select, MultiSelect};
use anyhow::{Context, Result};
use tracing::{debug, trace};
use crate::{client::spring_initializr::get_metadata, models::spring::QueryParam};

pub async fn pure_interactivity() -> Result<QueryParam> {
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
    
    Ok(QueryParam {
        project_type: project_type.id.clone(),
        language: language.id.clone(),
        boot_version: boot_version.id.clone(),
        group_id ,
        artifact_id: artifact_id.clone(),
        name,
        description,
        packaging: packaging.id.clone(),
        java_version: java_version.id.clone(),
        dependencies: dependencies.iter().map(|dep| dep.id.clone() ).collect::<Vec<_>>().join(","),
        base_dir: artifact_id
    })
}

pub async fn quick_interactivity(extended: bool) -> Result<QueryParam>{
    let spring_metadata = get_metadata().await
        .with_context(|| "Failed to get the metadata")?;

    let group_id: String = Text::new("Group ID:")
        .with_help_message("e.g. com.example")
        .prompt()
        .with_context(|| "Failed to get input for Group ID")?;
    debug!("Group ID: {}", group_id);

    let artifact_id = Text::new("Artifact ID:")
        .with_help_message("e.g. my-awesome-project")
        .prompt()
        .with_context(|| "Failed to get input for Artifact ID")?;
    debug!("Artifact ID: {}", artifact_id);

        
    let mut name: Option<String> = None;
    let mut description: Option<String> = None;

    if extended {
        let _name = Text::new("Project Name:")
            .with_help_message("This will be the display name for your project")
            .prompt()
            .with_context(|| "Failed to get input for Project Name")?;
        name = Some(_name);
        debug!("Project Name Choice: {:?}", name);

        let _description = Text::new("Description:")
            .with_help_message("A brief description of your project.")
            .prompt()
            .with_context(|| "Failed to get input for Description")?;
        description = Some(_description);
        debug!("Project Description Choice: {:?}", description);
      
    }

    Ok(QueryParam {
        project_type: spring_metadata.project_type.default,
        language: spring_metadata.language.default,
        boot_version: spring_metadata.boot_version.default,
        group_id,
        artifact_id: artifact_id.clone(),
        name: name.unwrap_or(spring_metadata.name.default),
        description: description.unwrap_or(spring_metadata.description.default),
        packaging: spring_metadata.packaging.default,
        java_version: spring_metadata.java_version.default,
        dependencies: "".to_string(),
        base_dir: artifact_id,
    })

}
