# Plan: `xspring add`

The Initializr can't edit existing projects, so `add` picks the dependency here, maps its Initializr id to real Maven coordinates, and inserts it into the user's build file with text surgery. No re-serialization anywhere, so the rest of the file keeps its bytes, formatting, and comments.

## Step 1. CLI surface

`src/cli/commands/mod.rs` gets a new variant. `-d` takes dependency ids directly, `--pick` opens the MultiSelect you already use.

```rust
Add {
    /// Dependency ids, e.g. -d web -d data-jpa (or --deps web,data-jpa)
    #[arg(short = 'd', long, value_delimiter = ',')]
    deps: Vec<String>,

    /// Interactively pick dependencies instead
    #[arg(short = 'p', long)]
    pick: bool,

    /// Project dir (defaults to cwd / -o)
    #[arg(short = 'r', long)]
    root: Option<PathBuf>,
}
```

Wire it in `main.rs`:

```rust
Commands::Add { deps, pick, root } => {
    let dir = root.or(cli.output).unwrap_or(current_dir()?);
    add_deps(&dir, deps, pick).await
        .with_context(|| "Failed to add dependencies")?;
}
```

Empty both of `deps` and `pick` → error with a hint, don't silently do nothing.

## Step 2. The built-in id → coordinates table

One new file, `src/models/deps_map.rs`. Compiled into the binary. No file IO.

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GradleScope {
    Implementation,
    TestImplementation,
    RuntimeOnly,
    AnnotationProcessor,
    DevelopmentOnly,
}

pub struct Dep {
    pub group: &'static str,
    pub artifact: &'static str,
    pub gradle_scope: GradleScope,
    /// true → Maven needs <optional>true</optional>
    pub maven_optional: bool,
}

pub static DEPS: &[(&str, Dep)] = &[
    ("web",        Dep { group: "org.springframework.boot", artifact: "spring-boot-starter-web",          gradle_scope: GradleScope::Implementation,      maven_optional: false }),
    ("data-jpa",   Dep { group: "org.springframework.boot", artifact: "spring-boot-starter-data-jpa",    gradle_scope: GradleScope::Implementation,      maven_optional: false }),
    ("devtools",   Dep { group: "org.springframework.boot", artifact: "spring-boot-devtools",            gradle_scope: GradleScope::DevelopmentOnly,     maven_optional: false }),
    ("h2",         Dep { group: "com.h2database",           artifact: "h2",                              gradle_scope: GradleScope::RuntimeOnly,         maven_optional: false }),
    ("postgresql", Dep { group: "org.postgresql",           artifact: "postgresql",                      gradle_scope: GradleScope::RuntimeOnly,         maven_optional: false }),
    ("test",       Dep { group: "org.springframework.boot", artifact: "spring-boot-starter-test",        gradle_scope: GradleScope::TestImplementation,  maven_optional: false }),
    ("configuration-processor", Dep { group: "org.springframework.boot", artifact: "spring-boot-configuration-processor", gradle_scope: GradleScope::AnnotationProcessor, maven_optional: true }),
    // fill the rest from start.spring.io's initializr/dependencies.yaml (~70 entries)
];

pub fn lookup(id: &str) -> Option<&'static Dep> {
    DEPS.iter().find(|(k, _)| *k == id).map(|(_, v)| v)
}
```

No version anywhere in the table. Boot's parent/BOM supplies every version, and the Initializr does the same in generated projects. Deps already present in the file get skipped, so re-runs are safe.

## Step 3. Project detection

`src/handlers/add.rs` (new module). Check in this order:

```rust
pub enum BuildFile { Maven(PathBuf), GradleGroovy(PathBuf), GradleKts(PathBuf) }

pub fn detect(dir: &Path) -> Option<BuildFile> {
    let maven = dir.join("pom.xml");
    if maven.is_file() { return Some(BuildFile::Maven(maven)); }
    let kts = dir.join("build.gradle.kts");
    if kts.is_file() { return Some(BuildFile::GradleKts(kts)); }
    let groovy = dir.join("build.gradle");
    if groovy.is_file() { return Some(BuildFile::GradleGroovy(groovy)); }
    None
}
```

None found → error naming what you looked for. Single-module support first; recursive module discovery is a later release.

## Step 4. Maven insertion (the core)

Read `pom.xml` as a `String`. Use quick-xml in **reader mode only** (no serialization, no re-emission) to find the production `<dependencies>` block and how deep the deps sit. Skip any `<dependencies>` inside `<dependencyManagement>`, `<profile>`, or `<build>` by tracking the open-element stack.

```rust
use quick_xml::events::Event;
use quick_xml::Reader;

/// Returns (start_byte, end_byte) of the matching </dependencies>, excluding tags
pub fn find_deps_block(pom: &str) -> Option<(usize, usize, usize)> {
    let mut reader = Reader::from_str(pom);
    let mut depth = 0usize;          // nesting level of the target block
    let mut target: Option<(usize, usize)> = None; // ontent start, open depth
    let mut stack: Vec<String> = Vec::new();

    loop {
        let (event_bytes, event) = match reader.read_event() { Ok(v) => v, Err(_) => return None };
        match event {
            quick_xml::events::Event::Start(ref e) => {
                let name = e.name().as_ref().to_vec();
                if name == b"dependencies" && target.is_none()
                    && !stack.iter().any(|p| p == "dependencyManagement" || p == "profile" || p == "build") {
                    depth = stack.len();
                    target = Some((event_bytes, stack.len()));
                }
                stack.push(unsafe { String::from_utf8_unchecked(name) });
            }
            quick_xml::events::Event::End(ref e) => {
                let name = e.name().as_ref().to_vec();
                stack.pop();
                if name == b"dependencies" && target.is_some() && stack.len() == target.unwrap().1 {
                    return Some((event_bytes, depth, pom.len())); // exact end offset from reader
                }
            }
            quick_xml::events::Event::Eof => return None,
            _ => {}
        }
    }
}
```

(The offsets need precise wiring with `reader.buffer_position()`; the sketch shows the shape. `read_event` gives you each event's byte span, which is what lets you edit the original text without touching anything else.)

Then the insert, from the child formatting the file already has:

```rust
let content = std::fs::read_to_string(&pom)?;
let (block_start, block_end) = find_deps_block(&content)
    .context("No <dependencies> block in pom.xml")?;
let block = &content[block_start..block_end];

// indentation: copy the first existing <dependency> line, else the block's indent + 4
let indent = block.lines()
    .find(|l| l.trim_start().starts_with("<dependency>"))
    .and_then(|l| Some(l[..l.find("<dependency>")?].to_string()))
    .unwrap_or_else(|| "        ".to_string()); // block indent + 4 spaces

let mut snippet = String::new();
snippet.push('\n');
for dep in selected {
    snippet.push_str(&indent);
    snippet.push_str("<dependency>\n");
    snippet.push_str(&indent); snippet.push_str("    "); snippet.push_str(&format!("<groupId>{}</groupId>\n", dep.group));
    snippet.push_str(&indent); snippet.push_str("    "); snippet.push_str(&format!("<artifactId>{}</artifactId>\n", dep.artifact));
    if dep.maven_optional {
        snippet.push_str(&indent); snippet.push_str("    "); snippet.push_str("<optional>true</optional>\n");
    }
    snippet.push_str(&indent);
    snippet.push_str("</dependency>");
}

// splice: put the snippet right before </dependencies>, keep everything else byte-identical
let mut out = String::with_capacity(content.len() + snippet.len());
out.push_str(&content[..block_end - "</dependencies>".len()]);
out.push_str(&snippet);
out.push_str(&content[block_end - "</dependencies>".len()..]);

std::fs::write(&pom, out)?; // also: check the file ends with a newline and preserve that
```

Idempotency check before inserting, against the same block text:

```rust
let already = block.contains(&format!("<artifactId>{}</artifactId>", dep.artifact));
if already { println!("{} already present, skipping", dep.artifact); continue; }
```

## Step 5. Gradle insertion

No XML parsing. `regex-lite` plus line scanning.

Find the `dependencies {` block, then the indentation from an existing `implementation` line if there is one:

```rust
static BLOCK: OnceLock<Regex> = OnceLock::new();
fn gradle_block_re() -> &'static Regex {
    BLOCK.get_or_init(|| Regex::new(r"(?m)^dependencies\s*\{").unwrap())
}
```

Then scan forward brace-depth to find the closing `}`. Gradle has several accepted styles, so handling needs to be aware of all of them:

| File | Existing style | What you emit |
|---|---|---|
| `build.gradle` (Groovy) | `implementation 'g:a:1'` | `implementation 'org.springframework.boot:spring-boot-starter-web'` |
| `build.gradle` | `implementation "g:a"` | `implementation "org.springframework.boot:spring-boot-starter-web"` |
| `build.gradle.kts` | `implementation("g:a")` | `implementation("org.springframework.boot:spring-boot-starter-web")` |

Decide Groovy vs KTS from the file extension (single quotes or no parens in existing lines confirms Groovy). Pick the quote style to match what the file already uses, fall back to defaults (single quote for Groovy, parens for KTS). Scopes map like this:

```
Implementation      → implementation / implementation(
TestImplementation  → testImplementation
RuntimeOnly         → runtimeOnly
AnnotationProcessor → annotationProcessor
DevelopmentOnly     → developmentOnly
```

Same structure as Maven: find the block, match the file's own indentation, insert before the closing brace, skip ids already present.

## Step 6. Handler wiring

`src/handlers/add.rs` orchestrates:

```rust
pub async fn add_deps(dir: &Path, mut ids: Vec<String>, pick: bool) -> Result<()> {
    let build_file = detect(dir).context("No pom.xml, build.gradle, or build.gradle.kts here")?;

    if pick || ids.is_empty() {
        ids = pick_dependencies().await?; // MultiSelect over get_lists("deps"), same UX as quick_interactivity
    }

    let resolved: Vec<&Dep> = ids.iter()
        .map(|id| lookup(id).ok_or_else(|| anyhow!("Unknown dependency id '{}'", id)))
        .collect::<Result<Vec<_>>>()?;

    match build_file {
        BuildFile::Maven(path)      => add_to_pom(&path, &resolved)?,
        BuildFile::GradleGroovy(p)  => add_to_gradle(&p, &resolved, Flavor::Groovy)?,
        BuildFile::GradleKts(p)     => add_to_gradle(&p, &resolved, Flavor::Kts)?,
    }
    println!("Added {} to {:?}", resolved.len(), build_file);
    Ok(())
}
```

Validation on ids before touching the file: unknown id → error listing close matches from the table. Don't write anything until all ids resolve, so a typo never leaves a half-edited file.

## Step 7. Tests that lock the no-reformat guarantee

This is how conditions 1 and 2 stay true. Fixtures built from real `xspring quick` output:

```
tests/fixtures/maven_with_comments/pom.xml
tests/fixtures/gradle_groovy/build.gradle
tests/fixtures/gradle_kts/build.gradle.kts
```

Each test: snapshot the file bytes, run your insert, assert exactly two things: (a) the original bytes are present in the new file in the same order, except for the inserted lines, and (b) comment lines are untouched. `pretty_assertions::assert_eq!` against a checked-in "expected" file, or just byte-compare length difference is `snippet.len()`.

Same trick applies to the Gradle flavors, especially with Windows line endings, since a fixture with `\r\n` proves you preserve them. `read_to_string` keeps `\r\n` intact, but make sure your inserted `\n`s conform to whichever the file uses:

```rust
let eol = if content.contains("\r\n") { "\r\n" } else { "\n" };
```

## Order of work

1. `deps_map.rs` + `lookup` + tests on the table itself.
2. `detect` + Maven insert + fixture tests.
3. Gradle Groovy insert + fixture tests.
4. Gradle KTS insert + fixture tests.
5. `Commands::Add`, `add_deps` wiring, `pick_dependencies` reusing your `MultiSelect`.
6. Manual test: run `xspring add -d web,data-jpa` over a real `xspring quick -m --deps` project, then `git diff` and confirm the only change is the new block.

One nit worth flagging: the snippet in `find_deps_block` above is a sketch of the approach. `read_event()` plus `buffer_position()` is how you get exact byte spans; spend the time to wire offsets cleanly rather than falling back to `find("</dependencies>")`, which picks the wrong block when `<dependencyManagement>` comes first.
