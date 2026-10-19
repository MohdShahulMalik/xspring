use xspring::handlers::add::{find_deps_block, insert_into_gradle, insert_into_pom, suggest_ids, Flavor};
use xspring::models::deps_map::lookup;

const MAVEN: &str = include_str!("fixtures/maven_with_comments/pom.xml");
const MAVEN_CRLF: &str = include_str!("fixtures/maven_crlf/pom.xml");
const GROOVY: &str = include_str!("fixtures/gradle_groovy/build.gradle");
const KTS: &str = include_str!("fixtures/gradle_kts/build.gradle.kts");

fn deps(ids: &[&str]) -> Vec<(&'static str, &'static xspring::models::deps_map::Dep)> {
    ids.iter().map(|id| lookup(id).unwrap()).collect()
}

/// `<artifactId>group…</artifactId>` marker for a resolved dep (for Maven assertions).
fn maven_marker(id: &str) -> String {
    format!("<artifactId>{}</artifactId>", lookup(id).unwrap().1.artifact)
}

/// `group:artifact` coordinates for a resolved dep (for Gradle assertions).
fn coords(id: &str) -> String {
    let dep = lookup(id).unwrap().1;
    format!("{}:{}", dep.group, dep.artifact)
}

/// Insertion only ever adds text, so every original line must still appear,
/// in the same order, in the updated file. This is the "no reformat" lock.
fn lines_preserved(original: &str, updated: &str) {
    let mut cursor = 0usize;
    for line in original.lines() {
        let Some(idx) = updated[cursor..].find(line) else {
            panic!("original line not found in order: {line:?}");
        };
        cursor += idx + line.len();
    }
}

// ---------------------------------------------------------------------
// Maven
// ---------------------------------------------------------------------

#[test]
fn maven_inserts_new_dependencies_in_production_block() {
    let out = insert_into_pom(MAVEN, &deps(&["web", "h2"])).expect("insert succeeds");

    let block = find_deps_block(&out).expect("block still found");
    let prod = &out[block.start_pos..block.end_pos];
    assert!(prod.contains(&maven_marker("web")));
    assert!(prod.contains(&maven_marker("h2")));

    // existing starter plus two new deps, nothing more
    assert_eq!(prod.matches("<dependency>").count(), 3);
}

#[test]
fn maven_never_touches_decoy_blocks_or_comments() {
    let out = insert_into_pom(MAVEN, &deps(&["web", "h2"])).expect("insert succeeds");

    // decoy blocks keep exactly their original dependencies
    assert_eq!(
        out.matches("<artifactId>spring-boot-dependencies</artifactId>").count(),
        MAVEN.matches("<artifactId>spring-boot-dependencies</artifactId>").count()
    );
    assert_eq!(
        out.matches("<artifactId>spring-boot-maven-plugin</artifactId>").count(),
        MAVEN.matches("<artifactId>spring-boot-maven-plugin</artifactId>").count()
    );

    // comments survive byte-for-byte
    assert!(out.contains("<!-- core: keep this comment -->"));
    assert!(out.contains("<!-- and this one too -->"));
    assert!(out.contains("<!-- decoy: managed deps must be ignored -->"));
    assert!(out.contains("<!-- decoy: build-time deps must be ignored -->"));

    lines_preserved(MAVEN, &out);
}

#[test]
fn maven_matches_existing_indentation() {
    let out = insert_into_pom(MAVEN, &deps(&["web"])).expect("insert succeeds");

    // fixture indents <dependency> with 8 spaces and children with 12
    assert!(out.contains(&format!("\n            {}", maven_marker("web"))));
    assert!(!out.contains(&format!("\n   {}", maven_marker("web"))));
}

#[test]
fn maven_reruns_skip_existing_dependencies() {
    let first = insert_into_pom(MAVEN, &deps(&["web", "h2"])).expect("first insert succeeds");
    let second = insert_into_pom(&first, &deps(&["web", "data-jpa"]))
        .expect("second insert succeeds (partially)");

    assert_eq!(second.matches(&maven_marker("web")).count(), 1);
    assert!(second.contains(&maven_marker("data-jpa")));
}

#[test]
fn maven_all_already_present_is_an_error() {
    let out = insert_into_pom(MAVEN, &deps(&["web"])).expect("insert succeeds");
    let err = insert_into_pom(&out, &deps(&["web"])).unwrap_err();
    assert!(err.to_string().contains("already present"), "got: {err}");
}

#[test]
fn maven_picks_production_block_when_management_comes_first() {
    let pom = r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0">
    <dependencyManagement>
        <dependencies>
            <dependency>
                <groupId>org.springframework.boot</groupId>
                <artifactId>spring-boot-dependencies</artifactId>
                <type>pom</type>
                <scope>import</scope>
            </dependency>
        </dependencies>
    </dependencyManagement>
    <dependencies>
        <dependency>
            <groupId>org.springframework.boot</groupId>
            <artifactId>spring-boot-starter</artifactId>
        </dependency>
    </dependencies>
</project>"#;

    let out = insert_into_pom(pom, &deps(&["web"])).expect("insert succeeds");

    // web must land in the production block, not inside dependencyManagement
    let block = find_deps_block(&out).expect("block found");
    let prod = &out[block.start_pos..block.end_pos];
    assert!(prod.contains(&maven_marker("web")));
    assert!(prod.contains("spring-boot-starter"));
    assert!(!prod.contains("spring-boot-dependencies"));
}

#[test]
fn maven_preserves_crlf_line_endings() {
    let out = insert_into_pom(MAVEN_CRLF, &deps(&["web"])).expect("insert succeeds");

    // every inserted newline is CRLF: no bare \n anywhere
    assert_eq!(out.matches("\r\n").count(), out.matches('\n').count());

    // exactly 4 new newlines (<dependency>, groupId, artifactId, </dependency>)
    assert_eq!(out.matches("\r\n").count() - MAVEN_CRLF.matches("\r\n").count(), 4);

    lines_preserved(MAVEN_CRLF, &out);
}

// ---------------------------------------------------------------------
// Gradle Groovy
// ---------------------------------------------------------------------

#[test]
fn groovy_inserts_with_single_quotes_and_matching_indent() {
    let out =
        insert_into_gradle(GROOVY, &deps(&["web", "configuration-processor"]), Flavor::Groovy)
            .expect("insert succeeds");

    // file uses single quotes → emitted statements use single quotes
    assert!(out.contains(&format!("implementation '{}'", coords("web"))));
    assert!(out.contains(&format!("annotationProcessor '{}'", coords("configuration-processor"))));

    // 4-space indentation like the existing scope lines
    assert!(out.contains(&format!("\n    implementation '{}'", coords("web"))));

    // no parens for Groovy
    assert!(!out.contains(&format!("implementation(\"{}\")", coords("web"))));

    assert!(out.contains("// core: keep this comment"));
    lines_preserved(GROOVY, &out);
}

#[test]
fn groovy_matches_double_quotes_when_file_uses_them() {
    let duck_typed = GROOVY.replace(
        "implementation 'org.springframework.boot:spring-boot-starter'",
        "implementation \"org.springframework.boot:spring-boot-starter\"",
    );
    let out =
        insert_into_gradle(&duck_typed, &deps(&["web"]), Flavor::Groovy).expect("insert succeeds");

    assert!(out.contains(&format!("implementation \"{}\"", coords("web"))));
    assert!(out.contains("runtimeOnly 'org.postgresql:postgresql'")); // untouched single quotes
}

#[test]
fn groovy_reruns_skip_existing_dependencies() {
    let first = insert_into_gradle(GROOVY, &deps(&["web"]), Flavor::Groovy).expect("first insert");
    // identical rerun must be a no-op error
    let second = insert_into_gradle(&first, &deps(&["web"]), Flavor::Groovy);
    assert!(
        second.err().unwrap().to_string().contains("already present"),
        "second identical run should be a no-op error"
    );
}

#[test]
fn groovy_starter_prefix_does_not_block_web() {
    // `spring-boot-starter` is a prefix of `spring-boot-starter-test`; adding
    // web (webmvc starter) must not be skipped because of the test line, and
    // the plain starter line must be counted including the new entry
    let out = insert_into_gradle(GROOVY, &deps(&["web"]), Flavor::Groovy).expect("insert succeeds");
    let plain_starter = format!("implementation '{}'", coords("web"));
    assert_eq!(out.matches(plain_starter.as_str()).count(), 1);
    assert!(out.matches(coords("web").as_str()).count() >= 1);
}

// ---------------------------------------------------------------------
// Gradle Kotlin DSL
// ---------------------------------------------------------------------

#[test]
fn kts_inserts_with_parens_and_double_quotes() {
    let out = insert_into_gradle(KTS, &deps(&["web"]), Flavor::Kts).expect("insert succeeds");

    assert!(out.contains(&format!("implementation(\"{}\")", coords("web"))));
    assert!(out.contains(&format!("\n    implementation(\"{}\")", coords("web"))));
    assert!(out.contains("// core: keep this comment"));
    lines_preserved(KTS, &out);
}

#[test]
fn kts_reruns_skip_existing_dependencies() {
    let first = insert_into_gradle(KTS, &deps(&["web"]), Flavor::Kts).expect("first insert");
    let second = insert_into_gradle(&first, &deps(&["web"]), Flavor::Kts);
    assert!(second.err().unwrap().to_string().contains("already present"));
}

// ---------------------------------------------------------------------
// id validation
// ---------------------------------------------------------------------

#[test]
fn unknown_ids_get_close_match_suggestions() {
    let suggestions = suggest_ids("data-jpal");
    assert!(
        suggestions.contains(&"data-jpa"),
        "expected data-jpa suggested for data-jpal, got: {suggestions:?}"
    );
}
