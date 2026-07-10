//! Phase A.1 (2026-07-10 25-language-expansion plan): catches the exact bug
//! class that shipped in production — calm-cli missing a `lang-csharp`
//! passthrough feature that calm-core and calm-server both declared. Cargo's
//! per-crate feature re-declaration (there is no cross-crate feature
//! registry) means this can silently drift again for any `lang-*` flag, and
//! will multiply as Phase C adds 9 more languages across all three files.
//!
//! This is a source-parity check, not a build check: `cargo check` alone
//! can't catch a *missing* feature declaration (it just means that feature
//! doesn't exist, not an error) — only a symmetric-set comparison across the
//! three Cargo.toml files does.

use std::collections::BTreeSet;
use std::path::Path;

fn lang_features_in(manifest_relpath: &str) -> BTreeSet<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    let text = std::fs::read_to_string(root.join(manifest_relpath))
        .unwrap_or_else(|e| panic!("failed to read {manifest_relpath}: {e}"));
    let doc: toml::Value = text
        .parse()
        .unwrap_or_else(|e| panic!("failed to parse {manifest_relpath}: {e}"));
    let features = doc
        .get("features")
        .and_then(|f| f.as_table())
        .unwrap_or_else(|| panic!("{manifest_relpath} has no [features] table"));
    features
        .keys()
        .filter(|k| k.starts_with("lang-"))
        .cloned()
        .collect()
}

/// Every `lang-*` feature declared in calm-core (the crate that owns the
/// actual `dep:tree-sitter-*` gate) must be re-declared as a passthrough in
/// both calm-server and calm-cli — otherwise a user building calm-cli with
/// `--features lang-X` gets a silent no-op (feature doesn't exist, Cargo
/// doesn't error) instead of the grammar they asked for.
#[test]
fn lang_feature_flags_match_across_core_server_cli() {
    let core = lang_features_in("crates/calm-core/Cargo.toml");
    let server = lang_features_in("crates/calm-server/Cargo.toml");
    let cli = lang_features_in("crates/calm-cli/Cargo.toml");

    assert!(
        !core.is_empty(),
        "sanity check failed: found zero lang-* features in calm-core/Cargo.toml — \
         did the [features] table move or get renamed?"
    );

    let missing_in_server: Vec<_> = core.difference(&server).collect();
    let missing_in_cli: Vec<_> = core.difference(&cli).collect();
    let extra_in_server: Vec<_> = server.difference(&core).collect();
    let extra_in_cli: Vec<_> = cli.difference(&core).collect();

    assert!(
        missing_in_server.is_empty(),
        "calm-server/Cargo.toml is missing passthrough for: {missing_in_server:?} \
         (declared in calm-core but not re-exported by calm-server)"
    );
    assert!(
        missing_in_cli.is_empty(),
        "calm-cli/Cargo.toml is missing passthrough for: {missing_in_cli:?} \
         (this is the exact bug class that shipped for lang-csharp — a user \
         building calm-cli with this feature gets silent no-op, not an error)"
    );
    assert!(
        extra_in_server.is_empty(),
        "calm-server/Cargo.toml declares lang-* features calm-core doesn't have: \
         {extra_in_server:?} (stale passthrough — calm-core dropped the dep gate?)"
    );
    assert!(
        extra_in_cli.is_empty(),
        "calm-cli/Cargo.toml declares lang-* features calm-core doesn't have: \
         {extra_in_cli:?} (stale passthrough — calm-core dropped the dep gate?)"
    );
}

/// Beyond key parity, each passthrough's *value* must actually forward to
/// the corresponding calm-core feature — a typo'd or empty passthrough
/// (`lang-r = []`) would pass the key-set check above but still silently
/// no-op.
#[test]
fn lang_feature_passthroughs_forward_to_calm_core() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    for (manifest_relpath, expect_transitive_server) in [
        ("crates/calm-server/Cargo.toml", false),
        ("crates/calm-cli/Cargo.toml", true),
    ] {
        let text = std::fs::read_to_string(root.join(manifest_relpath)).unwrap();
        let doc: toml::Value = text.parse().unwrap();
        let features = doc["features"].as_table().unwrap();
        for (name, value) in features.iter().filter(|(k, _)| k.starts_with("lang-")) {
            let deps = value
                .as_array()
                .unwrap_or_else(|| panic!("{manifest_relpath}::{name} is not an array"));
            let deps: Vec<&str> = deps.iter().filter_map(|v| v.as_str()).collect();
            let core_ref = format!("calm-core/{name}");
            assert!(
                deps.contains(&core_ref.as_str()),
                "{manifest_relpath}::{name} = {deps:?} does not forward to {core_ref}"
            );
            if expect_transitive_server {
                let server_ref = format!("calm-server/{name}");
                assert!(
                    deps.contains(&server_ref.as_str()),
                    "{manifest_relpath}::{name} = {deps:?} does not forward to {server_ref}"
                );
            }
        }
    }
}
