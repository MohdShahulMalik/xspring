//! Behavioural tests for the scaffolding step.
//!
//! The repository xspring writes is checked against real git whenever git is
//! on the `PATH`. Writing those files by hand only counts if git accepts the
//! result.

use std::path::{Path, PathBuf};
use std::process::Command;

use xspring::scaffold::finalize;
use xspring::scaffold::git::{Outcome, init_if_needed};

/// A throwaway directory that cleans itself up.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> TempDir {
        let path = std::env::temp_dir().join(format!(
            "xspring-scaffold-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&path).ok();
        std::fs::create_dir_all(&path).expect("create temp dir");
        TempDir(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

/// Runs git, or returns `None` when git is not installed.
fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .ok()?;

    if !output.status.success() {
        panic!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Whether git is available at all.
fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

#[test]
fn writes_a_gitignore_covering_the_spring_boot_build_output() {
    let dir = TempDir::new("gitignore");
    let project = dir.path().join("demo");
    std::fs::create_dir_all(&project).unwrap();

    finalize(&project, false).expect("finalize succeeds");

    let gitignore = std::fs::read_to_string(project.join(".gitignore")).expect("gitignore written");

    for pattern in ["target/", "build/", ".idea/", ".vscode/", "*.log"] {
        assert!(
            gitignore.lines().any(|line| line.trim() == pattern),
            "{pattern} should be ignored"
        );
    }
}

#[test]
fn creates_a_repository_without_the_git_binary() {
    let dir = TempDir::new("repo");
    let project = dir.path().join("demo");
    std::fs::create_dir_all(&project).unwrap();

    finalize(&project, true).expect("finalize succeeds");

    let git_dir = project.join(".git");
    assert!(git_dir.is_dir(), "the repository should exist");

    assert_eq!(
        std::fs::read_to_string(git_dir.join("HEAD")).unwrap(),
        "ref: refs/heads/main\n"
    );

    for entry in ["objects", "refs/heads", "refs/tags", "info", "hooks"] {
        assert!(
            git_dir.join(entry).is_dir(),
            "{entry} should be part of the repository"
        );
    }

    let config = std::fs::read_to_string(git_dir.join("config")).unwrap();
    assert!(config.contains("repositoryformatversion = 0"));
}

#[test]
fn does_not_create_a_repository_when_git_is_disabled() {
    let dir = TempDir::new("nogit");
    let project = dir.path().join("demo");
    std::fs::create_dir_all(&project).unwrap();

    finalize(&project, false).expect("finalize succeeds");

    assert!(!project.join(".git").exists());
    // The .gitignore is written either way.
    assert!(project.join(".gitignore").exists());
}

#[test]
fn leaves_an_existing_repository_alone() {
    let dir = TempDir::new("existing");
    let project = dir.path().join("demo");
    std::fs::create_dir_all(&project).unwrap();

    std::fs::create_dir_all(project.join(".git")).unwrap();

    let outcome = init_if_needed(&project).expect("init succeeds");

    assert_eq!(outcome, Outcome::AlreadyInitialized);
}

#[test]
fn does_not_nest_a_repository_inside_another_one() {
    let dir = TempDir::new("nested");
    let outer = dir.path().join("outer");
    let inner = outer.join("demo");
    std::fs::create_dir_all(&inner).unwrap();

    // A `.git` directory in the parent is enough to be "inside a repository".
    std::fs::create_dir_all(outer.join(".git")).unwrap();

    let outcome = init_if_needed(&inner).expect("init succeeds");

    assert_eq!(outcome, Outcome::AlreadyInsideRepo);
    assert!(!inner.join(".git").exists());
}

#[test]
fn a_real_parent_repository_stops_a_nested_one() {
    if !git_available() {
        eprintln!("skipping: git is not installed");
        return;
    }

    let dir = TempDir::new("real-parent");
    let outer = dir.path().join("outer");
    let inner = outer.join("demo");
    std::fs::create_dir_all(&inner).unwrap();

    git(&outer, &["init", "-q", "."]).expect("the outer repository is created");

    let outcome = init_if_needed(&inner).expect("init succeeds");

    assert_eq!(outcome, Outcome::AlreadyInsideRepo);
    assert!(!inner.join(".git").exists());
}

#[test]
fn git_accepts_the_repository() {
    if !git_available() {
        eprintln!("skipping: git is not installed");
        return;
    }

    let dir = TempDir::new("git-accepts");
    let project = dir.path().join("demo");
    std::fs::create_dir_all(&project).unwrap();

    std::fs::write(project.join("pom.xml"), "<project/>").unwrap();

    finalize(&project, true).expect("finalize succeeds");

    // The repository is structurally sound.
    git(&project, &["fsck"]).expect("git fsck passes");

    // Git sees the project as a repository on a branch, with the sources
    // waiting to be committed.
    let branch = git(&project, &["symbolic-ref", "HEAD"]).expect("HEAD is readable");
    assert_eq!(branch.trim(), "refs/heads/main");

    let status = git(&project, &["status", "--porcelain"]).expect("status works");
    assert!(
        status.contains("pom.xml"),
        "the generated files should show up as untracked: {status}"
    );

    // And the whole thing can be committed.
    git(
        &project,
        &[
            "-c",
            "user.email=test@example.com",
            "-c",
            "user.name=test",
            "add",
            ".",
        ],
    )
    .expect("git add works");

    git(
        &project,
        &[
            "-c",
            "user.email=test@example.com",
            "-c",
            "user.name=test",
            "commit",
            "-m",
            "initial commit",
        ],
    )
    .expect("git commit works");

    let log = git(&project, &["log", "--oneline"]).expect("git log works");
    assert!(log.contains("initial commit"));
}

#[test]
fn git_ignores_the_build_output() {
    if !git_available() {
        eprintln!("skipping: git is not installed");
        return;
    }

    let dir = TempDir::new("git-ignored");
    let project = dir.path().join("demo");
    std::fs::create_dir_all(&project).unwrap();

    finalize(&project, true).expect("finalize succeeds");

    std::fs::create_dir_all(project.join("target/classes")).unwrap();
    std::fs::write(project.join("target/classes/App.class"), "x").unwrap();
    std::fs::write(project.join("pom.xml"), "<project/>").unwrap();

    let ignored = git(
        &project,
        &["check-ignore", "-q", "target/classes/App.class"],
    );
    assert!(ignored.is_some(), "target/ should be ignored");

    let status = git(&project, &["status", "--porcelain"]).expect("status works");
    assert!(
        !status.contains("target"),
        "the build output should not show up in the status: {status}"
    );
}
