# README.md

# Serde Config Docs

**This project was totally vibecoded. i have no idea how it works.**

Automatically generate markdown documentation for configuration options defined in Rust structs that use Serde for serialization/deserialization.

## Features

- Generate detailed markdown documentation from Serde-annotated structs
- Support for nested configuration structures
- Customizable output format (supports TOML, can be extended)
- Documentation includes field names, types, and default values
- Automatic file export for documentation during tests

## Installation

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
serde-config-docs = "0.1.0"
serde = { version = "1.0", features = ["derive"] }
```

## Basic Usage

1. Add the `ConfigDocs` derive macro to your configuration struct
2. Use standard Serde attributes to customize field names
3. Add the `#[config_docs(export)]` attribute to enable automatic documentation export

```rust
use serde::{Serialize, Deserialize};
use serde_config_docs::ConfigDocs;

#[derive(Serialize, Deserialize, ConfigDocs)]
#[serde(rename_all = "snake_case")]
#[config_docs(export)] // Enable auto-export during tests
pub struct ServerConfig {
    /// The address to bind the server to
    #[serde(default = "default_address")]
    pub address: String,

    /// The port to listen on
    #[serde(default = "default_port")]
    pub port: u16,

    // Nested configuration
    pub logging: LogConfig,
}

fn default_address() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

#[derive(Serialize, Deserialize, ConfigDocs)]
pub struct LogConfig {
    /// The log level to use
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Whether to log to stdout
    #[serde(default = "default_log_stdout")]
    pub stdout: bool,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_stdout() -> bool {
    true
}
```

## Exporting Documentation

When you add the `#[config_docs(export)]` attribute to your struct, the library will automatically generate a test that exports markdown documentation to the `docs/` directory when you run tests:

```bash
cargo test
```

This will create a file named `YourStructName.toml.md` in the `docs/` directory.

You can specify a different format using the `CONFIG_DOCS_FORMAT` environment variable:

```bash
CONFIG_DOCS_FORMAT=toml cargo test
```

## Manual Generation

You can also generate documentation programmatically:

```rust
use serde_config_docs::{ConfigFormat, MarkdownOptions};

// Generate documentation with default options
let options = MarkdownOptions::new(ConfigFormat::Toml);
let docs = serde_config_docs::generate_config_docs_with_options::<ServerConfig>(&options);
println!("{}", docs);

// Or with custom title
let options = MarkdownOptions::new(ConfigFormat::Toml)
    .title(Some("Server Configuration".to_string()));
let docs = serde_config_docs::generate_config_docs_with_options::<ServerConfig>(&options);
```

## Generated Documentation Example

The generated markdown will look something like this:

````markdown
## Logging

```toml
[logging]

# The log level to use
# Default: "info"
level = "info"

# Whether to log to stdout
# Default: true
stdout = true
```
````

```

## How It Works

The `ConfigDocs` derive macro analyzes your struct at compile time and implements the `ConfigDocsStruct` trait. This trait provides a schema describing your configuration structure, which is then used to generate the markdown documentation.

The macro:
1. Extracts field names, types, and Serde attributes
2. Processes documentation comments
3. Handles nested configuration structures
4. Generates code to export the documentation when tests are run

## License

MIT
```
