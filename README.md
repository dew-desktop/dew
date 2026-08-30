# 💧 Dew (Development Branch)

Minimalist desktop HUD and Luau tooling platform.

---

## Overview

Dew is an open-source, keyboard-driven desktop HUD powered by an embedded Luau runtime engine. It enables fast, lightweight workflow utilities authored as typed Luau modules.

---

## Running it

```sh
cargo run --manifest-path host/Cargo.toml -- --mod timetracker
```

`--mod <id>` picks a widget; without it, Dew lists what it found and runs the
first alphabetically. `--snapshot <path>` renders one frame to a PNG and exits,
needing no window — which is how a widget gets diffed in CI, and the only way to
see one from a terminal.

Dew sits in the tray while it runs. The menu offers a max-FPS cap — **uncapped by
default**, with 30 / 60 / 120 / 144 / 240 — and Exit. Uncapped means the loop does
not sleep, so an idle widget will spin a core; pick a cap if that matters more
than latency.

Mods live in [`mods/`](mods/); each is a directory with a `mod.json` and a
`<id>.luau`. See [docs/mod_contract.md](docs/mod_contract.md) for what a mod
returns and how its permissions are granted.

Aether is pinned by commit in [`host/Cargo.toml`](host/Cargo.toml), from
[project-aether-ui/aether](https://github.com/project-aether-ui/aether). Both the
Rust crates and the Luau source come from that one revision, so no sibling
checkout is needed and nothing can drift between the two.

`pesde install` once, for vide — Aether declares it and a pinned checkout does
not carry it, so this host supplies it.

## Repository Structure (Dev)

- [`types/`](types/): Luau type definitions (`@dew/core.d.luau`) for mod SDK interfaces.
- [`mods/`](mods/): Reference Luau mod implementations.
- [`scripts/`](scripts/): Repository automation, versioning, and commit validation scripts written in Luau.
- [`VERSION`](VERSION): Current development version.

---

## License

[MIT](LICENSE)
