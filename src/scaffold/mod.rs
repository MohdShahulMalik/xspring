//! Post-processing applied to a freshly downloaded project.
//!
//! The Spring Initializr hands back a zip with the sources and a `.gitignore`
//! that only covers the build tool it generated for. `cargo new` leaves a
//! `.gitignore` behind as well, one that covers the build output of the tool
//! it scaffolds for. This module does the same for Spring Boot.

pub mod gitignore;

use std::path::Path;

use anyhow::Result;

/// Prepares a generated project directory.
///
/// Writes the Spring Boot `.gitignore` into the project.
pub fn finalize(project_dir: &Path) -> Result<()> {
    gitignore::write(project_dir)?;

    Ok(())
}
