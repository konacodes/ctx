use serde::Serialize;
use tree_sitter::{Node, Tree};

use super::treesitter::SupportedLanguage;

/// Represents a symbol extracted from source code via tree-sitter parsing.
///
/// Symbols include functions, classes, structs, enums, and other named
/// code elements that provide structural information about the codebase.
/// Each symbol captures its location, type, and optional metadata like
/// signatures and documentation.
///
/// # Fields
/// * `name` - The identifier name of the symbol
/// * `kind` - The type of symbol (function, class, etc.)
/// * `line` - The 1-indexed line number where the symbol is defined
/// * `signature` - Optional function/method signature (for callable symbols)
/// * `doc_comment` - Optional documentation comment extracted from source
#[derive(Debug, Clone, Serialize)]
pub struct Symbol {
    /// The identifier name of the symbol (e.g., function name, class name).
    pub name: String,
    /// The classification of this symbol (function, method, struct, etc.).
    pub kind: SymbolKind,
    /// The 1-indexed line number where this symbol is defined in the source file.
    pub line: usize,
    /// The function or method signature, if applicable. Contains the full
    /// declaration line up to (but not including) the body.
    pub signature: Option<String>,
    /// Documentation comment extracted from the source, if present.
    /// For Rust, this is `///` or `//!` comments. For Python, docstrings.
    pub doc_comment: Option<String>,
}

/// Classification of code symbols by their semantic role.
///
/// This enum categorizes the different types of named entities that can
/// be extracted from source code. The variants cover common programming
/// constructs across multiple languages (Rust, Python, JavaScript/TypeScript).
///
/// # Serialization
/// Variants are serialized to lowercase strings (e.g., `Function` -> `"function"`).
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SymbolKind {
    /// A standalone function (not associated with a type).
    Function,
    /// A method associated with a struct, class, or impl block.
    Method,
    /// A Rust struct or similar record type.
    Struct,
    /// A class definition (Python, JavaScript/TypeScript).
    Class,
    /// An enumeration type.
    Enum,
    /// A TypeScript interface definition.
    Interface,
    /// A Rust trait definition.
    Trait,
    /// A constant value declaration.
    Const,
    /// A variable declaration (primarily for JavaScript/TypeScript).
    Variable,
    /// A type alias definition.
    Type,
    /// A module declaration (Rust `mod` items).
    Module,
    /// An import statement (currently unused but reserved for future use).
    #[allow(dead_code)]
    Import,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolKind::Function => write!(f, "fn"),
            SymbolKind::Method => write!(f, "method"),
            SymbolKind::Struct => write!(f, "struct"),
            SymbolKind::Class => write!(f, "class"),
            SymbolKind::Enum => write!(f, "enum"),
            SymbolKind::Interface => write!(f, "interface"),
            SymbolKind::Trait => write!(f, "trait"),
            SymbolKind::Const => write!(f, "const"),
            SymbolKind::Variable => write!(f, "var"),
            SymbolKind::Type => write!(f, "type"),
            SymbolKind::Module => write!(f, "mod"),
            SymbolKind::Import => write!(f, "import"),
        }
    }
}

/// Extracts all symbols from a parsed syntax tree.
///
/// This function traverses the tree-sitter AST and identifies named code
/// elements such as functions, classes, structs, enums, and methods. The
/// extraction is language-aware and handles language-specific constructs
/// appropriately.
///
/// # Arguments
/// * `tree` - A parsed tree-sitter syntax tree
/// * `source` - The original source code (needed to extract text from nodes)
/// * `lang` - The programming language of the source file
///
/// # Returns
/// A vector of [`Symbol`] instances found in the source code, in the order
/// they appear in the file.
///
/// # Supported Languages
/// - Rust: functions, structs, enums, traits, impl methods, consts, types, modules
/// - Python: functions, classes, methods (with docstrings)
/// - JavaScript/TypeScript: functions, classes, methods, interfaces, type aliases, variables
pub fn extract_symbols(tree: &Tree, source: &str, lang: &SupportedLanguage) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    let root = tree.root_node();

    match lang {
        SupportedLanguage::Rust => extract_rust_symbols(&root, source, &mut symbols),
        SupportedLanguage::Python => extract_python_symbols(&root, source, &mut symbols),
        SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => {
            extract_js_symbols(&root, source, &mut symbols)
        }
    }

    symbols
}

fn extract_rust_symbols(node: &Node, source: &str, symbols: &mut Vec<Symbol>) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_item" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = get_text(&name_node, source);
                    let signature = get_function_signature(&child, source);
                    let doc = get_preceding_doc_comment(&child, source);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Function,
                        line: child.start_position().row + 1,
                        signature: Some(signature),
                        doc_comment: doc,
                    });
                }
            }
            "struct_item" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = get_text(&name_node, source);
                    let doc = get_preceding_doc_comment(&child, source);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Struct,
                        line: child.start_position().row + 1,
                        signature: None,
                        doc_comment: doc,
                    });
                }
            }
            "enum_item" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = get_text(&name_node, source);
                    let doc = get_preceding_doc_comment(&child, source);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Enum,
                        line: child.start_position().row + 1,
                        signature: None,
                        doc_comment: doc,
                    });
                }
            }
            "trait_item" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = get_text(&name_node, source);
                    let doc = get_preceding_doc_comment(&child, source);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Trait,
                        line: child.start_position().row + 1,
                        signature: None,
                        doc_comment: doc,
                    });
                }
            }
            "impl_item" => {
                extract_rust_impl_methods(&child, source, symbols);
            }
            "const_item" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = get_text(&name_node, source);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Const,
                        line: child.start_position().row + 1,
                        signature: None,
                        doc_comment: None,
                    });
                }
            }
            "type_item" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = get_text(&name_node, source);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Type,
                        line: child.start_position().row + 1,
                        signature: None,
                        doc_comment: None,
                    });
                }
            }
            "mod_item" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = get_text(&name_node, source);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Module,
                        line: child.start_position().row + 1,
                        signature: None,
                        doc_comment: None,
                    });
                }
            }
            _ => {
                extract_rust_symbols(&child, source, symbols);
            }
        }
    }
}

fn extract_rust_impl_methods(node: &Node, source: &str, symbols: &mut Vec<Symbol>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "declaration_list" {
            let mut inner_cursor = child.walk();
            for item in child.children(&mut inner_cursor) {
                if item.kind() == "function_item" {
                    if let Some(name_node) = item.child_by_field_name("name") {
                        let name = get_text(&name_node, source);
                        let signature = get_function_signature(&item, source);
                        let doc = get_preceding_doc_comment(&item, source);
                        symbols.push(Symbol {
                            name,
                            kind: SymbolKind::Method,
                            line: item.start_position().row + 1,
                            signature: Some(signature),
                            doc_comment: doc,
                        });
                    }
                }
            }
        }
    }
}

fn extract_python_symbols(node: &Node, source: &str, symbols: &mut Vec<Symbol>) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_definition" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = get_text(&name_node, source);
                    let signature = get_python_function_signature(&child, source);
                    let doc = get_python_docstring(&child, source);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Function,
                        line: child.start_position().row + 1,
                        signature: Some(signature),
                        doc_comment: doc,
                    });
                }
            }
            "class_definition" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = get_text(&name_node, source);
                    let doc = get_python_docstring(&child, source);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Class,
                        line: child.start_position().row + 1,
                        signature: None,
                        doc_comment: doc,
                    });
                }
                extract_python_class_methods(&child, source, symbols);
            }
            _ => {
                extract_python_symbols(&child, source, symbols);
            }
        }
    }
}

fn extract_python_class_methods(node: &Node, source: &str, symbols: &mut Vec<Symbol>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "block" {
            let mut inner_cursor = child.walk();
            for item in child.children(&mut inner_cursor) {
                if item.kind() == "function_definition" {
                    if let Some(name_node) = item.child_by_field_name("name") {
                        let name = get_text(&name_node, source);
                        let signature = get_python_function_signature(&item, source);
                        let doc = get_python_docstring(&item, source);
                        symbols.push(Symbol {
                            name,
                            kind: SymbolKind::Method,
                            line: item.start_position().row + 1,
                            signature: Some(signature),
                            doc_comment: doc,
                        });
                    }
                }
            }
        }
    }
}

fn extract_js_symbols(node: &Node, source: &str, symbols: &mut Vec<Symbol>) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_declaration" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = get_text(&name_node, source);
                    let signature = get_js_function_signature(&child, source);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Function,
                        line: child.start_position().row + 1,
                        signature: Some(signature),
                        doc_comment: None,
                    });
                }
            }
            "class_declaration" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = get_text(&name_node, source);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Class,
                        line: child.start_position().row + 1,
                        signature: None,
                        doc_comment: None,
                    });
                }
                extract_js_class_methods(&child, source, symbols);
            }
            "interface_declaration" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = get_text(&name_node, source);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Interface,
                        line: child.start_position().row + 1,
                        signature: None,
                        doc_comment: None,
                    });
                }
            }
            "type_alias_declaration" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = get_text(&name_node, source);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Type,
                        line: child.start_position().row + 1,
                        signature: None,
                        doc_comment: None,
                    });
                }
            }
            "lexical_declaration" | "variable_declaration" => {
                extract_js_variables(&child, source, symbols);
            }
            "export_statement" => {
                extract_js_symbols(&child, source, symbols);
            }
            _ => {
                extract_js_symbols(&child, source, symbols);
            }
        }
    }
}

fn extract_js_class_methods(node: &Node, source: &str, symbols: &mut Vec<Symbol>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "class_body" {
            let mut inner_cursor = child.walk();
            for item in child.children(&mut inner_cursor) {
                if item.kind() == "method_definition" {
                    if let Some(name_node) = item.child_by_field_name("name") {
                        let name = get_text(&name_node, source);
                        symbols.push(Symbol {
                            name,
                            kind: SymbolKind::Method,
                            line: item.start_position().row + 1,
                            signature: None,
                            doc_comment: None,
                        });
                    }
                }
            }
        }
    }
}

fn extract_js_variables(node: &Node, source: &str, symbols: &mut Vec<Symbol>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            if let Some(name_node) = child.child_by_field_name("name") {
                let name = get_text(&name_node, source);
                // Check if it's a function expression or arrow function
                if let Some(value) = child.child_by_field_name("value") {
                    let kind = match value.kind() {
                        "arrow_function" | "function" => SymbolKind::Function,
                        _ => SymbolKind::Variable,
                    };
                    symbols.push(Symbol {
                        name,
                        kind,
                        line: child.start_position().row + 1,
                        signature: None,
                        doc_comment: None,
                    });
                }
            }
        }
    }
}

fn get_text(node: &Node, source: &str) -> String {
    source[node.byte_range()].to_string()
}

fn get_function_signature(node: &Node, source: &str) -> String {
    let start = node.start_byte();
    let text = &source[start..];

    // Find the opening brace
    if let Some(brace_pos) = text.find('{') {
        let sig = text[..brace_pos].trim();
        return sig.to_string();
    }

    // Fallback: get until end of line
    text.lines().next().unwrap_or("").to_string()
}

fn get_python_function_signature(node: &Node, source: &str) -> String {
    let start = node.start_byte();
    let text = &source[start..];

    // Find the colon that ends the signature
    if let Some(colon_pos) = text.find(':') {
        let sig = text[..colon_pos].trim();
        return sig.to_string();
    }

    text.lines().next().unwrap_or("").to_string()
}

fn get_js_function_signature(node: &Node, source: &str) -> String {
    let start = node.start_byte();
    let text = &source[start..];

    // Find the opening brace
    if let Some(brace_pos) = text.find('{') {
        let sig = text[..brace_pos].trim();
        return sig.to_string();
    }

    text.lines().next().unwrap_or("").to_string()
}

fn get_preceding_doc_comment(node: &Node, source: &str) -> Option<String> {
    let mut prev = node.prev_sibling();

    while let Some(sibling) = prev {
        match sibling.kind() {
            "line_comment" => {
                let text = get_text(&sibling, source);
                if text.starts_with("///") || text.starts_with("//!") {
                    return Some(text[3..].trim().to_string());
                }
            }
            "block_comment" => {
                let text = get_text(&sibling, source);
                if text.starts_with("/**") {
                    // Extract doc comment content
                    let content = text
                        .trim_start_matches("/**")
                        .trim_end_matches("*/")
                        .lines()
                        .map(|l| l.trim().trim_start_matches('*').trim())
                        .collect::<Vec<_>>()
                        .join(" ");
                    return Some(content);
                }
            }
            _ => break,
        }
        prev = sibling.prev_sibling();
    }

    None
}

fn get_python_docstring(node: &Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "block" {
            let mut inner_cursor = child.walk();
            for item in child.children(&mut inner_cursor) {
                if item.kind() == "expression_statement" {
                    let mut expr_cursor = item.walk();
                    for expr in item.children(&mut expr_cursor) {
                        if expr.kind() == "string" {
                            let text = get_text(&expr, source);
                            // Clean up the docstring
                            let content = text
                                .trim_start_matches("\"\"\"")
                                .trim_start_matches("'''")
                                .trim_end_matches("\"\"\"")
                                .trim_end_matches("'''")
                                .trim();
                            if !content.is_empty() {
                                return Some(content.lines().next().unwrap_or("").to_string());
                            }
                        }
                    }
                }
                break; // Only check first statement
            }
        }
    }
    None
}

/// Generates a skeleton (outline) view of the source code structure.
///
/// This function creates a condensed representation of the code that shows
/// only the declarations and signatures without implementation details.
/// It's useful for getting a quick overview of a file's API and structure.
///
/// # Arguments
/// * `tree` - A parsed tree-sitter syntax tree
/// * `source` - The original source code
/// * `lang` - The programming language of the source file
///
/// # Returns
/// A string containing the skeleton representation with:
/// - Function signatures followed by `{ ... }`
/// - Struct/enum/class declarations with `{ ... }` bodies
/// - Proper indentation to show nesting (impl blocks, class methods)
///
/// # Example Output (Rust)
/// ```text
/// pub fn main() { ... }
/// impl MyStruct {
///     fn new() -> Self { ... }
///     fn method(&self) { ... }
/// }
/// ```
pub fn get_skeleton(tree: &Tree, source: &str, lang: &SupportedLanguage) -> String {
    let mut result = String::new();
    let root = tree.root_node();

    match lang {
        SupportedLanguage::Rust => get_rust_skeleton(&root, source, &mut result, 0),
        SupportedLanguage::Python => get_python_skeleton(&root, source, &mut result, 0),
        SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => {
            get_js_skeleton(&root, source, &mut result, 0)
        }
    }

    result
}

fn get_rust_skeleton(node: &Node, source: &str, result: &mut String, indent: usize) {
    let mut cursor = node.walk();
    let indent_str = "    ".repeat(indent);

    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_item" => {
                let sig = get_function_signature(&child, source);
                result.push_str(&format!("{}{} {{ ... }}\n", indent_str, sig));
            }
            "struct_item" | "enum_item" | "trait_item" => {
                let start = child.start_byte();
                let text = &source[start..];
                if let Some(brace_pos) = text.find('{') {
                    let sig = text[..brace_pos].trim();
                    result.push_str(&format!("{}{} {{ ... }}\n", indent_str, sig));
                }
            }
            "impl_item" => {
                let start = child.start_byte();
                let text = &source[start..];
                if let Some(brace_pos) = text.find('{') {
                    let sig = text[..brace_pos].trim();
                    result.push_str(&format!("{}{} {{\n", indent_str, sig));
                    get_rust_skeleton(&child, source, result, indent + 1);
                    result.push_str(&format!("{}}}\n", indent_str));
                }
            }
            "declaration_list" => {
                let mut inner_cursor = child.walk();
                for item in child.children(&mut inner_cursor) {
                    if item.kind() == "function_item" {
                        let sig = get_function_signature(&item, source);
                        result.push_str(&format!("{}{} {{ ... }}\n", indent_str, sig));
                    }
                }
            }
            _ => {}
        }
    }
}

fn get_python_skeleton(node: &Node, source: &str, result: &mut String, indent: usize) {
    let mut cursor = node.walk();
    let indent_str = "    ".repeat(indent);

    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_definition" => {
                let sig = get_python_function_signature(&child, source);
                result.push_str(&format!("{}{}:\n{}    ...\n", indent_str, sig, indent_str));
            }
            "class_definition" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = get_text(&name_node, source);
                    result.push_str(&format!("{}class {}:\n", indent_str, name));
                    get_python_skeleton(&child, source, result, indent + 1);
                }
            }
            "block" => {
                get_python_skeleton(&child, source, result, indent);
            }
            _ => {}
        }
    }
}

fn get_js_skeleton(node: &Node, source: &str, result: &mut String, indent: usize) {
    let mut cursor = node.walk();
    let indent_str = "    ".repeat(indent);

    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_declaration" => {
                let sig = get_js_function_signature(&child, source);
                result.push_str(&format!("{}{} {{ ... }}\n", indent_str, sig));
            }
            "class_declaration" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = get_text(&name_node, source);
                    result.push_str(&format!("{}class {} {{\n", indent_str, name));
                    get_js_skeleton(&child, source, result, indent + 1);
                    result.push_str(&format!("{}}}\n", indent_str));
                }
            }
            "class_body" => {
                let mut inner_cursor = child.walk();
                for item in child.children(&mut inner_cursor) {
                    if item.kind() == "method_definition" {
                        if let Some(name_node) = item.child_by_field_name("name") {
                            let name = get_text(&name_node, source);
                            result.push_str(&format!("{}{}() {{ ... }}\n", indent_str, name));
                        }
                    }
                }
            }
            "export_statement" => {
                get_js_skeleton(&child, source, result, indent);
            }
            _ => {}
        }
    }
}

/// Extracts all import/use statements from a parsed syntax tree.
///
/// This function scans the top level of the AST to find import declarations,
/// which are useful for understanding a file's dependencies and relationships
/// to other modules.
///
/// # Arguments
/// * `tree` - A parsed tree-sitter syntax tree
/// * `source` - The original source code
/// * `lang` - The programming language of the source file
///
/// # Returns
/// A vector of strings, each containing the full text of an import statement.
///
/// # Language-Specific Behavior
/// - **Rust**: Extracts `use` declarations (e.g., `use std::path::Path;`)
/// - **Python**: Extracts `import` and `from ... import` statements
/// - **JavaScript/TypeScript**: Extracts `import` statements
pub fn find_imports(tree: &Tree, source: &str, lang: &SupportedLanguage) -> Vec<String> {
    let mut imports = Vec::new();
    let root = tree.root_node();

    match lang {
        SupportedLanguage::Rust => find_rust_imports(&root, source, &mut imports),
        SupportedLanguage::Python => find_python_imports(&root, source, &mut imports),
        SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => {
            find_js_imports(&root, source, &mut imports)
        }
    }

    imports
}

fn find_rust_imports(node: &Node, source: &str, imports: &mut Vec<String>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "use_declaration" {
            let text = get_text(&child, source);
            imports.push(text);
        }
    }
}

fn find_python_imports(node: &Node, source: &str, imports: &mut Vec<String>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "import_statement" | "import_from_statement" => {
                let text = get_text(&child, source);
                imports.push(text);
            }
            _ => {}
        }
    }
}

fn find_js_imports(node: &Node, source: &str, imports: &mut Vec<String>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "import_statement" {
            let text = get_text(&child, source);
            imports.push(text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn parse_rust(source: &str) -> Tree {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        parser.parse(source, None).unwrap()
    }

    fn parse_python(source: &str) -> Tree {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_python::language()).unwrap();
        parser.parse(source, None).unwrap()
    }

    fn parse_javascript(source: &str) -> Tree {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_javascript::language()).unwrap();
        parser.parse(source, None).unwrap()
    }

    #[test]
    fn test_symbol_kind_display() {
        // Test all SymbolKind variants display correctly
        assert_eq!(format!("{}", SymbolKind::Function), "fn");
        assert_eq!(format!("{}", SymbolKind::Method), "method");
        assert_eq!(format!("{}", SymbolKind::Struct), "struct");
        assert_eq!(format!("{}", SymbolKind::Class), "class");
        assert_eq!(format!("{}", SymbolKind::Enum), "enum");
        assert_eq!(format!("{}", SymbolKind::Interface), "interface");
        assert_eq!(format!("{}", SymbolKind::Trait), "trait");
        assert_eq!(format!("{}", SymbolKind::Const), "const");
        assert_eq!(format!("{}", SymbolKind::Variable), "var");
        assert_eq!(format!("{}", SymbolKind::Type), "type");
        assert_eq!(format!("{}", SymbolKind::Module), "mod");
        assert_eq!(format!("{}", SymbolKind::Import), "import");

        // Test that Display can be used in string formatting
        let kind = SymbolKind::Function;
        let formatted = format!("Symbol type: {}", kind);
        assert_eq!(formatted, "Symbol type: fn");

        // Test multiple kinds in a single format string
        let output = format!("{} vs {}", SymbolKind::Struct, SymbolKind::Class);
        assert_eq!(output, "struct vs class");
    }

    #[test]
    fn test_extract_rust_functions() {
        let source = r#"
fn hello() {
    println!("Hello");
}

pub fn greet(name: &str) -> String {
    format!("Hello, {}", name)
}

async fn fetch_data() -> Result<Data, Error> {
    // async function
}
"#;
        let tree = parse_rust(source);
        let symbols = extract_symbols(&tree, source, &SupportedLanguage::Rust);

        assert_eq!(symbols.len(), 3);

        // Check first function
        let hello = symbols.iter().find(|s| s.name == "hello").unwrap();
        assert_eq!(hello.kind, SymbolKind::Function);
        assert!(hello.signature.is_some());
        assert!(hello.signature.as_ref().unwrap().contains("fn hello()"));

        // Check second function with parameters
        let greet = symbols.iter().find(|s| s.name == "greet").unwrap();
        assert_eq!(greet.kind, SymbolKind::Function);
        assert!(greet.signature.as_ref().unwrap().contains("pub fn greet"));
        assert!(greet.signature.as_ref().unwrap().contains("name: &str"));

        // Check async function
        let fetch = symbols.iter().find(|s| s.name == "fetch_data").unwrap();
        assert_eq!(fetch.kind, SymbolKind::Function);
        assert!(fetch.signature.as_ref().unwrap().contains("async fn"));
    }

    #[test]
    fn test_extract_rust_structs_and_enums() {
        let source = r#"
struct Point {
    x: i32,
    y: i32,
}

pub struct User {
    name: String,
    age: u32,
}

enum Color {
    Red,
    Green,
    Blue,
}

pub enum Result<T, E> {
    Ok(T),
    Err(E),
}
"#;
        let tree = parse_rust(source);
        let symbols = extract_symbols(&tree, source, &SupportedLanguage::Rust);

        // Should find 2 structs and 2 enums
        let structs: Vec<_> = symbols.iter().filter(|s| s.kind == SymbolKind::Struct).collect();
        let enums: Vec<_> = symbols.iter().filter(|s| s.kind == SymbolKind::Enum).collect();

        assert_eq!(structs.len(), 2);
        assert_eq!(enums.len(), 2);

        // Check struct names
        assert!(structs.iter().any(|s| s.name == "Point"));
        assert!(structs.iter().any(|s| s.name == "User"));

        // Check enum names
        assert!(enums.iter().any(|s| s.name == "Color"));
        assert!(enums.iter().any(|s| s.name == "Result"));
    }

    #[test]
    fn test_extract_rust_traits_and_impls() {
        let source = r#"
trait Drawable {
    fn draw(&self);
}

impl Drawable for Circle {
    fn draw(&self) {
        // implementation
    }
}

impl Circle {
    fn new(radius: f64) -> Self {
        Circle { radius }
    }

    pub fn area(&self) -> f64 {
        std::f64::consts::PI * self.radius * self.radius
    }
}
"#;
        let tree = parse_rust(source);
        let symbols = extract_symbols(&tree, source, &SupportedLanguage::Rust);

        // Should find trait
        let traits: Vec<_> = symbols.iter().filter(|s| s.kind == SymbolKind::Trait).collect();
        assert_eq!(traits.len(), 1);
        assert_eq!(traits[0].name, "Drawable");

        // Should find methods from impl blocks
        let methods: Vec<_> = symbols.iter().filter(|s| s.kind == SymbolKind::Method).collect();
        assert!(methods.len() >= 3);
        assert!(methods.iter().any(|s| s.name == "draw"));
        assert!(methods.iter().any(|s| s.name == "new"));
        assert!(methods.iter().any(|s| s.name == "area"));
    }

    #[test]
    fn test_extract_rust_consts_and_types() {
        let source = r#"
const MAX_SIZE: usize = 100;
const DEFAULT_NAME: &str = "Unknown";

type UserId = u64;
type Callback = fn(i32) -> i32;
"#;
        let tree = parse_rust(source);
        let symbols = extract_symbols(&tree, source, &SupportedLanguage::Rust);

        let consts: Vec<_> = symbols.iter().filter(|s| s.kind == SymbolKind::Const).collect();
        let types: Vec<_> = symbols.iter().filter(|s| s.kind == SymbolKind::Type).collect();

        assert_eq!(consts.len(), 2);
        assert!(consts.iter().any(|s| s.name == "MAX_SIZE"));
        assert!(consts.iter().any(|s| s.name == "DEFAULT_NAME"));

        assert_eq!(types.len(), 2);
        assert!(types.iter().any(|s| s.name == "UserId"));
        assert!(types.iter().any(|s| s.name == "Callback"));
    }

    #[test]
    fn test_extract_rust_modules() {
        let source = r#"
mod utils;
pub mod helpers;

mod internal {
    fn hidden() {}
}
"#;
        let tree = parse_rust(source);
        let symbols = extract_symbols(&tree, source, &SupportedLanguage::Rust);

        let modules: Vec<_> = symbols.iter().filter(|s| s.kind == SymbolKind::Module).collect();
        assert!(modules.len() >= 2);
        assert!(modules.iter().any(|s| s.name == "utils"));
        assert!(modules.iter().any(|s| s.name == "helpers"));
    }

    #[test]
    fn test_extract_python_functions_and_classes() {
        let source = r#"
def hello():
    print("Hello")

def greet(name: str) -> str:
    return f"Hello, {name}"

class User:
    def __init__(self, name):
        self.name = name

    def get_name(self):
        return self.name

class Admin(User):
    def __init__(self, name, role):
        super().__init__(name)
        self.role = role
"#;
        let tree = parse_python(source);
        let symbols = extract_symbols(&tree, source, &SupportedLanguage::Python);

        // Check functions
        let functions: Vec<_> = symbols.iter().filter(|s| s.kind == SymbolKind::Function).collect();
        assert!(functions.len() >= 2);
        assert!(functions.iter().any(|s| s.name == "hello"));
        assert!(functions.iter().any(|s| s.name == "greet"));

        // Check classes
        let classes: Vec<_> = symbols.iter().filter(|s| s.kind == SymbolKind::Class).collect();
        assert_eq!(classes.len(), 2);
        assert!(classes.iter().any(|s| s.name == "User"));
        assert!(classes.iter().any(|s| s.name == "Admin"));

        // Check methods
        let methods: Vec<_> = symbols.iter().filter(|s| s.kind == SymbolKind::Method).collect();
        assert!(methods.iter().any(|s| s.name == "__init__"));
        assert!(methods.iter().any(|s| s.name == "get_name"));
    }

    #[test]
    fn test_extract_javascript_symbols() {
        let source = r#"
function hello() {
    console.log("Hello");
}

function greet(name) {
    return `Hello, ${name}`;
}

class User {
    constructor(name) {
        this.name = name;
    }

    getName() {
        return this.name;
    }
}

const add = (a, b) => a + b;
let counter = 0;
"#;
        let tree = parse_javascript(source);
        let symbols = extract_symbols(&tree, source, &SupportedLanguage::JavaScript);

        // Check functions
        let functions: Vec<_> = symbols.iter().filter(|s| s.kind == SymbolKind::Function).collect();
        assert!(functions.iter().any(|s| s.name == "hello"));
        assert!(functions.iter().any(|s| s.name == "greet"));
        assert!(functions.iter().any(|s| s.name == "add")); // Arrow function

        // Check class
        let classes: Vec<_> = symbols.iter().filter(|s| s.kind == SymbolKind::Class).collect();
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].name, "User");

        // Check methods
        let methods: Vec<_> = symbols.iter().filter(|s| s.kind == SymbolKind::Method).collect();
        assert!(methods.iter().any(|s| s.name == "constructor"));
        assert!(methods.iter().any(|s| s.name == "getName"));

        // Check variable
        let variables: Vec<_> = symbols.iter().filter(|s| s.kind == SymbolKind::Variable).collect();
        assert!(variables.iter().any(|s| s.name == "counter"));
    }

    #[test]
    fn test_symbol_line_numbers() {
        let source = r#"fn first() {}

fn second() {}

fn third() {}"#;
        let tree = parse_rust(source);
        let symbols = extract_symbols(&tree, source, &SupportedLanguage::Rust);

        let first = symbols.iter().find(|s| s.name == "first").unwrap();
        let second = symbols.iter().find(|s| s.name == "second").unwrap();
        let third = symbols.iter().find(|s| s.name == "third").unwrap();

        assert_eq!(first.line, 1);
        assert_eq!(second.line, 3);
        assert_eq!(third.line, 5);
    }

    #[test]
    fn test_rust_doc_comments() {
        let source = r#"
/// This is a documented function
fn documented() {}

/// Multi-line doc comment
/// with additional details
fn multi_doc() {}

fn undocumented() {}
"#;
        let tree = parse_rust(source);
        let symbols = extract_symbols(&tree, source, &SupportedLanguage::Rust);

        let documented = symbols.iter().find(|s| s.name == "documented").unwrap();
        assert!(documented.doc_comment.is_some());
        assert!(documented.doc_comment.as_ref().unwrap().contains("documented function"));

        let undocumented = symbols.iter().find(|s| s.name == "undocumented").unwrap();
        assert!(undocumented.doc_comment.is_none());
    }

    #[test]
    fn test_find_rust_imports() {
        let source = r#"
use std::collections::HashMap;
use std::path::Path;
use crate::utils::helper;

fn main() {}
"#;
        let tree = parse_rust(source);
        let imports = find_imports(&tree, source, &SupportedLanguage::Rust);

        assert_eq!(imports.len(), 3);
        assert!(imports.iter().any(|i| i.contains("std::collections::HashMap")));
        assert!(imports.iter().any(|i| i.contains("std::path::Path")));
        assert!(imports.iter().any(|i| i.contains("crate::utils::helper")));
    }

    #[test]
    fn test_find_python_imports() {
        let source = r#"
import os
import sys
from collections import defaultdict
from typing import List, Dict

def main():
    pass
"#;
        let tree = parse_python(source);
        let imports = find_imports(&tree, source, &SupportedLanguage::Python);

        assert!(imports.len() >= 4);
        assert!(imports.iter().any(|i| i.contains("import os")));
        assert!(imports.iter().any(|i| i.contains("import sys")));
        assert!(imports.iter().any(|i| i.contains("from collections")));
        assert!(imports.iter().any(|i| i.contains("from typing")));
    }

    #[test]
    fn test_find_javascript_imports() {
        let source = r#"
import React from 'react';
import { useState, useEffect } from 'react';
import * as utils from './utils';

function App() {}
"#;
        let tree = parse_javascript(source);
        let imports = find_imports(&tree, source, &SupportedLanguage::JavaScript);

        assert_eq!(imports.len(), 3);
        assert!(imports.iter().any(|i| i.contains("React")));
        assert!(imports.iter().any(|i| i.contains("useState")));
        assert!(imports.iter().any(|i| i.contains("utils")));
    }

    #[test]
    fn test_get_skeleton_rust() {
        let source = r#"
fn hello() {
    println!("Hello");
}

struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }
}
"#;
        let tree = parse_rust(source);
        let skeleton = get_skeleton(&tree, source, &SupportedLanguage::Rust);

        // Skeleton should contain function signatures with { ... }
        assert!(skeleton.contains("fn hello()"));
        assert!(skeleton.contains("{ ... }"));
        assert!(skeleton.contains("struct Point"));
        assert!(skeleton.contains("impl Point"));
        assert!(skeleton.contains("fn new"));
    }

    #[test]
    fn test_get_skeleton_python() {
        let source = r#"
def hello():
    print("Hello")

class User:
    def __init__(self, name):
        self.name = name
"#;
        let tree = parse_python(source);
        let skeleton = get_skeleton(&tree, source, &SupportedLanguage::Python);

        assert!(skeleton.contains("def hello()"));
        assert!(skeleton.contains("class User"));
        assert!(skeleton.contains("def __init__"));
    }

    #[test]
    fn test_empty_source() {
        let source = "";
        let tree = parse_rust(source);
        let symbols = extract_symbols(&tree, source, &SupportedLanguage::Rust);
        assert!(symbols.is_empty());

        let imports = find_imports(&tree, source, &SupportedLanguage::Rust);
        assert!(imports.is_empty());

        let skeleton = get_skeleton(&tree, source, &SupportedLanguage::Rust);
        assert!(skeleton.is_empty());
    }
}
