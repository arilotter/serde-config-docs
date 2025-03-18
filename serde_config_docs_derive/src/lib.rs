use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, ExprPath, Field, Fields, Lit, Meta, MetaNameValue, NestedMeta, Type
};

#[proc_macro_derive(ConfigDocs, attributes(serde, doc, config_docs))]
pub fn derive_config_docs(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Extract struct name
    let struct_name = &input.ident;
    let struct_name_str = struct_name.to_string();

    // Check if export is enabled
    let should_export = input.attrs.iter().any(|attr| {
        if attr.path.is_ident("config_docs") {
            if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                for nested in meta_list.nested.iter() {
                    if let NestedMeta::Meta(Meta::Path(path)) = nested {
                        if path.is_ident("export") {
                            return true;
                        }
                    }
                }
            }
            return false;
        }
        false
    });

    // Extract struct-level rename_all
    let rename_all = extract_rename_all(&input.attrs);

    // Process fields
    let fields_tokens = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => process_fields(&fields.named, &rename_all),
            _ => panic!("Only named fields are supported"),
        },
        _ => panic!("ConfigDocs can only be derived for structs"),
    };

    // Generate the trait implementation
    let trait_impl = quote! {
        impl serde_config_docs::ConfigDocsStruct for #struct_name {
            fn schema() -> serde_config_docs::ConfigSchema {
                serde_config_docs::ConfigSchema::builder()
                    #fields_tokens
                    .build()
            }
        }
    };

    // If export is enabled, also generate a test function
    let test_fn = if should_export {
        let test_name = format_ident!("export_serde_docs_{}", struct_name_str.to_lowercase());

        quote! {
            #[cfg(test)]
            mod config_docs_tests {
                use super::*;
                
                #[test]
                fn #test_name() {
                    use std::path::Path;
                    use std::fs;
                    use std::fs::File;
                    use std::io::Write;
                    use std::env;

                    // Get format from environment variable or default to "toml"
                    let format_str = env::var("CONFIG_DOCS_FORMAT").unwrap_or_else(|_| "toml".to_string());
                    
                    // Parse format string
                    let format = match format_str.to_lowercase().as_str() {
                        // #[cfg(toml)]
                        "toml" => serde_config_docs::ConfigFormat::Toml,
                        _ => {
                            unimplemented!("Unsupported format '{}'", format_str);
                        }
                    };
                    
                    // Generate file name based on format
                    let file_name = format!("{}.{}.md", #struct_name_str, format.extension());
                    
                    let options = serde_config_docs::MarkdownOptions::new(format);
                        
                    let docs = serde_config_docs::generate_config_docs_with_options::<#struct_name>(&options);
                    
                    let file_path = Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("docs")
                        .join(file_name);
                    
                    // Create docs directory if it doesn't exist
                    if let Some(parent) = file_path.parent() {
                        fs::create_dir_all(parent).expect("Failed to create docs directory");
                    }
                    
                    let mut file = File::create(&file_path)
                        .expect("Failed to create documentation file");
                        
                    file.write_all(docs.as_bytes())
                        .expect("Failed to write documentation");
                    
                    println!("Generated documentation: {}", file_path.display());
                }
            }
        }
    } else {
        quote! {}
    };

    // Combine trait implementation with optional test function
    let output = quote! {
        #trait_impl
        
        #test_fn
    };

    output.into()
}

// Simplify process_fields to not need format at compile time
fn process_fields(
    fields: &syn::punctuated::Punctuated<Field, syn::token::Comma>,
    rename_all: &Option<String>,
) -> proc_macro2::TokenStream {
    let field_tokens = fields.iter().map(|field| {
        // Get field name
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();

        // Extract doc comments
        let doc_comment = extract_doc_comment(&field.attrs);

        // Extract serde attributes
        let rename = extract_rename(&field.attrs);
        let default_fn = extract_default_fn(&field.attrs);

        // Determine final field name after rename attributes
        let final_name = match rename {
            Some(name) => name,
            None => apply_rename_all(&field_name_str, rename_all),
        };

        // Get field type info
        let field_type_str = get_field_type_str(&field.ty);
        let is_nested = is_nested_type(&field.ty);

        if is_nested {
            // For nested fields, we need to recursively process them
            let nested_type_name = get_type_name(&field.ty);
            let nested_type_ident = format_ident!("{}", nested_type_name);

            quote! {
                .add_field(
                    serde_config_docs::FieldInfo::new(#final_name)
                        // .doc(#doc_comment)
                        .field_type(#field_type_str)
                        .nested(<#nested_type_ident as serde_config_docs::ConfigDocsStruct>::schema().fields)
                )
            }
        } else {
            let default_value_expr = match default_fn {
                Some(path) => {
                    // Create an expression to call the default function
                    let default_fn_path = syn::parse_str::<ExprPath>(&path).unwrap_or_else(|_| {
                        panic!("Failed to parse default function path: {}", path)
                    });
                    
                    quote! {
                        Some({
                            // Get the default value and convert to a string
                            let default_value = #default_fn_path();
                            format!("{:?}", default_value)
                        })
                    }
                },
                None => {
                    quote! { None }
                }
            };

            quote! {
                .add_field(
                    serde_config_docs::FieldInfo::new(#final_name)
                        // .doc(#doc_comment)
                        .default(#default_value_expr)
                        .field_type(#field_type_str)
                )
            }
        }
    });

    quote! {
        #(#field_tokens)*
    }
}

fn extract_doc_comment(attrs: &[Attribute]) -> Option<String> {
    let mut doc_lines = Vec::new();

    for attr in attrs {
        if attr.path.is_ident("doc") {
            if let Ok(Meta::NameValue(MetaNameValue {
                lit: Lit::Str(lit_str),
                ..
            })) = attr.parse_meta()
            {
                doc_lines.push(lit_str.value());
            }
        }
    }

    if doc_lines.is_empty() {
        None
    } else {
        Some(doc_lines.join("\n"))
    }
}


// Extract the default function path from serde attributes
fn extract_default_fn(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path.is_ident("serde") {
            if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                for nested in meta_list.nested {
                    if let NestedMeta::Meta(Meta::NameValue(name_value)) = nested {
                        if name_value.path.is_ident("default") {
                            if let Lit::Str(lit_str) = name_value.lit {
                                return Some(lit_str.value());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}



fn extract_rename(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path.is_ident("serde") {
            if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                for nested in meta_list.nested {
                    if let NestedMeta::Meta(Meta::NameValue(name_value)) = nested {
                        if name_value.path.is_ident("rename") {
                            if let Lit::Str(lit_str) = name_value.lit {
                                return Some(lit_str.value());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}


fn extract_rename_all(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path.is_ident("serde") {
            if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                for nested in meta_list.nested {
                    if let NestedMeta::Meta(Meta::NameValue(name_value)) = nested {
                        if name_value.path.is_ident("rename_all") {
                            if let Lit::Str(lit_str) = name_value.lit {
                                return Some(lit_str.value());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn is_nested_type(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) if type_path.path.segments.len() == 1 => {
            let type_name = type_path.path.segments[0].ident.to_string();
            // Primitive types are not nested
            !matches!(
                type_name.as_str(),
                "bool"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "isize"
                    | "u8"
                    | "u16"
                    | "u32"
                    | "u64"
                    | "u128"
                    | "usize"
                    | "f32"
                    | "f64"
                    | "char"
                    | "str"
                    | "String"
            )
        }
        _ => false,
    }
}

fn get_field_type_str(ty: &Type) -> String {
    match ty {
        Type::Path(type_path) if type_path.path.segments.len() == 1 => {
            type_path.path.segments[0].ident.to_string()
        }
        _ => "unknown".to_string(),
    }
}

fn get_type_name(ty: &Type) -> String {
    match ty {
        Type::Path(type_path) if type_path.path.segments.len() == 1 => {
            type_path.path.segments[0].ident.to_string()
        }
        _ => "Unknown".to_string(),
    }
}

fn apply_rename_all(field_name: &str, rename_all: &Option<String>) -> String {
    if let Some(style) = rename_all {
        match style.as_str() {
            "camelCase" => to_camel_case(field_name),
            "PascalCase" => to_pascal_case(field_name),
            "snake_case" => field_name.to_string(), // already snake case
            "SCREAMING_SNAKE_CASE" => field_name.to_uppercase(),
            "kebab-case" => field_name.replace('_', "-"),
            _ => field_name.to_string(),
        }
    } else {
        field_name.to_string()
    }
}

fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}