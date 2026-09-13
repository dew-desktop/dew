# Dew

A desktop applet platform. Applets are small Luau programs, a clock or a tracker
or a dashboard, that run in a sandboxed host and are drawn natively.

Each applet declares what it may touch and gets nothing else. It brings its own UI
library rather than being handed one, installs its own dependencies, and is
rendered by Dew's rasteriser against an object model the host implements to a
written standard.

## Running it

```sh
cargo run -p dew-host -- --applet timetracker
```

| | |
| :--- | :--- |
| `--applet <id>` | run one applet. Without it, Dew lists what it found and runs the first |
| `--snapshot <path>` | render one frame to a PNG and exit, needing no window |
| `--stats` | report where frame time goes |
| `--bench` | repaint every frame, which is the load a drag produces |

Dew sits in the tray while it runs.

## Writing an applet

An applet is a directory holding a `dew.toml` and a `<id>.luau`:

```luau
return {
    id = "clock",
    size = { width = 220, height = 56 },
    surface = { kind = "widget", anchor = "top-right" },

    mount = function(dew, root)
        -- build a tree under `root`
    end,
}
```

`mount` runs once. `dew` carries exactly the capabilities `dew.toml` declared, so
an applet that asks for nothing can render and touch nothing.

[docs/applet_contract.md](docs/applet_contract.md) is the contract in full.
[docs/datamodel_scope.md](docs/datamodel_scope.md) and
[docs/host_services.md](docs/host_services.md) are the standard the host
implements.

## Setup

```sh
rokit install     # lune, for the scripts and gates
pesde install     # the packages Dew's own tests run against
cargo test --workspace
```

An applet installs its own packages from its own directory.

## Contributing

[CONTRIBUTING.md](CONTRIBUTING.md), and
[docs/contributing/guidelines.md](docs/contributing/guidelines.md) for how work is
branched, written and landed. Contributions require agreeing to the
[CLA](CLA.md).

## License

**[PolyForm Shield 1.0.0](LICENSE)** for the client, meaning everything under
`host/` and `crates/`. Read it, build it, change it, use it for anything except
providing a product that competes with Dew or with the services Dew provides.

**[MIT](docs/LICENSE)** for the specification documents in `docs/`, so that other
implementations can adopt the standard.

Applets are not covered by either. An applet is an interpreted Luau file loaded
into its own sandboxed VM and given a table of host functions. It never links
against Dew, so its licence is entirely its author's own.

## Notices

Dew is an independent project, not affiliated with or endorsed by Roblox
Corporation. See [NOTICE](NOTICE).
