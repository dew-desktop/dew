# Contributing

## Running it

```sh
cargo run -- examples/host/basic-widget
```

Every example brings the framework it requires, so install one before running
it:

```sh
cd examples/aether/timetracker && pesde install
```

Dew itself declares no framework. `cargo test --manifest-path host/Cargo.toml`
borrows one from an example that has installed it.

## Checking it

```sh
lune run scripts/smoke.luau            every command the CLI promises
lune run scripts/verify_boundaries.luau  Dew owns its host outright
cargo test --manifest-path host/Cargo.toml
```

CI runs those, plus the build and the applet mounts on both Windows and Linux.

## Writing an applet

[docs/applet_contract.md](docs/applet_contract.md) is what a `dew.toml` and an
entry module have to say. [examples/](examples/) is the same thing working.

## Pull requests

**One observable goal per branch**, and the prefix says which kind of work it
is. A branch that needs "and" to describe it is two branches.

**`main` is always green.** Every commit on it compiles, passes its checks, and
runs.
