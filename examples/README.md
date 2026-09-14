# Examples

Everything that runs ON Dew rather than being Dew, in one tree.

There were three of these: `applets/`, `demos/` and `examples/`. Nobody could
state the difference, so every new one was a decision about where it went, and
the tooling had to be taught each directory separately. Twice a step looked in
one and not the others, and the gap had nothing to report it.

They are grouped by what they show.

## `aether/`

Applets built with Aether, showing the framework working inside the guest
environment. `framework-coverage` measures these, because an example that brings
no framework has nothing to report.

```sh
cargo run -- examples/aether/timetracker
```

## `widgets/`

What an applet looks like with no framework at all: `Instance.new`, property
assignment, `Parent`, and the vocabulary. Every line would build the same tree on
a conforming engine host, and nothing is imported.

```sh
cargo run -- examples/widgets/nameplate
```

## `host/`

The smallest thing the contract accepts, and the place to look for what `dew`
itself offers.

```sh
cargo run -- examples/host/asks
```

`host/standalone` is the odd one: a bare script rather than an applet, with no
`dew.toml` and no manifest, run with `--script`. It exercises the whole DataModel
path end to end, from the reflection database refusing a class that does not
exist, through the layout pass, to the rasteriser. If it renders, the path from
Luau to pixels is alive.

```sh
cargo run -- snapshot --script examples/host/standalone/app.luau --size 360x220 -o out.png
```

It parents into `DewRoot` rather than `game`. Aether's `Host.detect()` keys on
`typeof(game) == "Instance"`, so installing a `game` global before the services
and the member surface are behind it would flip every Aether applet in the same
binary onto its engine branch and break it.
