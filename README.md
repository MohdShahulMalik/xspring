# xspring

[![Crates.io](https://img.shields.io/crates/v/xspring.svg)](https://crates.io/crates/xspring)
[![Docs.rs](https://docs.rs/xspring/badge.svg)](https://docs.rs/xspring)
[![License: MIT OR Apache-2.0](https://img.shields.io/crates/l/xspring.svg)](https://opensource.org/licenses/MIT)

xspring replaces the click-through at [start.spring.io](https://start.spring.io) with a terminal tool. It scaffolds new spring boot project with git initialized and adding dependencies to existing projects. The plan is to become an all-in-one build tool for Spring, the way npm and bun are for JavaScript. `xspring add` is an early taste of that.

## Features

### Scaffold

`xspring` prompts you for the project metadata, such as the group id and artifact id, and scaffolds a Spring Boot project from your answers.

The build tool, language, Spring Boot version, packaging, Java version and dependencies are prompted for as well. The prompts are fuzzy finders. Type `sec` and the list narrows to Spring Security. Arrow keys move through the matches, space selects one.

Every project comes with a git repository and a `.gitignore` written for Spring Boot.

### Extend

`xspring add` inserts dependencies into the build file of a project you already have.

- It finds the build file in the target directory, checking `pom.xml` first, then `build.gradle.kts`, then `build.gradle`.
- **Your comments, indentation and line endings survive the edit.**
- For Maven it inserts into the production `<dependencies>` block only, never into `<dependencyManagement>`, a `<profile>` or a `<build>`.
- For Gradle it picks the right scope for each dependency (`implementation`, `testImplementation`, `runtimeOnly`, `annotationProcessor`, `developmentOnly`) and matches the file's own style. Groovy keeps single quotes, the Kotlin DSL uses `implementation("…")`.
- A dependency that is already in the file is reported and left alone.

The dependency table ships with the binary, so no network request is involved.

### Explore

`xspring list` prints the catalog the Initializr exposes, so you can check which dependency ids and versions exist.

## Installation

Install it from Crates.io:

```bash
cargo install xspring
```

Building from source needs Rust 1.88 or newer, the MSRV the `zip` crate requires.

## Usage

### Interactive mode

Run `xspring` with no arguments and answer every prompt:

https://github.com/user-attachments/assets/bd81382f-f7f5-406d-b36e-06b31715eaf7

### Quick mode

`xspring quick` uses the Initializr defaults for most options and only prompts for what it cannot guess.

**Default**

```bash
$ xspring quick
📦 Group ID: com.mycorp
🎫 Artifact ID: user-service
📝 Display Name: User Service
💡 Project Description: A RESTful API for managing users
```

**`-e` or `--extended`**

Skips the display name and the description.

```bash
$ xspring quick -e
📦 Group ID: com.mycorp
🎫 Artifact ID: user-service
```

**`-d` or `--deps`**

Adds the dependency picker. It combines with the other flags.

```bash
$ xspring quick -d
📦 Group ID: com.mycorp
🎫 Artifact ID: user-service
📝 Display Name: User Service
💡 Project Description: A RESTful API for managing users
🧩 Dependencies: › Spring Security - Spring Web - Lombok - ...
```

**`-m` or `--maven`**

`quick` defaults to Gradle. This flag switches to Maven without asking.

```bash
$ xspring quick -e -m
📦 Group ID: com.mycorp
🎫 Artifact ID: user-service
```

### Adding dependencies to an existing project

```bash
$ xspring add -d web,data-jpa
Added dependencies to pom "/path/to/project/pom.xml"
```

Pass `-d`/`--deps` with a comma-separated list of dependency ids to skip the picker:

```bash
xspring add -d web,data-jpa,lombok
```

Run `xspring add` with no arguments to pick from the full catalog with the same fuzzy finder used everywhere else.

An id that does not exist is reported before anything is written:

```text
Unknown dependency id 'data-jpal'. Did you mean: data-jpa?
```

### Listing options

- `xspring list --deps` or `-d`: dependencies
- `xspring list --boot` or `-b`: Spring Boot versions
- `xspring list --java` or `-j`: Java versions
- `xspring list --type` or `-t`: project types, Maven and Gradle
- `xspring list --language` or `-l`: languages

```bash
# every available dependency
$ xspring list --deps

# every supported Java version
$ xspring list --java
```

### Output directory

`-o` or `--output` sets the directory to generate into, or the project whose build file `add` should edit.

```bash
xspring -o my-new-project
xspring -o my-new-project add -d web
```

### Git and `.gitignore`

Here is what a generated project looks like:

```bash
$ xspring quick
📦 Group ID: com.mycorp
🎫 Artifact ID: user-service
...
Generated a Spring Boot project in "user-service"
```

```bash
$ cd user-service
$ git status
On branch main

No commits yet

Untracked files:
  .gitattributes
  .gitignore
  mvnw
  mvnw.cmd
  pom.xml
  src/
```

xspring writes the repository itself rather than shelling out to `git`, so git does not have to be installed to get one. `HEAD` points at `main`, or at whatever `init.defaultBranch` your git configuration sets. No commit is made, so the first one is yours.

The `.gitignore` covers the build output of both Maven and Gradle, the common IDEs, log files and OS clutter:

```gitignore
### Maven ###
target/
.mvn/wrapper/maven-wrapper.jar

### Gradle ###
.gradle/
build/
!gradle/wrapper/gradle-wrapper.jar
...
```

Build output that lives under `src/` is re-included, so a `src/main/target` directory is still tracked.

Two things change the outcome:

- If the project already sits inside another git repository, no nested repository is created. `cargo new` behaves the same way.
- If you do not want a repository, pass `--no-git` or `-ng`:

  ```bash
  xspring quick --no-git
  xspring quick -ng
  ```

### Logging

`xspring` writes daily rotating log files to a `logs` directory in the directory it runs from.

- `-v, --verbose`: raises the verbosity. Repeat it for more detail (`-v` for warnings, `-vv` for info, `-vvv` for debug, `-vvvv` for trace).
- `-q, --quiet`: everything except errors is suppressed.

```bash
# debug output on the terminal
$ xspring -vvv

# errors only
$ xspring --quiet
```

Raising the verbosity turns on two writers:

- stderr shows the level you asked for (`-v` gives warnings, `-vv` gives info, and so on).
- the log file always records `info` and above, so it explains what the run did, where the time went, whether the metadata came from the cache and how long the download took.

The log file only appears once you ask for more than errors, so a plain `xspring list --java` leaves nothing behind in your working directory.

### Caching

The Spring Initializr answers `GET /` with a ~75 KB JSON document describing every dependency, boot version and packaging option. Producing that document is the slowest thing this CLI does. The server needs a few hundred milliseconds to build it, and the response is not compressed.

That document barely ever changes, so `xspring` keeps it on disk:

- a fresh cached document is used without any network request at all
- a stale one is revalidated with a conditional request (`If-None-Match`), which normally answers `304 Not Modified` and costs one round trip instead of a full download
- if the network is unreachable, the stale copy is used as a fallback, so the CLI keeps working offline

The `ETag` is a fingerprint of the document, so a cached copy cannot drift silently. When the content changes, the server sends a new `ETag` with it.

### Environment variables

Every request has a timeout, so a hung server cannot hang the CLI. These are the values you can override:

| Variable | Effect |
|---|---|
| `XSPRING_HTTP_TIMEOUT` | Overall timeout for a single request, in seconds. Default: `30`. |
| `XSPRING_HTTP_CONNECT_TIMEOUT` | Timeout for establishing the connection, in seconds. Default: `10`. |
| `XSPRING_API_URL` | Base URL of the Spring Initializr. Default: `https://start.spring.io`. |
| `XSPRING_CACHE_DIR` | Where the cache lives. Default: `$XDG_CACHE_HOME/xspring`, else `$HOME/.cache/xspring`, else a temp directory. |
| `XSPRING_CACHE_TTL` | Seconds a cached document is served without revalidation. Default: `7200` (matches the server's `cache-control: max-age`). `0` revalidates on every run. |
| `XSPRING_CACHE` | Set to `off` to disable the cache entirely. |
| `XSPRING_LOG` | Maximum level written to the log file (`off`, `error`, `warn`, `info`, `debug`, `trace`). Default: `info`. |
| `XSPRING_LOG_DIR` | Directory the rotating log file is written to. Default: `logs`. |

### Profiling

```bash
# where the time goes, phase by phase
cargo run --release --example profile_phases

# a flame graph of the whole process
cargo install samply
samply record -- ./target/release/xspring list --java
```

## Building from source

1. Clone the repository:

    ```bash
    git clone https://github.com/MohdShahulMalik/xspring.git
    ```

2. Change into the project directory:

    ```bash
    cd xspring
    ```

3. Install it:

    ```bash
    cargo install --path .
    ```

4. The executable ends up in `~/.cargo/bin/xspring`.

## License

This project is licensed under either of the following, at your option:

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

## Contributing

Pull requests and issues are welcome.
