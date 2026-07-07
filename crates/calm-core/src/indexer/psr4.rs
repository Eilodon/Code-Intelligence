//! PSR-4 autoload resolution: reads `composer.json`'s `autoload.psr-4` (and
//! `autoload-dev.psr-4`) mapping — namespace prefix -> source directory —
//! once per indexing run (mirrors `crate_map::CrateMap::build`'s "read real
//! files, build an in-memory map, thread it through the pipeline" pattern),
//! so `use App\Service\Foo;` can resolve to a real file without a
//! directory-matches-namespace convention scan — PHP namespaces don't
//! reliably mirror directory structure the way Go packages do (see the
//! 8-language plan's P1.2).

use std::path::Path;

pub struct Psr4Map {
    /// (namespace prefix incl. trailing `\`, source directory relative to
    /// project root, without a trailing `/`) pairs, sorted longest-prefix-
    /// first so a more specific mapping (`App\Tests\`) is tried before a
    /// broader one (`App\`) sharing the same leading segment.
    prefixes: Vec<(String, String)>,
}

impl Psr4Map {
    pub fn build(project_root: &Path) -> Self {
        let mut prefixes = Self::from_composer_json(project_root);
        prefixes.sort_by_key(|(p, _)| std::cmp::Reverse(p.len()));
        Psr4Map { prefixes }
    }

    fn from_composer_json(project_root: &Path) -> Vec<(String, String)> {
        let Ok(text) = std::fs::read_to_string(project_root.join("composer.json")) else {
            return Vec::new();
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for key in ["autoload", "autoload-dev"] {
            let Some(map) = json
                .get(key)
                .and_then(|a| a.get("psr-4"))
                .and_then(|p| p.as_object())
            else {
                continue;
            };
            for (prefix, dirs) in map {
                let dirs: Vec<String> = match dirs {
                    serde_json::Value::String(s) => vec![s.clone()],
                    serde_json::Value::Array(a) => a
                        .iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect(),
                    _ => continue,
                };
                for dir in dirs {
                    out.push((prefix.clone(), dir.trim_end_matches('/').to_string()));
                }
            }
        }
        out
    }

    pub fn is_empty(&self) -> bool {
        self.prefixes.is_empty()
    }

    /// Resolve a backslash-separated namespace path (e.g. `App\Service\Foo`,
    /// as written in a `use` statement) to a candidate `.php` file path
    /// relative to the project root — existence against the real index is
    /// checked by the caller (`resolve_module_to_path`), same as every
    /// other language's candidate there.
    pub fn resolve(&self, namespace_path: &str) -> Option<String> {
        // A trailing `\` so a prefix can never match a partial segment
        // (e.g. prefix "App\" must not match namespace "AppFoo\Bar").
        let full = format!("{namespace_path}\\");
        for (prefix, dir) in &self.prefixes {
            if let Some(rest) = full.strip_prefix(prefix.as_str()) {
                let rest = rest.trim_end_matches('\\').replace('\\', "/");
                if rest.is_empty() {
                    continue;
                }
                return Some(if dir.is_empty() {
                    format!("{rest}.php")
                } else {
                    format!("{dir}/{rest}.php")
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_composer(dir: &Path, body: &str) {
        std::fs::write(dir.join("composer.json"), body).unwrap();
    }

    #[test]
    fn resolves_simple_prefix() {
        let dir = std::env::temp_dir().join(format!("ci_psr4_simple_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_composer(&dir, r#"{"autoload": {"psr-4": {"App\\": "src/"}}}"#);
        let map = Psr4Map::build(&dir);
        assert_eq!(
            map.resolve("App\\Service\\Foo"),
            Some("src/Service/Foo.php".to_string())
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_composer_json_yields_empty_map() {
        let dir = std::env::temp_dir().join(format!("ci_psr4_none_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let map = Psr4Map::build(&dir);
        assert!(map.is_empty());
        assert_eq!(map.resolve("App\\Foo"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn longest_prefix_wins() {
        let dir = std::env::temp_dir().join(format!("ci_psr4_longest_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_composer(
            &dir,
            r#"{"autoload": {"psr-4": {"App\\": "src/", "App\\Tests\\": "tests/"}}}"#,
        );
        let map = Psr4Map::build(&dir);
        assert_eq!(
            map.resolve("App\\Tests\\FooTest"),
            Some("tests/FooTest.php".to_string()),
            "the more specific App\\Tests\\ prefix must win over the broader App\\"
        );
        assert_eq!(
            map.resolve("App\\Service\\Foo"),
            Some("src/Service/Foo.php".to_string())
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unrelated_namespace_yields_none() {
        let dir = std::env::temp_dir().join(format!("ci_psr4_unrelated_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_composer(&dir, r#"{"autoload": {"psr-4": {"App\\": "src/"}}}"#);
        let map = Psr4Map::build(&dir);
        assert_eq!(map.resolve("Vendor\\Package\\Thing"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
