//! Git repository initialization that does not need the `git` binary.
//!
//! `cargo new` initializes a repository through libgit2, so the user never
//! needs git on their `PATH` for it to work. Doing the same here means
//! writing the handful of files that make up an empty repository by hand:
//! `HEAD`, `config`, `description`, `info/exclude` and the object and ref
//! directories. The result is an ordinary repository that any version of git
//! picks up without complaining.
//!
//! No commit is created. The generated files stay untracked, which is what
//! `cargo new` leaves behind as well: the first commit belongs to the user.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::debug;

/// Branch used when the user has not configured `init.defaultBranch`.
const FALLBACK_BRANCH: &str = "main";

/// The `description` git writes into a fresh repository.
const DESCRIPTION: &str =
    "Unnamed repository; edit this file 'description' to name the repository.\n";

/// The `info/exclude` git writes into a fresh repository.
const INFO_EXCLUDE: &str = "\
# git ls-files --others --exclude-from=.git/info/exclude
# Lines that start with '#' are comments.
# For a project mostly in C, the following would be a good set of
# exclude patterns (uncomment them if you want to use them):
# *.[oa]
# *~
";

/// What happened when a repository was requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// A new repository was created.
    Initialized,
    /// The project already has a repository.
    AlreadyInitialized,
    /// The project sits inside another repository, so no nested one was made.
    AlreadyInsideRepo,
}

/// Initializes a git repository in `project_dir`.
///
/// Nothing happens when the project already has a `.git` directory, or when
/// the project lives inside another repository. Creating a repository inside
/// a repository is almost never what the user meant.
pub fn init_if_needed(project_dir: &Path) -> Result<Outcome> {
    let git_dir = project_dir.join(".git");

    if git_dir.exists() {
        debug!("git: {} already exists", git_dir.display());
        return Ok(Outcome::AlreadyInitialized);
    }

    if let Some(parent_repo) = enclosing_repository(project_dir) {
        debug!(
            "git: {:?} is inside the repository at {:?}",
            project_dir, parent_repo
        );
        return Ok(Outcome::AlreadyInsideRepo);
    }

    let branch = default_branch();
    write_repository(&git_dir, &branch)?;

    debug!(
        "git: initialized a repository in {:?} on branch {branch}",
        git_dir
    );

    Ok(Outcome::Initialized)
}

/// Writes the files that make up an empty repository.
fn write_repository(git_dir: &Path, branch: &str) -> Result<()> {
    for dir in [
        "objects/info",
        "objects/pack",
        "refs/heads",
        "refs/tags",
        "info",
        "hooks",
    ] {
        let path = git_dir.join(dir);
        fs::create_dir_all(&path)
            .with_context(|| format!("Failed to create {}", path.display()))?;
    }

    let head = git_dir.join("HEAD");
    fs::write(&head, format!("ref: refs/heads/{branch}\n"))
        .with_context(|| format!("Failed to write {}", head.display()))?;

    let config = git_dir.join("config");
    fs::write(&config, repository_config())
        .with_context(|| format!("Failed to write {}", config.display()))?;

    let description = git_dir.join("description");
    fs::write(&description, DESCRIPTION)
        .with_context(|| format!("Failed to write {}", description.display()))?;

    let exclude = git_dir.join("info/exclude");
    fs::write(&exclude, INFO_EXCLUDE)
        .with_context(|| format!("Failed to write {}", exclude.display()))?;

    Ok(())
}

/// The `config` file of a fresh repository.
fn repository_config() -> String {
    let mut config = String::from("[core]\n");
    config.push_str("\trepositoryformatversion = 0\n");
    config.push_str("\tfilemode = true\n");
    config.push_str("\tbare = false\n");
    config.push_str("\tlogallrefupdates = true\n");
    config
}

/// Finds the repository that contains `path`, if there is one.
///
/// A `.git` entry counts whether it is a directory or a file: worktrees and
/// submodules use a file that points at the real repository.
fn enclosing_repository(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .skip(1)
        .find(|dir| dir.join(".git").exists())
        .map(Path::to_path_buf)
}

/// The branch the initial `HEAD` points at.
///
/// Git reads `init.defaultBranch` from the user's configuration, so we do
/// too, falling back to `main` when nothing is set.
fn default_branch() -> String {
    // Git reads the files in order and lets the later ones win, so the most
    // specific file is looked at first.
    config_files()
        .iter()
        .rev()
        .find_map(|file| read_default_branch(file))
        .unwrap_or_else(|| FALLBACK_BRANCH.to_string())
}

/// The configuration files git would read, least specific first.
fn config_files() -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Some(path) = crate::util::env_non_empty("GIT_CONFIG_SYSTEM") {
        files.push(PathBuf::from(path));
    } else {
        files.push(PathBuf::from("/etc/gitconfig"));
    }

    match crate::util::env_non_empty("GIT_CONFIG_GLOBAL") {
        Some(path) => files.push(PathBuf::from(path)),
        None => {
            if let Some(home) = crate::util::env_non_empty("HOME") {
                let home = PathBuf::from(home);

                if let Some(xdg) = crate::util::env_non_empty("XDG_CONFIG_HOME") {
                    files.push(PathBuf::from(xdg).join("git/config"));
                } else {
                    files.push(home.join(".config/git/config"));
                }

                // `~/.gitconfig` is read last, so it wins over the XDG file.
                files.push(home.join(".gitconfig"));
            }
        }
    }

    files
}

/// Reads `init.defaultBranch` from a git configuration file.
///
/// Git's configuration format is not trivial, but the section and key we care
/// about are plain enough to spot: an `[init]` section holding a
/// `defaultBranch` key.
fn read_default_branch(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;

    let mut in_init = false;

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        if let Some(section) = line.strip_prefix('[') {
            let section = section.split(']').next().unwrap_or("");
            in_init = section.trim().eq_ignore_ascii_case("init");
            continue;
        }

        if !in_init {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        if key.trim().eq_ignore_ascii_case("defaultbranch") {
            let value = value.trim().trim_matches('"');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{FALLBACK_BRANCH, read_default_branch, repository_config};

    #[test]
    fn config_matches_what_git_writes() {
        let config = repository_config();
        assert!(config.starts_with("[core]\n"));
        assert!(config.contains("repositoryformatversion = 0"));
        assert!(config.contains("bare = false"));
        assert!(config.contains("logallrefupdates = true"));
    }

    #[test]
    fn reads_the_default_branch_from_a_config_file() {
        let dir = std::env::temp_dir().join("xspring-git-config-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("gitconfig");

        std::fs::write(
            &path,
            "[core]\n\tfilemode = true\n[init]\n\tdefaultBranch = trunk\n",
        )
        .unwrap();

        assert_eq!(read_default_branch(&path).as_deref(), Some("trunk"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn ignores_branches_from_other_sections() {
        let dir = std::env::temp_dir().join("xspring-git-config-other-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("gitconfig");

        std::fs::write(&path, "[user]\n\tdefaultBranch = nope\n").unwrap();

        assert_eq!(read_default_branch(&path), None);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn falls_back_to_main() {
        assert_eq!(FALLBACK_BRANCH, "main");
    }

    /// Git reads the configuration files in order and lets the later ones
    /// win, so `~/.gitconfig` beats the XDG file and the system file.
    #[test]
    fn the_most_specific_config_file_wins() {
        let dir = std::env::temp_dir().join("xspring-git-config-order-test");
        std::fs::create_dir_all(&dir).unwrap();

        let xdg = dir.join("xdg");
        let home = dir.join("home");
        std::fs::create_dir_all(&xdg).unwrap();
        std::fs::create_dir_all(&home).unwrap();

        std::fs::write(
            home.join(".gitconfig"),
            "[init]\n\tdefaultBranch = fromgitconfig\n",
        )
        .unwrap();
        std::fs::write(xdg.join("config"), "[init]\n\tdefaultBranch = fromxdg\n").unwrap();

        let files = [xdg.join("config"), home.join(".gitconfig")];

        let branch = files
            .iter()
            .rev()
            .find_map(|file| read_default_branch(file))
            .unwrap();

        assert_eq!(branch, "fromgitconfig");

        std::fs::remove_dir_all(&dir).ok();
    }
}
