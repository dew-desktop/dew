# Dew

A desktop applet platform. Widgets are written in Luau and run on
[Aether](https://github.com/project-aether-ui/aether), the same UI framework they
would run on inside Roblox.

## Running it

```sh
cargo run --manifest-path host/Cargo.toml -- --mod timetracker
```

`--mod <id>` picks a widget. Without it, Dew lists what it found and runs the
first alphabetically.

`--snapshot <path>` renders one frame to a PNG and exits, needing no window. It
is how a widget gets diffed in CI, and the only way to see one from a terminal.

`--stats` reports where frame time goes; `--bench` repaints every frame, which is
the load a drag produces.

Dew sits in the tray while it runs. The menu offers a max-FPS cap, uncapped by
default, and Exit.

## Writing a widget

A widget is a directory under [`mods/`](mods/) holding a `mod.json` and a
`<id>.luau`:

```luau
return {
    id = "clock",
    size = { width = 220, height = 56 },
    surface = { kind = "widget", anchor = "top-right" },

    mount = function(dew)
        return create "Frame" { --[[ ... ]] }
    end,
}
```

`mount` runs once and returns a tree. `dew` carries exactly the capabilities
`mod.json` declared, so a widget that asks for nothing can render and touch
nothing. [docs/mod_contract.md](docs/mod_contract.md) has the rest.

Three surfaces are available: a floating transparent widget, an ordinary window,
or an overlay covering the desktop.

## Setup

Aether is pinned by commit in [`host/Cargo.toml`](host/Cargo.toml). Both the Rust
crates and the Luau source come from that one revision, so no sibling checkout is
needed.

Run `pesde install` once, for vide. Aether declares it and a pinned checkout does
not carry it, so this host supplies it.

## Contributing

[CONTRIBUTING.md](CONTRIBUTING.md), and
[docs/contributing/guidelines.md](docs/contributing/guidelines.md) for how work
is branched, written and landed.

## License

[MIT](LICENSE)
