//! Post-processing applied to a freshly downloaded project.
//!
//! The Spring Initializr hands back a zip with the sources and a `.gitignore`
//! that only covers the build tool it generated for. `cargo new` leaves a git
//! repository behind as well, and a `.gitignore` that covers the build output
//! of the tool it scaffolds for. This module does the same for Spring Boot.

pub mod git;
pub mod gitignore;

use std::path::Path;

use anyhow::Result;
use tracing::{debug, info};

/// Prepares a generated project directory.
///
/// Writes the Spring Boot `.gitignore` and, unless `git` is turned off,
/// initializes a git repository inside the project.
///
/// A repository is only created when the project does not already sit inside
/// one, which mirrors `cargo new`: nesting a repository inside another one
/// confuses both git and the user.
pub fn finalize(project_dir: &Path, init_git: bool) -> Result<()> {
    gitignore::write(project_dir)?;

    if !init_git {
        debug!("scaffold: git initialization disabled, skipping");
        return Ok(());
    }

    match git::init_if_needed(project_dir)? {
        git::Outcome::Initialized => {
            info!(
                "scaffold: initialized a git repository in {:?}",
                project_dir.join(".git")
            );
        }
        git::Outcome::AlreadyInsideRepo => {
            info!(
                "scaffold: {:?} is already inside a git repository, no new repository created",
                project_dir
            );
        }
        git::Outcome::AlreadyInitialized => {
            info!(
                "scaffold: {:?} already has a repository, leaving it alone",
                project_dir.join(".git")
            );
        }
    }

    Ok(())
}
