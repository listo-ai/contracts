//! Generates TypeScript Zod schemas from Rust contract types.
//!
//! Run via `cargo run -p listo-codegen` from the contracts workspace, or
//! `mani run codegen --projects contracts` from the workspace root. The
//! generated files are committed to downstream repos (e.g. agent-client-ts)
//! so consumers don't need Rust toolchain at install time.
//!
//! Adding a new generated enum/type:
//!   1. Derive `schemars::JsonSchema` on the Rust type in `listo-spi`.
//!   2. Register a generator below in `main`.
//!   3. Re-run codegen, commit the output.

use std::{fs, path::PathBuf};

use schemars::schema_for;

fn main() {
    let out_root = resolve_agent_client_ts_root();
    let generated_dir = out_root.join("src").join("generated");
    fs::create_dir_all(&generated_dir).expect("create generated dir");

    write_facets(&generated_dir);

    eprintln!("codegen: wrote {}", generated_dir.display());
}

/// Resolves the on-disk path to the sibling `agent-client-ts` repo.
/// Override via `AGENT_CLIENT_TS_DIR` for non-standard layouts.
fn resolve_agent_client_ts_root() -> PathBuf {
    if let Ok(p) = std::env::var("AGENT_CLIENT_TS_DIR") {
        return PathBuf::from(p);
    }
    // Default layout: workspace/contracts/codegen → workspace/agent-client-ts
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent() // contracts/
        .and_then(|p| p.parent()) // workspace/
        .map(|p| p.join("agent-client-ts"))
        .expect("resolve agent-client-ts path")
}

fn write_facets(dir: &std::path::Path) {
    let schema = schema_for!(spi::Facet);
    let json = serde_json::to_value(&schema).expect("schema to json");
    let values = extract_string_enum_json(&json).unwrap_or_else(|| {
        panic!(
            "Facet schema must be a string enum — check JsonSchema derive. Schema was: {json:#}"
        )
    });

    let mut ts = String::new();
    ts.push_str(&banner());
    ts.push_str("import { z } from \"zod\";\n\n");
    ts.push_str("export const FacetSchema = z.enum([\n");
    for v in &values {
        ts.push_str(&format!("  \"{v}\",\n"));
    }
    ts.push_str("]);\n\n");
    ts.push_str("export type Facet = z.infer<typeof FacetSchema>;\n\n");
    ts.push_str("export const FACET_VALUES = FacetSchema.options;\n");

    let path = dir.join("facets.ts");
    fs::write(&path, ts).expect("write facets.ts");
}

/// Walks a schema JSON value and pulls out a string `enum: [...]` list.
/// Handles both flat enums and `oneOf` variants with `const` values — covers
/// schemars' output for unit enums regardless of version.
fn extract_string_enum_json(v: &serde_json::Value) -> Option<Vec<String>> {
    if let Some(arr) = v.get("enum").and_then(|x| x.as_array()) {
        let out: Option<Vec<String>> = arr
            .iter()
            .map(|x| x.as_str().map(|s| s.to_owned()))
            .collect();
        if out.is_some() {
            return out;
        }
    }
    if let Some(arr) = v.get("oneOf").and_then(|x| x.as_array()) {
        let mut out = Vec::new();
        for variant in arr {
            if let Some(c) = variant.get("const").and_then(|x| x.as_str()) {
                out.push(c.to_owned());
                continue;
            }
            if let Some(inner) = variant.get("enum").and_then(|x| x.as_array()) {
                for s in inner {
                    let s = s.as_str()?;
                    out.push(s.to_owned());
                }
                continue;
            }
            return None;
        }
        if !out.is_empty() {
            return Some(out);
        }
    }
    None
}

fn banner() -> String {
    format!(
        "// @generated — DO NOT EDIT BY HAND\n\
         // Regenerate via: mani run codegen --projects contracts\n\
         // Source: listo-ai/contracts ({})\n\n",
        env!("CARGO_PKG_VERSION")
    )
}

