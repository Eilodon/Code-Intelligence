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

        // Merge builtins (e.g. Python's `len`, `print`, `range`) into the working
        // graph so references to them can resolve — without this, `graph` only
        // ever contains the single file being analyzed and every builtin
        // reference is unresolvable by construction. `add_from_graph` copies the
        // builtins' files/nodes/edges in and returns their new file handles,
        // which also need indexing into `db` below so they're stitchable.
        let builtin_files = graph
            .add_from_graph(&config.builtins)
            .map_err(|h| anyhow::anyhow!("Duplicate builtin file: {}", &graph[h]))?;

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

        // Also index partial paths for the merged builtins files, so a
        // reference in `file` can stitch all the way to a builtin definition.
        for builtin_file in &builtin_files {
            ForwardPartialPathStitcher::find_minimal_partial_path_set_in_file(
                &graph,
                &mut partials,
                *builtin_file,
                stitch_config,
                &stack_graphs::NoCancellation,
                |g, ps, p| {
                    db.add_partial_path(g, ps, p.clone());
                },
            )
            .map_err(|e| anyhow::anyhow!("Builtins partial path extraction cancelled: {e}"))?;
        }

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
            reference_nodes,
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

    /// Regression for B6: `builtins` was loaded into `FormalLanguageConfig`
    /// but never merged into the per-file working graph (`StackGraph::new()`
    /// started empty every time), so nothing in `builtins` could ever be
    /// referenced, and a malformed/duplicate builtins graph would have gone
    /// unnoticed since it was never touched.
    ///
    /// NOTE on scope: this only verifies the merge happens correctly (file
    /// count grows, no `add_from_graph` error). It deliberately does NOT
    /// assert that a Python builtin like `len()` resolves end-to-end —
    /// investigating that surfaced two independent upstream gaps in the
    /// pinned `tree-sitter-stack-graphs-python` 0.3.0: (1) its bundled
    /// `src/builtins.py` is empty (`include_str!` yields 0 bytes), and (2)
    /// even with a synthetic non-empty builtins file built through the same
    /// real `sgl` rules, `stack-graphs.tsg`'s `global -> ROOT_NODE` edge
    /// pushes the symbol `"<builtins>"` but no node anywhere in that grammar
    /// pops `"<builtins>"` — the binding is a dead end at the grammar level,
    /// not something this merge can route around. Fixing that needs a newer/
    /// different `tree-sitter-stack-graphs-python` version or a project-
    /// authored builtins.py + matching tsg rule, both out of scope here.
    #[test]
    fn test_resolve_file_merges_builtins_without_error() {
        let mut resolver = FormalResolver::new();
        resolver.load_python().unwrap();
        let config = resolver.configs.get("python").unwrap();
        let builtins_file_count = config.builtins.iter_files().count();

        // Sanity: the merge path only matters once builtins actually has
        // file(s) to merge — if this ever goes back to 0, the test below
        // would pass vacuously.
        assert!(
            builtins_file_count > 0,
            "builtins graph should contain at least the <builtins> file, even though its source is empty"
        );

        let mut graph = StackGraph::new();
        let merged = graph
            .add_from_graph(&config.builtins)
            .expect("merging builtins into a fresh graph should never collide");
        assert_eq!(merged.len(), builtins_file_count);

        // resolve_file() must still work normally with the merge in place.
        let edges = resolver
            .resolve_file(
                "python",
                "test.py",
                "def foo():\n    pass\n\ndef bar():\n    foo()\n",
            )
            .unwrap();
        assert!(
            edges.iter().any(|e| e.definition_symbol == "foo"),
            "same-file resolution must still work after merging builtins. Edges: {edges:?}"
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
