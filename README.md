# xspring: The Spring Boot Generator and Dependency Manager

[![Crates.io](https://img.shields.io/crates/v/xspring.svg)](https://crates.io/crates/xspring)
[![Docs.rs](https://docs.rs/xspring/badge.svg)](https://docs.rs/xspring)
[![License: MIT OR Apache-2.0](https://img.shields.io/crates/l/xspring.svg)](https://opensource.org/licenses/MIT)

**xspring** is a fast, modern command-line interface (CLI) for Spring Boot projects. It does three things: it **scaffolds** new projects through an interactive walkthrough, it **explains** what the Spring ecosystem offers, and it **adds dependencies** to projects you already have — all without leaving the terminal.

Built in Rust on top of the official Spring Initializr API, xspring turns `start.spring.io` into something you can drive from a shell. From an idea to `public static void main` in seconds; from a bare `pom.xml` to a fully wired-up service in one command.

## Core Features

### Scaffold

- **Interactive by Default**: Run `xspring` with no arguments to enter a step-by-step guided mode. It prompts you for everything needed to create a project, including:
    - Project metadata (Group ID, Artifact ID, Name, Description)
    - Build tool (Maven or Gradle)
    - Language (Java, Kotlin, or Groovy)
    - Spring Boot version
    - Packaging (Jar or War)
    - Java version
    - Dependencies
- **Intelligent Fuzzy-Finding**: Forget memorizing exact option names. Simply start typing to filter through choices for dependencies, Spring Boot versions, languages, and more.
    - Example: Typing `sec` might suggest `Spring Security`.
    - Navigate suggestions with arrow keys and select your desired thing.
- **Accelerated Setup With Quick Interactivity**: Use the `quick` subcommand for a faster, streamlined project setup with sensible defaults.
    - use `-e` or `--extended` flag to scaffold a project more quickly
    - use `-m` or `--maven` if you prefer to change the default project type
    - use `-d` or `--deps` to add dependencies to your project interactively

### Explore

- **Discoverability and Exploration**: xspring helps you explore the Spring ecosystem without leaving your terminal.
    - `xspring list --deps` or `-d`: Browse all available dependencies by category.
    - `xspring list --boot` or `-b`: See a list of supported Spring Boot versions (stable, milestone, and snapshot).
    - `xspring list --java` or `-j`: Check the available Java runtimes.
    - `xspring list --type` or `-t`: Check the available project types.
    - `xspring list --language` or `-l`: Check the available languages.

### Extend

- **Add Dependencies to an Existing Project**: The `add` subcommand wires dependencies into a project you already have, editing the build file in place.
    - Works with Maven (`pom.xml`), Gradle Groovy (`build.gradle`) and Gradle Kotlin (`build.gradle.kts`).
    - Uses the same dependency ids as `xspring list --deps`.
    - Surgically edits the build file: your comments, indentation and line endings survive.
    - Idempotent — dependencies that are already present are skipped.
    - Works without a network connection.

## Why Use xspring?

- **One tool for the whole loop**: Scaffold a project, explore what's available, and extend what you built — without switching between a browser, an IDE and a build tool.
- **Fast**: Written in Rust, xspring is incredibly fast and responsive. The Initializr metadata is cached on disk, so repeat commands return instantly instead of waiting on the network.
- **Safe edits**: Build files are edited surgically rather than regenerated, so a `pom.xml` full of comments and custom formatting comes out the other side intact.
- **Works offline**: The cached metadata keeps `xspring list` and the interactive picker usable even when the network is down.
- **User-Friendly**: Its interactive nature makes it accessible to developers of all skill levels, while every prompt has a flag for scripting.
- **Reduces Errors**: By guiding the user, it helps prevent typos and misconfigurations.

## Installation

You can install `xspring` from Crates.io:

```bash
cargo install xspring
```

Building from source requires **Rust 1.88 or newer** (the `zip` crate raised its MSRV there).

## Usage

### Interactive Mode

Run `xspring` with no arguments to enter the interactive mode:

https://github.com/user-attachments/assets/bd81382f-f7f5-406d-b36e-06b31715eaf7

### Quick-Interactive Mode

For a faster setup, you can use the `quick` subcommand. This mode uses sensible defaults for most options, only prompting for the essential information.

**Default Quick Mode**

By default, the `quick` subcommand will prompt for the Group ID, Artifact ID, Display Name, and Description.

```bash
$ xspring quick
📦 Group ID: com.mycorp
🎫 Artifact ID: user-service
📝 Display Name: User Service
💡 Project Description: A RESTful API for managing users
```

**Extended Quick Mode (`-e` or `--extended`)**

You can further streamline the process with the `-e` or `--extended` flag, which uses default values for the Display Name and Description, only prompting for the Group ID and Artifact ID.

```bash
$ xspring quick -e
📦 Group ID: com.mycorp
🎫 Artifact ID: user-service
```

**Dependencies Flag (`-d` or `--deps`)**

The `-d` or `--deps` flag allows you to select dependencies for your project. This can be combined with other flags.

```bash
$ xspring quick -d
📦 Group ID: com.mycorp
🎫 Artifact ID: user-service
📝 Display Name: User Service
💡 Project Description: A RESTful API for managing users
🧩 Dependencies: › Spring Security - Spring Web - Lombok - ...
```

**Maven Flag (`-m` or `--maven`)**

By default, the `quick` subcommand uses Gradle as the project type. The `-m` or `--maven` flag allows you to override this and set the project type to Maven without prompting. This can be combined with either of the above modes.

```bash
$ xspring quick -e -m
📦 Group ID: com.mycorp
🎫 Artifact ID: user-service
```

### Adding Dependencies to an Existing Project

The `add` subcommand edits the build file of a project that already exists.

```bash
$ xspring add -d web,data-jpa
Added dependencies to pom "/path/to/project/pom.xml"
```

**Non-interactive** — pass `-d`/`--deps` with a comma-separated list of dependency ids:

```bash
xspring add -d web,data-jpa,lombok
```

**Interactive** — run `xspring add` with no arguments to pick from the full catalog with the same fuzzy finder used everywhere else.

**What it does**

- **Detects the build file** in the target directory: `pom.xml`, then `build.gradle.kts`, then `build.gradle`.
- **Maven**: inserts into the production `<dependencies>` block only — never into `dependencyManagement`, a `<profile>` or `<build>` — and keeps your comments, indentation and line endings intact.
- **Gradle**: picks the right scope for every dependency (`implementation`, `testImplementation`, `runtimeOnly`, `annotationProcessor`, `developmentOnly`) and matches the file's own style — Groovy keeps single quotes, Kotlin DSL uses `implementation("…")`.
- **Skips what is already there**: a dependency that is already in the build file is reported and left alone.
- **Never leaves a half-edited file**: every id is resolved before anything is written, so one typo aborts the whole operation.
- **Catches typos**:

  ```text
  Unknown dependency id 'data-jpal'. Did you mean: data-jpa?
  ```

**Targeting another directory** with the global `-o`/`--output` flag:

```bash
xspring -o ../user-service add -d web
```

### Listing Options

- **List Dependencies**: `xspring list -d` or `xspring list --deps`
- **List Spring Boot Versions**: `xspring list -b` or `xspring list --boot`
- **List Java Versions**: `xspring list -j` or `xspring list --java`
- **List Project Types(Ex. Maven)**: `xspring list -t` or `xspring list --type`
- **List Languages**: `xspring list -l` or `xspring list --language`

```bash
# List all available dependencies
$ xspring list --deps

# List all supported Java versions
$ xspring list --java
```

### Output Directory

- `-o` or `--output`: Specify a directory to output the generated project to, or the project whose build file `add` should edit.

  ```bash
  xspring -o my-new-project
  xspring -o my-new-project add -d web
  ```

### Logging

`xspring` creates daily rotating log files in a `logs` directory in the directory where it is executed.

You can control the log verbosity with the following flags:

- `-v, --verbose`: Increases the verbosity of the output. Can be used multiple times to increase the level of detail (e.g., `-v` for warnings, `-vv` for info, `-vvv` for debug, `-vvvv` for trace).
- `-q, --quiet`: Suppresses all output except for errors.

```bash
# Run with verbose output to show debug information
$ xspring -vvv

# Run in quiet mode
$ xspring --quiet
```

Two things are written when you raise the verbosity:

- **the terminal (stderr)** shows the level you asked for (`-v` → warnings, `-vv` → info, …);
- **the log file** always records `info` and above, so it explains what the run did — where the time went, whether the metadata came from the cache, how long the download took.

The log file is only created once you ask for more than errors, so a plain `xspring list --java` still leaves no trace in your working directory.

### Caching

The Spring Initializr answers `GET /` with a ~75 KB JSON document describing every dependency, boot version and packaging option. Producing that document is the slowest thing this CLI does — the server needs a few hundred milliseconds to think about it, and the response is not compressed.

That document barely ever changes, so `xspring` keeps it on disk:

- a **fresh** cached document is used without any network request at all;
- a **stale** one is revalidated with a conditional request (`If-None-Match`), which normally answers `304 Not Modified`;
- if the network is unreachable, the stale copy is used as a fallback, so the CLI keeps working offline.

The Initializr's `ETag` is a fingerprint of the document, so the cached copy can never silently drift: if the content changes, the server sends a new one.

| Variable | Effect |
|---|---|
| `XSPRING_CACHE_DIR` | Where the cache lives. Default: `$XDG_CACHE_HOME/xspring`, else `$HOME/.cache/xspring`, else a temp directory. |
| `XSPRING_CACHE_TTL` | Seconds a cached document is served without revalidation. Default: `7200` (matches the server's `cache-control: max-age`). `0` revalidates on every run. |
| `XSPRING_CACHE` | Set to `off` to disable the cache entirely. |

### Network

Every request has a timeout, so a hung server can never hang the CLI.

| Variable | Effect |
|---|---|
| `XSPRING_HTTP_TIMEOUT` | Overall timeout for a single request, in seconds. Default: `30`. |
| `XSPRING_HTTP_CONNECT_TIMEOUT` | Timeout for establishing the connection, in seconds. Default: `10`. |
| `XSPRING_API_URL` | Base URL of the Spring Initializr. Default: `https://start.spring.io`. |
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

## Building from Source

1.  Clone the repository:
    ```bash
    git clone https://github.com/MohdShahulMalik/xspring.git
    ```
2.  Navigate to the project directory:
    ```bash
    cd xspring
    ```
3.  Install the project:
    ```bash
    cargo install --path .
    ```
4.  The executable will be in `~/.cargo/bin/xspring`.

## License

This project is licensed under either of the following, at your option:

-   Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
-   MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

## Contributing

Contributions are welcome! Please feel free to submit a pull request or open an issue.
