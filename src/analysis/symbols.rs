use serde::Serialize;
use tree_sitter::{Node, Tree};

use super::treesitter::SupportedLanguage;

#[derive(Debug, Clone, Serialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
    pub signature: Option<String>,
    pub doc_comment: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Class,
    Enum,
    Interface,
    Trait,
    Const,
    Variable,
    Type,
    Module,
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
