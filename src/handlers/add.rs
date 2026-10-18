use std::path::{Path, PathBuf};

use quick_xml::{Reader, events::Event};

pub enum BuildFile {
    Maven(PathBuf),
    GradleGroovy(PathBuf),
    GradleKts(PathBuf),
}

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

pub fn detect_build_file(dir: &Path) -> Option<BuildFile> {
    let maven_file = dir.join("pom.xml");
    if maven_file.exists() {
        return Some(BuildFile::Maven(maven_file));
    }

    let gradle_groovy_file = dir.join("build.gradle");
    if gradle_groovy_file.exists() {
        return Some(BuildFile::GradleGroovy(gradle_groovy_file));
    }

    let gradle_kts_file = dir.join("build.gradle.kts");
    if gradle_kts_file.exists() {
        return Some(BuildFile::GradleKts(gradle_kts_file));
    }

    None
}

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
                    && !stack.iter().any(|x| {
                        x == "dependencyManagement" || x == "profile" || x == "build"
                    });

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
