# XSpring Project Notes

This document outlines the design, features, and implementation plan for the `xspring` CLI.

## 1. Core Functionality

-   **Spring API Integration**:
    -   Fetch metadata for Spring Boot versions, Java versions, and dependencies.
    -   Build a Spring Boot project based on selected options.
-   **Interactive Mode**:
    -   Use the `inquire` crate for an interactive user experience when creating projects.

## 2. CLI Design

### 2.1. Global Flags

-   `--yes` / `-y`: Skip interactive prompts and use default values.
-   `--verbose` / `-v`: Enable verbose output.
-   `--output <PATH>` / `-o <PATH>`: Specify the output directory.
-   `--config <PATH>` / `-c <PATH>`: Specify a configuration file to use.

### 2.2. Subcommands

-   **`init`**: Create a new Spring Boot project interactively.
    -   `--name <NAME>`: Project name (will prompt if not provided).
    -   `--version <VERSION>`: Spring Boot version.
-   **`quick`**: Non-interactive project generation with default values.
    -   `--name <NAME>`: Override the default project name.
-   **`new`**: Alias for `init`.
-   **`template <TEMPLATE_NAME>`**: Generate a project from a predefined template.
    -   `web`: Web application template.
    -   `api`: REST API template.
    -   `batch`: Batch processing template.
    -   `--name <NAME>`: Override template defaults.
-   **`list <TYPE>`**: Show available options.
    -   `deps`: List available dependencies.
    -   `versions`: List Spring Boot versions.
    -   `java`: List Java versions.
    -   `packaging`: List packaging options.
    -   `templates`: List available templates.
-   **`config`**: Manage configuration.
    -   `show`: Show current configuration.
    -   `set <KEY> <VALUE>`: Set a configuration value.
    -   `reset`: Reset configuration to defaults.
    -   `edit`: Open the configuration file for editing.

### 2.3. Example Usage

```bash
# Interactive mode (most common)
xspring init

# Quick default project
xspring quick

# Quick with custom name
xspring quick --name my-awesome-app

# Non-interactive with specific values
xspring init --name myapp --version 3.2.0 --yes

# Use a template
xspring template web --name my-web-app

# List available options
xspring list deps
xspring list versions

# Use a config file
xspring init --config my-spring-config.toml

# Verbose output
xspring init --verbose
```

## 3. Proposed Project Structure

```
src/
├── main.rs                 # Entry point and orchestration
├── cli/
│   ├── mod.rs             # CLI module declarations
│   ├── root.rs            # Root CLI struct and global args
│   └── commands/
│       ├── mod.rs         # Commands module declarations
│       ├── template.rs    # Template subcommand
│       ├── list.rs        # List subcommands
│       └── config.rs      # Config subcommands
└── handlers/              # Business logic handlers
    ├── mod.rs
    ├── template.rs
    ├── list.rs
    └── config.rs
```

## 4. Implementation Snippets

### `src/main.rs`

```rust
mod cli;
mod handlers;

use cli::Cli;
use clap::Parser;

fn main() {
    let cli = Cli::parse();

    if cli.verbose {
        println!("Running in verbose mode");
    }

    match cli.command {
        Some(cli::commands::Commands::Template(cmd)) => {
            handlers::handle_template(&cmd, cli.yes, cli.verbose);
        }
        Some(cli::commands::Commands::List(cmd)) => {
            handlers::handle_list(&cmd, cli.verbose);
        }
        Some(cli::commands::Commands::Config(cmd)) => {
            handlers::handle_config(&cmd, cli.verbose);
        }
        None => {
            // Default interactive mode
            if cli.yes {
                generate_with_defaults();
            } else {
                run_interactive_mode();
            }
        }
    }
}

fn generate_with_defaults() {
    println!("Generating with defaults");
}

fn run_interactive_mode() {
    println!("Running interactive mode");
}
```

### `src/cli/mod.rs`

```rust
pub mod root;
pub mod commands;

pub use root::Cli;
```

### `src/cli/root.rs`

```rust
use clap::Parser;
use super::commands::Commands;

#[derive(Parser)]
#[command(name = "xspring")]
#[command(about = "Interactive Spring Boot project generator")]
pub struct Cli {
    /// Skip interactive prompts and use defaults
    #[arg(long, short = 'y', global = true)]
    pub yes: bool,

    /// Verbose output
    #[arg(long, short, global = true)]
    pub verbose: bool,

    /// Output directory
    #[arg(long, short, global = true)]
    pub output: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}
```

### `src/cli/commands/mod.rs`

```rust
pub mod template;
pub mod list;
pub mod config;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Generate project using predefined templates
    #[command(flatten)]
    Template(template::TemplateCommand),

    /// List available options
    #[command(flatten)]
    List(list::ListCommand),

    /// Configuration management
    #[command(flatten)]
    Config(config::ConfigCommand),
}
```

### `src/cli/commands/template.rs`

```rust
use clap::{Args, Subcommand};

#[derive(Args)]
pub struct TemplateCommand {
    #[command(subcommand)]
    pub action: TemplateActions,
}

#[derive(Subcommand)]
pub enum TemplateActions {
    /// Generate from web template
    Web {
        /// Override project name
        #[arg(long)]
        project_name: Option<String>,

        /// Override group ID
        #[arg(long)]
        group_id: Option<String>,
    },

    /// Generate from API template
    Api {
        /// Override project name
        #[arg(long)]
        project_name: Option<String>,

        /// Override group ID
        #[arg(long)]
        group_id: Option<String>,
    },

    /// Generate from batch template
    Batch {
        /// Override project name
        #[arg(long)]
        project_name: Option<String>,

        /// Override group ID
        #[arg(long)]
        group_id: Option<String>,
    },

    /// List available templates
    List,
}
```

### `src/cli/commands/list.rs`

```rust
use clap::{Args, Subcommand};

#[derive(Args)]
pub struct ListCommand {
    #[command(subcommand)]
    pub action: ListActions,
}

#[derive(Subcommand)]
pub enum ListActions {
    /// List available dependencies
    Deps {
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,
    },

    /// List Spring Boot versions
    Versions,

    /// List available templates
    Templates,

    /// List Java versions
    Java,
}
```

### `src/cli/commands/config.rs`

```rust
use clap::{Args, Subcommand};

#[derive(Args)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub action: ConfigActions,
}

#[derive(Subcommand)]
pub enum ConfigActions {
    /// Show current configuration
    Show,

    /// Set a configuration value
    Set {
        key: String,
        value: String,
    },

    /// Reset configuration to defaults
    Reset,

    /// Edit configuration file
    Edit,
}
```

### `src/handlers/mod.rs`

```rust
pub mod template;
pub mod list;
pub mod config;

// Re-export handler functions
pub use template::handle_template;
pub use list::handle_list;
pub use config::handle_config;
```

### `src/handlers/template.rs`

```rust
use crate::cli::commands::template::{TemplateCommand, TemplateActions};

pub fn handle_template(cmd: &TemplateCommand, global_yes: bool, global_verbose: bool) {
    match &cmd.action {
        TemplateActions::Web { project_name, group_id } => {
            println!("Creating web template");
            // Implementation here
        }
        TemplateActions::Api { project_name, group_id } => {
            println!("Creating API template");
            // Implementation here
        }
        TemplateActions::Batch { project_name, group_id } => {
            println!("Creating batch template");
            // Implementation here
        }
        TemplateActions::List => {
            println!("Listing available templates");
            // Implementation here
        }
    }
}
```

### `src/handlers/list.rs`

```rust
use crate::cli::commands::list::{ListCommand, ListActions};

pub fn handle_list(cmd: &ListCommand, global_verbose: bool) {
    match &cmd.action {
        ListActions::Deps { category } => {
            println!("Listing dependencies");
            if let Some(cat) = category {
                println!("Filtered by category: {}", cat);
            }
        }
        ListActions::Versions => {
            println!("Listing Spring Boot versions");
        }
        ListActions::Templates => {
            println!("Listing available templates");
        }
        ListActions::Java => {
            println!("Listing Java versions");
        }
    }
}
```

### `src/handlers/config.rs`

```rust
use crate::cli::commands::config::{ConfigCommand, ConfigActions};

pub fn handle_config(cmd: &ConfigCommand, global_verbose: bool) {
    match &cmd.action {
        ConfigActions::Show => {
            println!("Showing configuration");
        }
        ConfigActions::Set { key, value } => {
            println!("Setting {}={}", key, value);
        }
        ConfigActions::Reset => {
            println!("Resetting configuration");
        }
        ConfigActions::Edit => {
            println!("Opening configuration for editing");
        }
    }
}
```