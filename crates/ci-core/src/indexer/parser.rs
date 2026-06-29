use crate::types::{SymbolKind, IndexingPhase};

pub struct ParsedSymbol {
    pub qualified_name: String,
    pub name: String,
    pub kind: SymbolKind,
    pub language: String,
    pub path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub signature: String,
    pub docstring: String,
    pub name_tokens: String,
    pub is_entry_point: bool,
}

pub fn extract_symbols(source: &str, language: &str, path: &str) -> Result<Vec<ParsedSymbol>, String> {
    let mut parser = tree_sitter::Parser::new();
    if language == "python" {
        let lang: tree_sitter::Language = tree_sitter_python::LANGUAGE.into();
        parser.set_language(&lang).map_err(|e| e.to_string())?;
    } else {
        return Err(format!("Unsupported language: {}", language));
    }

    let tree = parser.parse(source, None).ok_or("Failed to parse")?;
    let mut cursor = tree.walk();
    let mut symbols = Vec::new();

    for child in tree.root_node().children(&mut cursor) {
        if child.kind() == "function_definition" {
            if let Some(name_node) = child.child_by_field_name("name") {
                let name = source[name_node.byte_range()].to_string();
                symbols.push(ParsedSymbol {
                    qualified_name: name.clone(),
                    name: name.clone(),
                    kind: SymbolKind::Function,
                    language: language.to_string(),
                    path: path.to_string(),
                    line_start: child.start_position().row + 1,
                    line_end: child.end_position().row + 1,
                    signature: "".to_string(),
                    docstring: "".to_string(),
                    name_tokens: name,
                    is_entry_point: false,
                });
            }
        }
    }

    Ok(symbols)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SymbolKind;

    #[test]
    fn test_python_symbol_extraction() {
        let code = "def hello(): pass";
        let symbols = extract_symbols(code, "python", "test.py").unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "hello");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
    }
}
