use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::{Context, Result, anyhow};
use quick_xml::{Reader, events::Event};
use regex_lite::Regex;
use tracing::debug;

use crate::handlers::interactive::pick_dependencies;
use crate::models::deps_map::{DEPS, Dep, GradleScope, lookup};

pub enum BuildFile {
    Maven(PathBuf),
    GradleGroovy(PathBuf),
    GradleKts(PathBuf),
}

impl std::fmt::Debug for BuildFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildFile::Maven(p) => write!(f, "pom {:?}", p.display()),
            BuildFile::GradleGroovy(p) => write!(f, "build.gradle {:?}", p.display()),
            BuildFile::GradleKts(p) => write!(f, "build.gradle.kts {:?}", p.display()),
        }
    }
}

/// A dependency id resolved against the built-in table, giving its Initializr
/// id and its coordinates.
pub type ResolvedDep = (&'static str, &'static Dep);

#[derive(Clone, Copy, Debug)]
struct TargetBlock {
    start_pos: usize,
    open_depth: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct DepsBlock {
    pub start_pos: usize,
    pub depth: usize,
    pub end_pos: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Flavor {
    Groovy,
    Kts,
}

pub fn detect_build_file(dir: &Path) -> Option<BuildFile> {
    let maven_file = dir.join("pom.xml");
    if maven_file.exists() {
        return Some(BuildFile::Maven(maven_file));
    }

    let gradle_kts_file = dir.join("build.gradle.kts");
    if gradle_kts_file.exists() {
        return Some(BuildFile::GradleKts(gradle_kts_file));
    }

    let gradle_groovy_file = dir.join("build.gradle");
    if gradle_groovy_file.exists() {
        return Some(BuildFile::GradleGroovy(gradle_groovy_file));
    }

    None
}

/// Finds the byte span of the production `<dependencies>` block in a pom.xml,
/// skipping any `<dependencies>` nested inside `<dependencyManagement>`,
/// `<profile>`, or `<build>`.
///
/// `start_pos` points right after the opening `<dependencies>` tag and
/// `end_pos` right after the matching `</dependencies>` tag, so the block can
/// be spliced out of the original text without any re-serialization.
pub fn find_deps_block(pom: &str) -> Option<DepsBlock> {
    let mut reader = Reader::from_str(pom);
    let mut depth = 0usize;
    let mut target: Option<TargetBlock> = None;
    let mut stack: Vec<String> = Vec::new();

    loop {
        let event = match reader.read_event() {
            Ok(v) => v,
            Err(_) => return None,
        };

        match event {
            Event::Start(ref e) => {
                let name = e.name().as_ref().to_string();
                let is_target = name == "dependencies"
                    && target.is_none()
                    && !stack
                        .iter()
                        .any(|x| x == "dependencyManagement" || x == "profile" || x == "build");

                if is_target {
                    depth = stack.len();
                    target = Some(TargetBlock {
                        start_pos: reader.buffer_position() as usize,
                        open_depth: stack.len(),
                    });
                }

                stack.push(name);
            }

            Event::End(ref e) => {
                let name = e.name().as_ref().to_string();
                stack.pop();

                if name != "dependencies" {
                    continue;
                }

                let Some(t) = target else {
                    continue;
                };

                if stack.len() != t.open_depth {
                    continue;
                }

                let end_pos = reader.buffer_position() as usize;
                return Some(DepsBlock {
                    start_pos: t.start_pos,
                    depth,
                    end_pos,
                });
            }
            Event::Eof => return None,
            _ => {}
        }
    }
}

/// Text-only edit of the pom contents. Returns the new text with one
/// `<dependency>` entry per dep, inserted at the end of the production
/// dependencies block. Nothing else in the file changes.
pub fn insert_into_pom(content: &str, deps: &[ResolvedDep]) -> Result<String> {
    let block = find_deps_block(content).context("No <dependencies> block in pom.xml")?;
    let closing = "</dependencies>";

    // Right before the closing tag there is whitespace: the closing tag's own
    // indentation. Split the insertion point from that whitespace so new
    // entries land cleanly on their own lines.
    let mut cut = block.end_pos - closing.len();
    while cut > 0 && content[..cut].ends_with([' ', '\t']) {
        cut -= 1;
    }
    let closing_indent = content[cut..block.end_pos - closing.len()].to_string();
    debug_assert!(content[block.end_pos - closing.len()..].starts_with(closing));

    let eol = eol_of(content);
    let indent = maven_indent(content, block).context("Failed to derive indentation")?;

    let existing = &content[block.start_pos..cut];
    let mut snippet = String::new();
    let mut inserted: HashSet<&'static str> = HashSet::new();
    let mut skipped = 0usize;

    for (id, dep) in deps {
        let marker = format!("<artifactId>{}</artifactId>", dep.artifact);
        if existing.contains(&marker) || inserted.contains(&dep.artifact) {
            println!("    {id} already present, skipping");
            skipped += 1;
            continue;
        }
        inserted.insert(dep.artifact);

        let child = format!("{indent}    ");
        snippet.push_str(&indent);
        snippet.push_str("<dependency>");
        snippet.push_str(eol);
        snippet.push_str(&child);
        snippet.push_str(&format!("<groupId>{}</groupId>", dep.group));
        snippet.push_str(eol);
        snippet.push_str(&child);
        snippet.push_str(&format!("<artifactId>{}</artifactId>", dep.artifact));
        snippet.push_str(eol);
        if dep.maven_optional {
            snippet.push_str(&child);
            snippet.push_str("<optional>true</optional>");
            snippet.push_str(eol);
        }
        snippet.push_str(&indent);
        snippet.push_str("</dependency>");
        snippet.push_str(eol);
    }

    if snippet.is_empty() {
        return Err(anyhow!(
            "Nothing to add: {} already present, 0 added",
            skipped
        ));
    }

    // The closing indent after the last entry puts </dependencies> back on
    // its own line at its original indentation.
    snippet.push_str(&closing_indent);

    let mut out = String::with_capacity(content.len() + snippet.len());
    out.push_str(&content[..cut]);
    out.push_str(&snippet);
    out.push_str(&content[block.end_pos - closing.len()..]);
    Ok(out)
}

/// Indentation for a new `<dependency>` entry: copied from the first existing
/// `<dependency>` line in the block, otherwise the block's own indentation + 4.
fn maven_indent(content: &str, block: DepsBlock) -> Option<String> {
    for line in content[block.start_pos..block.end_pos].lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("<dependency>") {
            return Some(line[..line.len() - trimmed.len()].to_string());
        }
    }

    // Fallback: the line `<dependencies>` starts on + 4 spaces.
    let open = content[..block.end_pos].rfind("<dependencies>")?;
    let line_start = content[..open].rfind('\n').map(|i| i + 1).unwrap_or(0);
    Some(format!("{}    ", &content[line_start..open]))
}

fn eol_of(content: &str) -> &'static str {
    if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

fn gradle_block_re() -> &'static Regex {
    static BLOCK: OnceLock<Regex> = OnceLock::new();
    BLOCK.get_or_init(|| Regex::new(r"(?m)^dependencies\s*\{").expect("static regex"))
}

/// Regex matching scope lines, capturing their leading whitespace. Trailing
/// `[^a-zA-Z]` guards against matching `implementationFoo`; the class also
/// happily matches end-of-line.
fn gradle_scope_re() -> &'static Regex {
    static SCOPE: OnceLock<Regex> = OnceLock::new();
    SCOPE.get_or_init(|| {
        Regex::new(
            "(?m)^([ \\t]*)(implementation|testImplementation|runtimeOnly|annotationProcessor|developmentOnly)[^a-zA-Z]",
        )
        .expect("valid static regex")
    })
}

/// Finds the byte position of the `}` that closes the block opened at
/// `open_brace_pos` (which points right after the opening `{`).
fn find_matching_brace(content: &str, open_brace_pos: usize) -> Option<usize> {
    let mut depth = 1usize;
    for (i, c) in content[open_brace_pos..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open_brace_pos + i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Text-only edit of a Gradle build script. Inserts one scoped statement per
/// dep right before the block's closing brace, matching the file's own quote
/// style and indentation.
pub fn insert_into_gradle(content: &str, deps: &[ResolvedDep], flavor: Flavor) -> Result<String> {
    let open_brace_pos = gradle_block_re()
        .find(content)
        .map(|m| m.end() - 1) // the match ends with `{`
        .context("No `dependencies {` block found in build file")?;
    debug_assert!(content[open_brace_pos..].starts_with('{'));

    let close_pos = find_matching_brace(content, open_brace_pos + 1)
        .context("Unbalanced braces in dependencies block")?;

    let block_text = &content[open_brace_pos..close_pos];
    let eol = eol_of(content);

    // Indentation: from an existing scope line, else the closing brace's
    // indentation, else 4 spaces.
    let indent = gradle_scope_re()
        .captures(block_text)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .unwrap_or_else(|| {
            let line_start = content[..close_pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let ws = content[line_start..close_pos].trim();
            if ws == "}" {
                content[line_start..close_pos][..content[line_start..close_pos].len() - ws.len()]
                    .to_string()
            } else {
                "    ".to_string()
            }
        });

    // Quote/paren style: KTS always uses parens + double quotes; Groovy
    // matches the file's own quote style, falling back to single quotes.
    let double_quote = match flavor {
        Flavor::Kts => true,
        Flavor::Groovy => block_text
            .lines()
            .any(|l| gradle_scope_re().find(l).is_some() && l.contains('"')),
    };

    let mut lines = Vec::new();
    let mut skipped = 0usize;
    let mut seen: HashSet<&'static str> = HashSet::new();

    for (id, dep) in deps {
        let coords = format!("{}:{}", dep.group, dep.artifact);
        if gradle_dep_present(block_text, &coords) || seen.contains(&dep.artifact) {
            println!("    {id} already present, skipping");
            skipped += 1;
            continue;
        }
        seen.insert(dep.artifact);

        let statement = gradle_statement(dep, flavor, double_quote);
        lines.push(format!("{indent}{statement}"));
    }

    if lines.is_empty() {
        return Err(anyhow!(
            "Nothing to add: {} already present, 0 added",
            skipped
        ));
    }

    // Insert on their own lines just before the closing brace's line.
    let closing_line_start = content[..close_pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
    if content[closing_line_start..close_pos].trim() == "}" {
        // Closing brace on its own line with its own indentation: replace
        // nothing, just put the new lines before it.
        let snippet = lines
            .iter()
            .map(|l| format!("{l}{eol}"))
            .collect::<String>();
        let mut out = String::with_capacity(content.len() + snippet.len());
        out.push_str(&content[..closing_line_start]);
        out.push_str(&snippet);
        out.push_str(&content[closing_line_start..]);
        return Ok(out);
    }

    // Orphaned/inline closing brace: insert right before it, making sure we
    // start on a fresh line.
    let mut snippet = lines
        .iter()
        .map(|l| format!("{l}{eol}"))
        .collect::<String>();
    if !content[..close_pos].ends_with('\n') {
        snippet.insert_str(0, eol);
    }
    let mut out = String::with_capacity(content.len() + snippet.len());
    out.push_str(&content[..close_pos]);
    out.push_str(&snippet);
    out.push_str(&content[close_pos..]);
    Ok(out)
}

fn gradle_scope(scope: GradleScope) -> &'static str {
    match scope {
        GradleScope::Implementation => "implementation",
        GradleScope::TestImplementation => "testImplementation",
        GradleScope::RuntimeOnly => "runtimeOnly",
        GradleScope::AnnotationProcessor => "annotationProcessor",
        GradleScope::DevelopmentOnly => "developmentOnly",
    }
}

/// `group:artifact` present check that respects boundaries, so
/// `spring-boot-starter` does not match the line holding
/// `spring-boot-starter-test`. A hit counts as present when the char right
/// after the coordinates is a delimiter (quote, paren, whitespace, ...).
fn gradle_dep_present(block_text: &str, coords: &str) -> bool {
    let mut from = 0usize;
    while let Some(i) = block_text[from..].find(coords) {
        let after = from + i + coords.len();
        match block_text[after..].chars().next() {
            None => return true,
            Some(c) if !c.is_alphanumeric() && !"-_:./".contains(c) => return true,
            _ => {}
        }
        from = after;
    }
    false
}

fn gradle_statement(dep: &Dep, flavor: Flavor, double_quote: bool) -> String {
    let coords = format!("{}:{}", dep.group, dep.artifact);
    match flavor {
        Flavor::Groovy => {
            let q = if double_quote { '"' } else { '\'' };
            format!("{} {}{}{}", gradle_scope(dep.gradle_scope), q, coords, q)
        }
        Flavor::Kts => format!("{}(\"{}\")", gradle_scope(dep.gradle_scope), coords),
    }
}
/// Close-match suggestions for an unknown dependency id. Every token of the
/// input must match a token of the key (exact or prefix), so `data-jpal`
/// suggests `data-jpa`.
pub fn suggest_ids(input: &str) -> Vec<&'static str> {
    let lowered = input.to_lowercase();
    let tokens: Vec<&str> = lowered.split(['-', '_', '.']).collect();
    let matches_token = |tok: &str, part: &str| {
        !tok.is_empty()
            && !part.is_empty()
            && (part == tok || tok.starts_with(part) || part.starts_with(tok))
    };
    DEPS.iter()
        .filter(|(k, _)| {
            let parts: Vec<&str> = k.split(['-', '_', '.']).collect();
            tokens.len() <= parts.len()
                && tokens
                    .iter()
                    .all(|t| parts.iter().any(|p| matches_token(t, p)))
        })
        .filter(|(k, _)| *k != input)
        .map(|(k, _)| *k)
        .take(5)
        .collect()
}

pub fn unknown_id_error(id: &str) -> anyhow::Error {
    let suggestions = suggest_ids(id);
    let hint = if suggestions.is_empty() {
        "Run `xspring list --deps` to see available dependency ids".to_string()
    } else {
        format!("Did you mean: {}?", suggestions.join(", "))
    };
    anyhow!("Unknown dependency id '{id}'. {hint}")
}

pub async fn add_deps(dir: &Path, ids: Vec<String>) -> Result<()> {
    let build_file = detect_build_file(dir)
        .context("No pom.xml, build.gradle, or build.gradle.kts found in this directory")?;

    let mut ids = ids;
    if ids.is_empty() {
        ids = pick_dependencies().await?;
    }
    if ids.is_empty() {
        return Err(anyhow!("No dependencies selected, nothing to do"));
    }

    // Resolve everything first: a typo must never leave a half-edited file.
    let resolved: Vec<ResolvedDep> = ids
        .iter()
        .map(|id| lookup(id).ok_or_else(|| unknown_id_error(id)))
        .collect::<Result<Vec<_>>>()?;
    debug!("Resolved dependencies: {:?}", resolved);

    match &build_file {
        BuildFile::Maven(path) => {
            let content = fs::read_to_string(path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            let out = insert_into_pom(&content, &resolved)?;
            fs::write(path, out)?;
        }
        BuildFile::GradleGroovy(path) | BuildFile::GradleKts(path) => {
            let flavor = if matches!(build_file, BuildFile::GradleKts(_)) {
                Flavor::Kts
            } else {
                Flavor::Groovy
            };
            let content = fs::read_to_string(path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            let out = insert_into_gradle(&content, &resolved, flavor)?;
            fs::write(path, out)?;
        }
    }

    println!("Added dependencies to {build_file:?}");
    Ok(())
}
