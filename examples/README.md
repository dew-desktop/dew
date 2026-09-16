# Examples

Programs that run on Dew, each showing one thing.

## `host/`

What the host itself offers, with no framework involved.

`basic-widget` is the smallest applet that draws: it opens a widget, builds a
card out of instances, and puts text on it.

```sh
cargo run -- examples/host/basic-widget
```

`standalone` is a bare script rather than an applet. It has no `dew.toml` and no
manifest, and runs with `--script`, so it is the shortest path from Luau to
pixels: the reflection database, the vocabulary, the layout pass and the
rasteriser, with nothing else in the way.

```sh
cargo run -- snapshot --script examples/host/standalone/app.luau --size 360x220 -o out.png
```

It parents into `DewRoot` rather than `game`. Aether keys on
`typeof(game) == "Instance"` to decide which host it is running on, so a `game`
global here would send every Aether applet in the same process down its engine
branch.

## `widgets/`

Applets written against the DataModel directly: `Instance.new`, property
assignment, `Parent`, and the vocabulary. Nothing is imported, and every line
would build the same tree on any host implementing the DataModel.

```sh
cargo run -- examples/widgets/nameplate
```

## `aether/`

Applets built with [Aether](https://github.com/project-aether-ui/aether), a UI
framework that runs on any conforming host. An applet installs it with pesde like
any other package; Dew supplies nothing.

```sh
cargo run -- examples/aether/timetracker
```

`contextmenu` opens a menu from a right-click the host reports through its input
service, walks the pointer onto a row and presses it.

Each one installs its own framework: a `pesde.toml` naming Aether and vide by
commit, required through the redirect `pesde install` writes beside it. Dew
supplies nothing and its own dependencies are empty.

`framework-coverage` measures these to report how much of the framework is
actually exercised rather than merely shipped. An example opts in by exporting
`Measure`; the rest are skipped and the run says how many of each it saw.

| export | what it is |
| :--- | :--- |
| `Measure` | mounts the same tree with a given Aether, for the coverage pass |
| `Aether` | the table the tool wraps to record what was used |
| `Script` | the interaction to drive, as data |
| `Session` | what the tool steps, from `Aether.Desktop.Mount` |
| `Width`, `Height` | the surface to paint |
| a reader | what the example saw: `Presses`, `Opened`, `Chosen`, `Hovered`, `Ticks` |

A reader is what separates "the input never arrived" from "the feature ignored
it". Both paint the same picture, and `contextmenu` answers `Chosen` with the
row a press landed on, which no pixel count can report.
