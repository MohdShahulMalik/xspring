# xspring: The Interactive Spring Boot Project Generator

[![Crates.io](https://img.shields.io/crates/v/xspring.svg)](https://crates.io/crates/xspring)
[![Docs.rs](https://docs.rs/xspring/badge.svg)](https://docs.rs/xspring)
[![License: MIT OR Apache-2.0](https://img.shields.io/crates/l/xspring.svg)](https://opensource.org/licenses/MIT)

**xspring** is a powerful, modern command-line interface (CLI) for creating and configuring Spring Boot applications. Built for speed, ease of use, and developer productivity, xspring provides an interactive, guided experience that streamlines project setup, from simple web applications to complex microservices. It combines the flexibility of the official Spring Initializr with the convenience of a fast, local, and feature-rich tool.

Whether you're a seasoned Spring developer who values automation or a newcomer learning the ecosystem, xspring is designed to get you from idea to `public static void main` in seconds.

## Core Features

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
- **Discoverability and Exploration**: xspring helps you explore the Spring ecosystem without leaving your terminal.
    - `xspring list --deps` or `-d`: Browse all available dependencies by category.
    - `xspring list --boot` or `-b`: See a list of supported Spring Boot versions (stable, milestone, and snapshot).
    - `xspring list --java` or `-j`: Check the available Java runtimes.
    - `xspring list --type` or `-t`: Check the available project types.
    - `xspring list --language` or `-l`: Check the available languages.

## Why Use xspring?

- **Speed**: Written in Rust, xspring is incredibly fast and responsive.
- **User-Friendly**: Its interactive nature makes it accessible to developers of all skill levels.
- **Reduces Errors**: By guiding the user, it helps prevent typos and misconfigurations.
- **Boosts Productivity**: Automates the repetitive boilerplate of starting a new project, letting you focus on writing code.
- **Modern Tooling**: Brings a CLI experience on par with tools from other ecosystems like `create-react-app` or `cargo`.

## Installation

You can install `xspring` from Crates.io:

```bash
cargo install xspring
```

## Usage

### Interactive Mode

Run `xspring` with no arguments to enter the interactive mode:

```bash
$ xspring
📦 Group ID: com.mycorp
🎫 Artifact ID: user-service
📝 Display Name: User Service
💡 Project Description: A RESTful API for managing users
🧰 Project Type: › Maven - Gradle -  ...
💻 Language: › Java - Kotlin - Groovy
🚀 Spring Boot Version: › 3.2.1 - 3.1.8 - ...
🎁 Package Type: › Jar - War
☕ Java Version: › 17 - 21 - ...
🧩 Dependencies: › Spring Security - Spring Web - Lombok - ...
```

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

### Listing Options

- **List Dependencies**: `xspring list -d` or `xspring list --deps`
- **List Spring Boot Versions**: `xspring list -b` or `xspring list --boot`
- **List Java Versions**: `xspring list -j` or `xspring list --java`
- **List Project Types(Ex. Maven)**: `xspring list -t` or `xspring list --type`
- **List Languages**: `xspring list -l` or `xspring list --language`

### Output Directory

- `-o` or `--output`: Specify a directory to output the generated project to.

  ```bash
  xspring -o my-new-project
  ```

### Logging

`xspring` creates daily rotating log files in a `logs` directory in the directory where it is executed.

You can control the log verbosity with the following flags:

- `-v, --verbose`: Increases the verbosity of the output. Can be used multiple times to increase the level of detail (e.g., `-v` for warnings, `-vv` for info, `-vvv` for debug, `-vvvv` for trace).
- `-q, --quiet`: Suppresses all output except for errors.

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
