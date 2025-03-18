// serde_config_docs/src/lib.rs
//! Autogenerate Markdown documentation for Serde struct configuration options
//!
//! This crate generates documentation for configuration options defined in Rust
//! structs that use serde for serialization/deserialization.

use serde::Serialize;
pub use serde_config_docs_derive::ConfigDocs;

use std::fmt::{self, Write};

/// Options to customize the structure of the output Markdown document
#[derive(Debug, Clone)]
pub struct MarkdownOptions {
    pub title: Option<String>,
    pub format: ConfigFormat,
}

/// The serialization format to display examples in
#[derive(Debug, Clone, Copy)]
pub enum ConfigFormat {
    // #[cfg(toml)]
    Toml,
    // Other formats could be added later:
    // Json,
    // Yaml,
}

impl MarkdownOptions {
    /// Set the configuration format to display examples in
    pub fn new(format: ConfigFormat) -> Self {
        MarkdownOptions {
            title: None,
            format,
        }
    }
    /// Set a custom title to use in the generated document
    pub fn title(mut self, title: Option<String>) -> Self {
        self.title = title;
        self
    }
}

/// Information about a configuration field
#[derive(Debug, Clone)]
pub struct FieldInfo {
    /// The name of the field as it appears in the serialized format
    pub name: String,
    pub doc_comments: Option<String>,
    pub default_value: Option<String>,
    pub field_type: String,
    pub is_nested: bool,
    pub nested_fields: Vec<FieldInfo>,
}

impl FieldInfo {
    /// Create a new field info object
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            doc_comments: None,
            default_value: None,
            field_type: "".to_string(),
            is_nested: false,
            nested_fields: Vec::new(),
        }
    }

    /// Set the documentation comment for this field
    pub fn doc(mut self, doc: impl Into<String>) -> Self {
        self.doc_comments = Some(doc.into());
        self
    }

    /// Set the default value for this field
    pub fn default(mut self, default: impl Into<Option<String>>) -> Self {
        self.default_value = default.into();
        self
    }

    /// Set the type of this field
    pub fn field_type(mut self, field_type: impl Into<String>) -> Self {
        self.field_type = field_type.into();
        self
    }

    /// Make this field a nested section with child fields
    pub fn nested(mut self, nested_fields: Vec<FieldInfo>) -> Self {
        self.is_nested = true;
        self.nested_fields = nested_fields;
        self
    }
}

/// Builder for a config schema
#[derive(Debug, Default)]
pub struct ConfigSchemaBuilder {
    fields: Vec<FieldInfo>,
}

impl ConfigSchemaBuilder {
    /// Create a new config schema builder
    pub fn new() -> Self {
        Self { fields: Vec::new() }
    }

    /// Add a field to the schema
    pub fn add_field(mut self, field: FieldInfo) -> Self {
        self.fields.push(field);
        self
    }

    /// Build the schema
    pub fn build(self) -> ConfigSchema {
        ConfigSchema {
            fields: self.fields,
        }
    }
}

/// A schema describing a configuration structure
#[derive(Debug)]
pub struct ConfigSchema {
    pub fields: Vec<FieldInfo>,
}

impl ConfigSchema {
    /// Create a new builder for a config schema
    pub fn builder() -> ConfigSchemaBuilder {
        ConfigSchemaBuilder::new()
    }

    /// Generate markdown documentation for this schema with custom options
    pub fn generate_docs_with_options(&self, options: &MarkdownOptions) -> String {
        generate_markdown(&self.fields, options)
    }
}

/// Generate markdown documentation for a list of fields
pub fn generate_markdown(fields: &[FieldInfo], options: &MarkdownOptions) -> String {
    let mut buffer = String::new();

    if let Some(title) = &options.title {
        writeln!(buffer, "# {}", title).unwrap();
        writeln!(buffer).unwrap();
    }

    for field in fields {
        write_field_docs(&mut buffer, field, &options.format, 0, "").unwrap();
    }

    buffer
}

/// Write documentation for a field and its nested fields
fn write_field_docs(
    buffer: &mut String,
    field: &FieldInfo,
    format: &ConfigFormat,
    depth: usize,
    path: &str,
) -> fmt::Result {
    if field.is_nested {
        // Capitalize section name for header
        let section_name = capitalize(&field.name);

        writeln!(buffer, "## {}", section_name)?;

        if let Some(doc) = &field.doc_comments {
            writeln!(buffer)?;
            writeln!(buffer, "{}", doc)?;
        }

        match format {
            // #[cfg(toml)]
            ConfigFormat::Toml => {
                writeln!(buffer, "```toml")?;
                writeln!(buffer, "[{}]", field.name)?;
                writeln!(buffer)?;

                for nested_field in &field.nested_fields {
                    if !nested_field.is_nested {
                        if let Some(doc) = &nested_field.doc_comments {
                            for line in doc.lines() {
                                writeln!(buffer, "# {}", line)?;
                            }
                        }

                        if let Some(default) = &nested_field.default_value {
                            writeln!(buffer, "# Default: {}", default)?;
                        }

                        let value_str = match &nested_field.default_value {
                            Some(val) => format.format_value(&val),
                            None => "...".to_string(),
                        };

                        writeln!(buffer, "{} = {}", nested_field.name, value_str)?;
                        writeln!(buffer)?;
                    }
                }

                writeln!(buffer, "```")?;
            }
            _ => unimplemented!("no config format specified!!"),
        }

        writeln!(buffer)?;

        // Recursively document nested fields
        let current_path = if path.is_empty() {
            field.name.clone()
        } else {
            format!("{}.{}", path, field.name)
        };

        for nested_field in &field.nested_fields {
            if nested_field.is_nested {
                write_field_docs(buffer, nested_field, format, depth + 1, &current_path)?;
            }
        }
    }

    Ok(())
}

/// Capitalize the first letter of a string
fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Trait for structs that can generate config documentation
pub trait ConfigDocsStruct {
    /// Generate a schema describing this struct and its fields
    fn schema() -> ConfigSchema;
}

/// Generate markdown documentation with custom options for a type that implements ConfigDocsStruct
pub fn generate_config_docs_with_options<T: ConfigDocsStruct>(options: &MarkdownOptions) -> String {
    T::schema().generate_docs_with_options(options)
}

impl ConfigFormat {
    /// Get the file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            // #[cfg(toml)]
            ConfigFormat::Toml => "toml",
            _ => unimplemented!("no config format specified!!"),
        }
    }

    /// Format a value appropriately for this format
    pub fn format<T: Serialize + std::fmt::Debug>(&self, value: T) -> String {
        match self {
            // #[cfg(toml)]
            ConfigFormat::Toml => toml::to_string(dbg!(&value)).unwrap(),
            _ => unimplemented!("no config format specified!!"),
        }
    }

    pub fn format_value<T: Serialize>(&self, value: T) -> String {
        let mut res = String::new();
        serde::Serialize::serialize(&value, toml::ser::ValueSerializer::new(&mut res)).unwrap();
        res
    }
}
