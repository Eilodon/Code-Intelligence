use std::collections::HashMap;

use stack_graphs::graph::StackGraph;
use stack_graphs::partial::{PartialPath, PartialPaths};
use stack_graphs::stitching::{
    Database, DatabaseCandidates, ForwardPartialPathStitcher, StitcherConfig,
};
use tree_sitter_stack_graphs::CancellationFlag as TsgCancellationFlag;
use tree_sitter_stack_graphs::NoCancellation as TsgNoCancellation;
use tree_sitter_stack_graphs::StackGraphLanguage;
use tree_sitter_stack_graphs::Variables;

fn cancellation_flag() -> &'static dyn TsgCancellationFlag {
    &TsgNoCancellation
}

pub struct FormalResolver {
    configs: HashMap<String, FormalLanguageConfig>,
}

struct FormalLanguageConfig {
    sgl: StackGraphLanguage,
    #[allow(dead_code)]
    builtins: StackGraph,
    #[allow(dead_code)]
    no_similar_paths_in_file: bool,
}

#[derive(Debug, Clone)]
pub struct FormalEdge {
    pub reference_symbol: String,
    pub definition_symbol: String,
}

impl FormalResolver {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }

    pub fn load_python(&mut self) -> anyhow::Result<()> {
        let lc = tree_sitter_stack_graphs_python::try_language_configuration(cancellation_flag())
            .map_err(|e| anyhow::anyhow!("Failed to load Python stack-graphs config: {e}"))?;
        self.configs.insert(
            "python".to_string(),
            FormalLanguageConfig {
                sgl: lc.sgl,
                builtins: lc.builtins,
                no_similar_paths_in_file: lc.no_similar_paths_in_file,
            },
        );
        Ok(())
    }

    pub fn resolve_file(
        &self,
        language: &str,
        file_path: &str,
        source: &str,
    ) -> anyhow::Result<Vec<FormalEdge>> {
        let config = self
            .configs
            .get(language)
            .ok_or_else(|| anyhow::anyhow!("No formal resolver for language: {language}"))?;

        let mut graph = StackGraph::new();

        let file = graph.get_or_create_file(file_path);

        let mut globals = Variables::new();
        globals
            .add("FILE_PATH".into(), file_path.into())
            .map_err(|_| anyhow::anyhow!("Failed to set FILE_PATH global"))?;

        config
            .sgl
            .build_stack_graph_into(&mut graph, file, source, &globals, cancellation_flag())
            .map_err(|e| anyhow::anyhow!("Stack graph build error: {e:?}"))?;

        let mut partials = PartialPaths::new();
        let mut db = Database::new();
        let stitch_config = StitcherConfig::default();

        // Index: find partial paths in this file
        ForwardPartialPathStitcher::find_minimal_partial_path_set_in_file(
            &graph,
            &mut partials,
            file,
            stitch_config,
            &stack_graphs::NoCancellation,
            |g, ps, p| {
                db.add_partial_path(g, ps, p.clone());
            },
        )
        .map_err(|e| anyhow::anyhow!("Partial path extraction cancelled: {e}"))?;

        // Resolve: find complete paths from references to definitions
        let reference_nodes: Vec<_> = graph
            .nodes_for_file(file)
            .filter(|&n| graph[n].is_reference())
            .collect();

        if reference_nodes.is_empty() {
            return Ok(Vec::new());
        }

        let mut edges = Vec::new();
        ForwardPartialPathStitcher::find_all_complete_partial_paths(
            &mut DatabaseCandidates::new(&graph, &mut partials, &mut db),
            reference_nodes.into_iter(),
            stitch_config,
            &stack_graphs::NoCancellation,
            |g, _ps, path: &PartialPath| {
                let start = path.start_node;
                let end = path.end_node;

                if g[start].is_reference() && g[end].is_definition() {
                    let ref_sym = g[start].symbol().map(|s| g[s].to_string());
                    let def_sym = g[end].symbol().map(|s| g[s].to_string());

                    if let (Some(r), Some(d)) = (ref_sym, def_sym) {
                        edges.push(FormalEdge {
                            reference_symbol: r,
                            definition_symbol: d,
                        });
                    }
                }
            },
        )
        .map_err(|e| anyhow::anyhow!("Path stitching cancelled: {e}"))?;

        Ok(edges)
    }

    pub fn has_language(&self, language: &str) -> bool {
        self.configs.contains_key(language)
    }

    pub fn supported_languages(&self) -> Vec<&str> {
        self.configs.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for FormalResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formal_resolver_new() {
        let resolver = FormalResolver::new();
        assert!(!resolver.has_language("python"));
        assert!(resolver.supported_languages().is_empty());
    }

    #[test]
    fn test_load_python() {
        let mut resolver = FormalResolver::new();
        resolver.load_python().unwrap();
        assert!(resolver.has_language("python"));
    }

    #[test]
    fn test_resolve_no_language() {
        let resolver = FormalResolver::new();
        let result = resolver.resolve_file("python", "test.py", "x = 1");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_simple_python_def_ref() {
        let mut resolver = FormalResolver::new();
        resolver.load_python().unwrap();
        let source = r#"
def foo():
    pass

def bar():
    foo()
"#;
        let edges = resolver.resolve_file("python", "test.py", source).unwrap();
        let has_foo_edge = edges
            .iter()
            .any(|e| e.definition_symbol == "foo" || e.reference_symbol == "foo");
        assert!(
            has_foo_edge,
            "Should resolve foo() call to foo definition. Edges: {edges:?}"
        );
    }

    #[test]
    fn test_resolve_python_no_refs() {
        let mut resolver = FormalResolver::new();
        resolver.load_python().unwrap();
        let source = "x = 1\n";
        let edges = resolver.resolve_file("python", "test.py", source).unwrap();
        // Simple assignment may or may not produce edges depending on rules
        // Just verify it doesn't crash
        let _ = edges;
    }

    #[test]
    fn test_resolve_python_class() {
        let mut resolver = FormalResolver::new();
        resolver.load_python().unwrap();
        let source = r#"
class MyClass:
    def method(self):
        pass

def use_class():
    obj = MyClass()
"#;
        let edges = resolver.resolve_file("python", "test.py", source).unwrap();
        let has_class_edge = edges
            .iter()
            .any(|e| e.definition_symbol == "MyClass" || e.reference_symbol == "MyClass");
        assert!(
            has_class_edge,
            "Should resolve MyClass() to class definition. Edges: {edges:?}"
        );
    }
}
