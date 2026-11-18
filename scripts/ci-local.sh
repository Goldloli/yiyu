#!/usr/bin/env bash
set -euo pipefail
actionlint
pnpm typecheck
pnpm test
pnpm qa:interactions
pnpm qa:screenshots
pnpm qa:screenshots:compare
pnpm build:web
node scripts/measure-first-load.mjs web/dist
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
node scripts/version.mjs
openspec validate --all --strict
