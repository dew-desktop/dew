# Dew

A desktop applet platform. Widgets are written in Luau and run on
[Aether](https://github.com/project-aether-ui/aether), the same UI framework they
would run on inside Roblox.

## Running it

```sh
cargo run -p dew-host -- --applet timetracker
```

From inside `host/`, plain `cargo run -- --applet timetracker` works too: the host
walks up for `applets/` and for the installed vide, so either directory is fine.

`--applet <id>` picks a widget. Without it, Dew lists what it found and runs the
first alphabetically.

`--snapshot <path>` renders one frame to a PNG and exits, needing no window. It
is how a widget gets diffed in CI, and the only way to see one from a terminal.

`--stats` reports where frame time goes; `--bench` repaints every frame, which is
the load a drag produces.

Dew sits in the tray while it runs. The menu offers a max-FPS cap, uncapped by
default, and Exit.

## Writing a widget

A widget is a directory under [`applets/`](applets/) holding a `dew.toml` and a
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
`dew.toml` declared, so a widget that asks for nothing can render and touch
nothing. [docs/applet_contract.md](docs/applet_contract.md) has the rest.

Three surfaces are available: a floating transparent widget, an ordinary window,
or an overlay covering the desktop.

### Without a framework

A widget does not have to be an Aether component. `"runtime": "datamodel"` in
`dew.toml` gets an applet the DataModel Dew implements itself and nothing else:

```luau
return {
    id = "nameplate",
    size = { width = 340, height = 148 },

    mount = function(dew, root)
        local card = Instance.new("Frame")
        card.Size = UDim2.new(1, 0, 1, 0)
        card.BackgroundColor3 = Color3.fromRGB(20, 26, 36)
        card.Parent = root
    end,
}
```

No `require`, nothing imported, and every line of it would build the same tree
inside Roblox. [`applets/nameplate`](applets/nameplate/) is the whole example. The
runtime is declared rather than detected, and everything else -- discovery, the
manifest, capabilities, the surface -- is identical either way.

What "the same tree inside Roblox" means precisely is the DataModel Standard, and
it is written down in two halves.
[docs/datamodel_scope.md](docs/datamodel_scope.md) is what a host must ACCEPT --
classes, properties, members -- and is generated from the engine's own reflection
database. [docs/host_services.md](docs/host_services.md) is what a host must be
able to DO: measure a string synchronously, and hand out a frame subscription.
Neither of those two is a member of any class, which is why they have a document
of their own rather than a hand-written appendix to a generated one.

## Setup

```sh
pesde install
```

That is the whole of it, and it installs two GUEST packages: Aether and the vide
it declares. Both are pinned by commit in [`pesde.toml`](pesde.toml), and no
sibling checkout is needed.

Aether used to arrive through Cargo instead, because the rendering stack lived in
its repository and the Luau travelled in the same checkout. It does not any more:
`crates/raster`, `crates/runtime` and `crates/window` are Dew's, and Aether is a
framework Dew's guests may choose, exactly like any other Luau package. ADR-004
is the reasoning; `scripts/verify_boundaries.luau` is what keeps it true.

## Contributing

[CONTRIBUTING.md](CONTRIBUTING.md), and
[docs/contributing/guidelines.md](docs/contributing/guidelines.md) for how work
is branched, written and landed.

## License

[GPL-3.0](LICENSE).

Widgets are not covered by it. A widget is an interpreted Luau file loaded at
runtime into its own sandboxed VM, given a table of host functions and nothing
else; it never links against Dew and never touches Dew's own code. Roblox does
the same thing at far greater scale, shipping CorePackages into every game's
runtime beside the game's own scripts.

Widgets build on [Aether](https://github.com/project-aether-ui/aether), which is
MIT, and stay entirely their authors' own.
