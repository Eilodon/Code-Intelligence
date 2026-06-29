pub struct LangConstants {
    pub function_node_types: &'static [&'static str],
    pub name_field: &'static str,
    pub docstring_type: Option<&'static str>,
    /// Node kinds that represent a call / invocation site.
    pub call_node_types: &'static [&'static str],
    /// Field name on a call node that holds the callee expression (the called function).
    pub call_function_field: &'static str,
}

pub fn get_lang_constants(lang: &str) -> Option<LangConstants> {
    match lang {
        "python" => Some(LangConstants {
            function_node_types: &["function_definition", "class_definition"],
            name_field: "name",
            docstring_type: Some("expression_statement"), // Python docstrings are expression_statements
            call_node_types: &["call"],
            call_function_field: "function",
        }),
        "rust" => Some(LangConstants {
            function_node_types: &["function_item", "struct_item", "trait_item", "impl_item"],
            name_field: "name",
            docstring_type: Some("line_comment"),
            call_node_types: &["call_expression"],
            call_function_field: "function",
        }),
        "go" => Some(LangConstants {
            function_node_types: &[
                "function_declaration",
                "method_declaration",
                "type_declaration",
            ],
            name_field: "name",
            docstring_type: Some("comment"),
            call_node_types: &["call_expression"],
            call_function_field: "function",
        }),
        "javascript" | "typescript" => Some(LangConstants {
            function_node_types: &[
                "function_declaration",
                "class_declaration",
                "method_definition",
                "lexical_declaration",
            ],
            name_field: "name",
            docstring_type: Some("comment"),
            call_node_types: &["call_expression"],
            call_function_field: "function",
        }),
        "java" => Some(LangConstants {
            function_node_types: &[
                "method_declaration",
                "class_declaration",
                "interface_declaration",
            ],
            name_field: "name",
            docstring_type: Some("block_comment"),
            call_node_types: &["method_invocation"],
            call_function_field: "name",
        }),
        _ => None,
    }
}

/// Map a file extension to a tier-0 language id, or `None` if unsupported.
pub fn language_for_extension(ext: &str) -> Option<&'static str> {
    match ext {
        "py" => Some("python"),
        "rs" => Some("rust"),
        "go" => Some("go"),
        "js" | "jsx" | "mjs" | "cjs" => Some("javascript"),
        "ts" | "tsx" => Some("typescript"),
        "java" => Some("java"),
        _ => None,
    }
}
