# contracts

Wire types and schemas — the root of the Listo dependency graph. Everything
depends on this; this depends on nothing internal.

## Crates

| Crate | Published as | Purpose |
|-------|--------------|---------|
| `spi` | `listo-spi` | Wire types: `Msg`, `KindManifest`, `NodeId`, `Facet`, proto + JSON schemas |
| `ui-ir` | `listo-ui-ir` | SDUI component tree types (`ComponentTree`, `Component`, `Action`) |
| `codegen` | — (internal tool) | Generates TypeScript Zod schemas from Rust types |

## Build

```bash
cargo build --workspace
cargo test --workspace
```

## Codegen

TypeScript schemas in `../agent-client-ts/src/generated/` are generated from
Rust types annotated with `#[derive(schemars::JsonSchema)]`. Regenerate after
adding a variant to any mirrored enum:

```bash
cargo run -p listo-codegen
# or from workspace root:
mani run codegen
```

## Rule

`spi` and `ui-ir` have zero deps on any other internal crate. Only third-party
(`serde`, `schemars`, `semver`). This makes them safe to publish and depend on
from every direction.

Part of the [listo-ai workspace](../).
