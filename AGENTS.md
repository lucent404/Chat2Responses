# Repository Guidelines

## Project Structure & Module Organization

This repository contains `Chat2Responses`, a Rust service proxy that translates Codex CLI Responses API traffic into Chat Completions API requests. Core Rust code lives in `src/`: `main.rs` defines the Axum service and admin API, `db.rs` owns SQLite persistence, `security.rs` handles key hashing/encryption, `translate.rs` handles protocol conversion, `stream.rs` handles SSE streaming, `session.rs` stores response/session state, and `types.rs` contains wire types. The React/Vite admin UI lives in `admin-ui/`. Tests live in `tests/`, with versioned captured payloads under `tests/fixtures/codex_<version>/`; update `tests/fixtures/VERSIONS.md` when adding fixture sets. `chat2responses/` is the Python package shim used by maturin packaging.

## Build, Test, and Development Commands

- `cargo build`: compile the Rust binary and library.
- `CHAT2RESPONSES_SECRET=dev-secret cargo run -- --port 4444`: run the local service.
- `cd admin-ui && npm install && npm run build`: build the admin console served from `/admin/`.
- `cargo test`: run offline compatibility and translation tests.
- `DEEPSEEK_API_KEY=sk-... cargo test --test compat_deepseek_live -- --ignored --test-threads=1`: run gated live provider tests.
- `maturin build --release`: build Python wheels using `pyproject.toml`.

## Coding Style & Naming Conventions

Use Rust 2021 conventions and `rustfmt` defaults: four-space indentation, `snake_case` functions/modules, `PascalCase` types, and `SCREAMING_SNAKE_CASE` constants. Keep protocol structs in `types.rs` and translation behavior in `translate.rs`; avoid mixing server routing concerns into conversion code. Prefer typed `serde` structures over ad hoc JSON manipulation unless the upstream shape is intentionally dynamic.

## Testing Guidelines

Add offline regression tests for every protocol-shape change. Name tests by observable behavior, for example `namespace_tools_are_flattened`. Keep fixtures minimal and versioned by Codex CLI release. Live tests are `#[ignore]` and must remain key-gated so `cargo test` is deterministic without network access.

## Commit & Pull Request Guidelines

This exported checkout has no `.git` history, so no local commit convention can be verified. Use concise, imperative commit subjects such as `Handle Codex 0.128 reasoning items`. Pull requests should describe the protocol behavior changed, list test commands run, mention affected providers, and include fixture provenance when captured payloads change.

## Security & Configuration Tips

Never commit provider API keys or the SQLite database. Provider credentials are entered through the admin UI and encrypted with `CHAT2RESPONSES_SECRET`; service-issued caller keys are stored only as hashes. Prefer localhost ports because the relay is intended to bind `127.0.0.1`.
