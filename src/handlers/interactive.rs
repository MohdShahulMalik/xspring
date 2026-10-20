use inquire::{required, validator::Validation, MultiSelect, Select, Text};
use anyhow::{Context, Result};
use tracing::{debug, info, trace};

use crate::models::spring::InitializrMetadata;
use crate::{cli::interactive_ui::base_config, client::spring_initializr::get_metadata, models::spring::QueryParam};

/// Controls which prompts are shown and which values fall back to the
/// metadata defaults from start.spring.io.
#[derive(Debug, Clone, Copy, Default)]
pub struct InteractiveConfig {
    /// Prompt for project type, language, boot version, packaging and java
    /// version. When false, the metadata defaults are used without prompting.
    pub full: bool,

    /// Prompt for display name and description. When false, the metadata
    /// defaults are used without prompting.
    pub name_description: bool,

    /// Show the dependency MultiSelect prompt.
    pub deps: bool,

    /// Force Maven as the project type (skips the project type prompt).
    pub maven: bool,
}

impl InteractiveConfig {
    /// Everything gets prompted (what `xspring` with no args does today).
    pub fn full() -> Self {
        Self { full: true, name_description: true, deps: true, maven: false }
    }

    /// The `xspring quick` behavior: only group/artifact (+ name/description
    /// and/or deps depending on the flags).
    pub fn quick(maven: bool, extended: bool, deps: bool) -> Self {
        Self { full: false, name_description: !extended, deps, maven }
    }
}

/// Normalizes a start.spring.io boot version id (e.g. `3.3.4-SNAPSHOT`) into
/// the format expected by the query (`3.3.4-SNAPSHOT`, `3.2.0`, `1.0.0-M1`...).
fn normalize_boot_version(id: &str) -> String {
    id.replace(".RELEASE", "")
        .replace(".BUILD-SNAPSHOT", "-SNAPSHOT")
        .replace(".M", "-M")
        .replace(".RC", "-RC")
}

/// Prompts for a required, space-free identifier (group id / artifact id).
fn ask_id(label: &'static str, help: &'static str, icon: &'static str) -> Result<String> {
    Text::new(label)
        .with_validator(required!())
        .with_validator(|input: &str| {
            if input.contains(' ') {
                Ok(Validation::Invalid("This field's input cannot contain spaces".into()))
            } else {
                Ok(Validation::Valid)
            }
        })
        .with_help_message(help)
        .with_render_config(base_config(icon))
        .prompt()
        .with_context(|| format!("Failed to get input for {label}"))
}

/// Fixed fallbacks matching start.spring.io's own defaults when the API
/// omits the `default` field for name/description.
fn default_name(metadata: &InitializrMetadata) -> String {
    metadata.name.default.clone().unwrap_or_else(|| "demo".to_string())
}

fn default_description(metadata: &InitializrMetadata) -> String {
    metadata
        .description
        .default
        .clone()
        .unwrap_or_else(|| "Demo project for xspring".to_string())
}

pub async fn interactive(config: InteractiveConfig) -> Result<QueryParam> {
    let spring_metadata = get_metadata()
        .await
        .with_context(|| "Failed to get the metadata")?;
    trace!("Spring Metadata: {:?}", spring_metadata);
    info!(
        "interactive: full={}, name_description={}, deps={}, maven={}",
        config.full, config.name_description, config.deps, config.maven
    );

    // ------------------------------------------------------------------
    // Shared prompts: group id + artifact id (always required)
    // ------------------------------------------------------------------
    let group_id = ask_id("Group ID:", "e.g. com.example", "📦")?;
    debug!("Group ID: {}", group_id);

    let artifact_id = ask_id("Artifact ID:", "e.g. my-awesome-project", "🎫")?;
    debug!("Artifact ID: {}", artifact_id);

    // ------------------------------------------------------------------
    // Name + description (skippable)
    // ------------------------------------------------------------------
    let (name, description) = if config.name_description {
        let mut name = Text::new("Display Name:")
            .with_help_message("This will be the display name for your project")
            .with_render_config(base_config("📝"))
            .prompt()
            .with_context(|| "Failed to get input for Project Name")?;
        if name.is_empty() {
            name = default_name(&spring_metadata);
            debug!("Project Name after replacing the empty string: {}", name);
        }

        let mut description = Text::new("Project Description:")
            .with_help_message("A brief description of your project.")
            .with_render_config(base_config("💡"))
            .prompt()
            .with_context(|| "Failed to get input for Description")?;
        if description.is_empty() {
            description = default_description(&spring_metadata);
            debug!("Project description after replacing the empty string: {}", description);
        }

        (name, description)
    } else {
        debug!("Using default name/description");
        (default_name(&spring_metadata), default_description(&spring_metadata))
    };

    // ------------------------------------------------------------------
    // Project type (skippable, forced to Maven when config.maven)
    // ------------------------------------------------------------------
    let project_type = if config.maven {
        debug!("Project type forced to maven-project via --maven flag");
        "maven-project".to_string()
    } else if config.full {
        let project_types = spring_metadata.project_type.values.clone();
        let project_type = Select::new("Project Type:", project_types)
            .with_help_message("Choose the build system for your project (Maven or Gradle)")
            .with_render_config(base_config("🧰"))
            .prompt()
            .with_context(|| "Failed to get input for Project Type")?;
        debug!("Project type: {:?}", project_type.id);
        project_type.id
    } else {
        spring_metadata.project_type.default.clone()
    };

    // ------------------------------------------------------------------
    // Language (skippable)
    // ------------------------------------------------------------------
    let language = if config.full {
        let languages = spring_metadata.language.values.clone();
        let language = Select::new("Language:", languages)
            .with_help_message("Choose the programming language for your project")
            .with_render_config(base_config("💻"))
            .prompt()
            .with_context(|| "Failed to get input for Language")?;
        debug!("Language Choice: {:?}", language.id);
        language.id
    } else {
        spring_metadata.language.default.clone()
    };

    // ------------------------------------------------------------------
    // Boot version (skippable); in full mode the default is moved to the
    // front of the list
    // ------------------------------------------------------------------
    let boot_version = if config.full {
        let mut boot_versions = spring_metadata.boot_version.values.clone();
        trace!("Spring Boot Versions: {:?}", boot_versions);
        let default_idx = boot_versions
            .iter()
            .position(|v| v.id == spring_metadata.boot_version.default)
            .unwrap_or(0);
        boot_versions.swap(0, default_idx);

        let boot_version = Select::new("Spring Boot Version:", boot_versions)
            .with_help_message("Choose the version of Spring Boot for your project")
            .with_render_config(base_config("🚀"))
            .prompt()
            .with_context(|| "Failed to get input for Spring Boot Version")?;
        debug!("Selected Boot Version: {:?}", boot_version.id);
        normalize_boot_version(&boot_version.id)
    } else {
        normalize_boot_version(&spring_metadata.boot_version.default)
    };

    // ------------------------------------------------------------------
    // Packaging (skippable)
    // ------------------------------------------------------------------
    let packaging = if config.full {
        let packages = spring_metadata.packaging.values.clone();
        let packaging = Select::new("Package Type:", packages)
            .with_help_message("Choose how your project will be packaged")
            .with_render_config(base_config("🎁"))
            .prompt()
            .with_context(|| "Failed to get input for Package Type")?;
        debug!("Project Packaging Choice: {:?}", packaging.id);
        packaging.id
    } else {
        spring_metadata.packaging.default.clone()
    };

    // ------------------------------------------------------------------
    // Java version (skippable); in full mode the default is moved to the
    // front of the list
    // ------------------------------------------------------------------
    let java_version = if config.full {
        let mut java_versions = spring_metadata.java_version.values.clone();
        let default_idx = java_versions
            .iter()
            .position(|v| v.id == spring_metadata.java_version.default)
            .unwrap_or(0);
        java_versions.swap(0, default_idx);

        let java_version = Select::new("Java Version:", java_versions)
            .with_help_message("Select java version for your project")
            .with_render_config(base_config("☕"))
            .prompt()
            .with_context(|| "Failed to get input for Java Version")?;
        debug!("Select Java Version: {:?}", java_version.id);
        java_version.id
    } else {
        spring_metadata.java_version.default.clone()
    };

    // ------------------------------------------------------------------
    // Dependencies (skippable)
    // ------------------------------------------------------------------
    let dependencies = if config.deps {
        let dependency_names = spring_metadata.dependencies.values
            .iter()
            .flat_map(|category| category.values.iter())
            .collect::<Vec<_>>();

        let dependency_choices = MultiSelect::new("Dependencies:", dependency_names)
            .with_page_size(5)
            .with_keep_filter(true)
            .with_help_message("🔍 Type to search • Space to select • ↑↓ to navigate • Enter to confirm")
            .with_render_config(base_config("🧩"))
            .prompt()
            .with_context(|| "Failed to get input for Dependencies")?;
        debug!("Selected Dependencies: {:?}", dependency_choices);

        dependency_choices
            .iter()
            .map(|dep| dep.id.clone())
            .collect::<Vec<_>>()
            .join(",")
    } else {
        String::new()
    };

    Ok(QueryParam {
        project_type,
        language,
        boot_version,
        group_id,
        artifact_id: artifact_id.clone(),
        name,
        description,
        packaging,
        java_version,
        dependencies,
        base_dir: artifact_id,
    })
}

/// Full interactivity: every field is prompted.
pub async fn pure_interactivity() -> Result<QueryParam> {
    interactive(InteractiveConfig::full()).await
}

/// Quick interactivity: only group/artifact are always prompted; the rest
/// depends on the flags.
pub async fn quick_interactivity(maven: bool, extended: bool, deps: bool) -> Result<QueryParam> {
    interactive(InteractiveConfig::quick(maven, extended, deps)).await
}

/// Standalone dependency picker used by `xspring add` when no `-d/--deps`
/// values are given.
pub async fn pick_dependencies() -> Result<Vec<String>> {
    let spring_metadata = get_metadata()
        .await
        .with_context(|| "Failed to get the metadata")?;

    let dependency_names = spring_metadata.dependencies.values
        .iter()
        .flat_map(|category| category.values.iter())
        .collect::<Vec<_>>();

    let dependency_choices = MultiSelect::new("Dependencies:", dependency_names)
        .with_page_size(5)
        .with_keep_filter(true)
        .with_help_message("🔍 Type to search • Space to select • ↑↓ to navigate • Enter to confirm")
        .with_render_config(base_config("🧩"))
        .prompt()
        .with_context(|| "Failed to get input for Dependencies")?;

    Ok(dependency_choices.iter().map(|dep| dep.id.clone()).collect())
}
