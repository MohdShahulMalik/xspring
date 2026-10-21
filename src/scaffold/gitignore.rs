//! The `.gitignore` written into every generated project.
//!
//! The Initializr ships a `.gitignore` of its own, but it only knows about
//! the build tool it generated for. A Spring Boot project picks up more over
//! time: an IDE, a frontend build, a log directory. This file covers the
//! usual suspects in one place.
//!
//! The order of the rules matters. Git applies the last matching pattern, so
//! the `!` rules that re-include build output inside `src/` have to come
//! after the rules that ignore it.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::debug;

/// The contents written to `.gitignore`.
pub const SPRING_BOOT_GITIGNORE: &str = "\
### Maven ###
target/
.mvn/wrapper/maven-wrapper.jar

### Gradle ###
.gradle/
build/
!gradle/wrapper/gradle-wrapper.jar

### Build output that may live inside the source tree ###
out/
bin/

### Keep build output that lives under src/ ###
!**/src/main/**/target/
!**/src/test/**/target/
!**/src/main/**/build/
!**/src/test/**/build/
!**/src/main/**/out/
!**/src/test/**/out/
!**/src/main/**/bin/
!**/src/test/**/bin/

### IntelliJ IDEA ###
.idea/
*.iws
*.iml
*.ipr

### Eclipse ###
.apt_generated
.classpath
.factorypath
.project
.settings/
.springBeans
.sts4-cache

### NetBeans ###
/nbproject/private/
/nbbuild/
/dist/
/nbdist/
/.nb-gradle/

### VS Code ###
.vscode/

### Generated documentation ###
HELP.md

### Logs ###
*.log
logs/

### OS files ###
.DS_Store
Thumbs.db
desktop.ini
";

/// Writes the Spring Boot `.gitignore` into the project directory,
/// replacing the one the Initializr shipped.
pub fn write(project_dir: &Path) -> Result<()> {
    let path = project_dir.join(".gitignore");

    fs::write(&path, SPRING_BOOT_GITIGNORE)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    debug!("scaffold: wrote {}", path.display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::SPRING_BOOT_GITIGNORE;

    /// Git takes the last matching pattern, so a `!` rule that is meant to
    /// re-include a path has to appear after the rule that ignores it.
    #[test]
    fn negations_come_after_the_rules_they_override() {
        let ignore_all = |pattern: &str| -> usize {
            SPRING_BOOT_GITIGNORE
                .lines()
                .position(|line| line.trim() == pattern)
                .expect("pattern is present")
        };

        for target in ["target/", "build/", "out/", "bin/"] {
            let negation = format!("!**/src/main/**/{target}");
            assert!(
                ignore_all(&negation) > ignore_all(target),
                "{negation} must come after {target}"
            );
        }
    }

    #[test]
    fn covers_the_usual_suspects() {
        for pattern in [
            "target/",
            "build/",
            ".idea/",
            ".vscode/",
            ".classpath",
            "*.log",
            ".DS_Store",
        ] {
            assert!(
                SPRING_BOOT_GITIGNORE
                    .lines()
                    .any(|line| line.trim() == pattern),
                "{pattern} should be ignored"
            );
        }
    }
}
