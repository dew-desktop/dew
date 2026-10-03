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

### The same file on Roblox

`flex-list-demo`, `flex-item-demo` and `grid-layout-demo` also run in Roblox
Studio, from the same file. The only line that depends on the host is where the
tree goes, and each file says it at the top:

```luau
local root: Instance
if desktop then
	root = desktop.Widget({ width = WIDTH, height = HEIGHT })
else
	-- a Frame of the same size, centred in a ScreenGui in the player's PlayerGui
end
```

Dew installs `desktop`; a Roblox place does not. Nothing defines `game`, so
Aether's own host check is unaffected. The file ends in `return card` because a
Roblox ModuleScript must return exactly one value.

Beside each file is a `default.project.json` and a `roblox.client.luau`. The
project puts the demo in `ReplicatedStorage` as a ModuleScript and the entry in
`StarterPlayerScripts` as a LocalScript, and the entry requires the demo. With
[Rojo](https://rojo.space) 7.6 (pinned in `rokit.toml`), either serve it to a
place open in Studio and press Play:

```sh
rojo serve examples/widgets/flex-list-demo
```

or build a place file and open that:

```sh
rojo build examples/widgets/flex-list-demo -o flex-list-demo.rbxl
```

CI builds every example that has a `default.project.json`. It cannot press
Play, so whether the tree looks right in Studio is checked by a person.

`flex-list-demo` lays out a tray of tag chips and its own toolbar with
`UIListLayout` and positions nothing by hand. The toolbar's buttons change the
tray's `Wraps`, `HorizontalFlex`, `VerticalFlex`, alignment and
`ItemLineAlignment` while it runs, and its test clicks them and checks where the
chips land.

```sh
cargo run -- examples/widgets/flex-list-demo
cargo run -- test examples/widgets/flex-list-demo
rojo serve examples/widgets/flex-list-demo
```

`flex-item-demo` is a chat pane where each child decides its own share of a
row: a `UIFlexItem` makes the message field take the spare width and lets the
channel chip give width back, and a `UISizeConstraint` caps the sidebar and the
message bubbles and stops the chip at a minimum. Its buttons narrow and widen
the pane and change the field's `FlexMode` and Send's `GrowRatio`, and its test
clicks them and checks each width against the engine's.

```sh
cargo run -- examples/widgets/flex-item-demo
cargo run -- test examples/widgets/flex-item-demo
rojo serve examples/widgets/flex-item-demo
```

`grid-layout-demo` is an app launcher whose tiles are placed by one
`UIGridLayout`; no tile has a position or a size of its own. Its buttons change
the grid's `CellSize`, `FillDirectionMaxCells`, `StartCorner` and
`FillDirection` while it runs, a click selects a tile, and a
`UIAspectRatioConstraint` keeps each icon square in a wide cell. Its test clicks
the buttons and the tiles and checks where each tile lands.

```sh
cargo run -- examples/widgets/grid-layout-demo
cargo run -- test examples/widgets/grid-layout-demo
rojo serve examples/widgets/grid-layout-demo
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

## Where each example runs

The table below is generated by `dew compat` and checked in CI, so it says what
the code does rather than what someone remembered to write. Each example is
loaded the way `dew snapshot` loads it, one frame is painted, and two things are
read.

**Dew-only API it reaches** comes from the example's own Luau, with comments,
strings, tests and installed packages left out. It lists every global the host
installs that the engine has no counterpart for, and every manifest permission
with no engine equivalent. Some entries do not decide the verdict: the surface
an example parents its tree into, marked "mount", which a mount helper can
replace, and Aether, which asks which host it is on and runs on both. Asking
whether a global is there at all, as in `if desktop then`, is not counted as
reaching it; what the branch then reaches is.

**Set, and not read by Dew's renderer** walks the tree the example built. For
every instance, and every property whose value differs from the reflection
database's default, it asks the renderer whether it reads that property on that
class. The column lists what it does not read, and what it reads only in part.
A property missing from this column is one the renderer reads and that changes
pixels. That does not mean Dew draws it the way the engine does: text is the
known case, measured in Segoe UI whatever font it asks for and placed
differently.

The walk sees the tree as it stands after loading. A property an example sets
only in response to a click, a hover or a timer is not in it.

The question is asked per class and property, not per value, so a property set
to a value Dew happens to match is still listed: `BorderSizePixel = 0` asks for
no border and Dew draws none, and a `CornerRadius` with no scale loses nothing
to "the scale is dropped".

A verdict is `portable` when an example reaches nothing Dew-only past its mount
and sets nothing the renderer skips or reads in part. It is `Roblox-ahead` when
the code would run on both once mounted and the engine draws something Dew
skips or reads in part, `Dew-only` when it reaches API no Roblox place has, and
`error` when it did not load or built nothing.

<!-- BEGIN GENERATED: examples compatibility. Regenerate with `cargo run --manifest-path host/Cargo.toml -- compat`; CI runs it with `--check`. Do not edit by hand. -->

Globals the host installs that the engine has no counterpart for: `DewRoot`, `desktop`, `services`. Of those, the ones holding a root to parent into: `DewRoot`. Capability members that are a surface with an engine equivalent: `desktop.Widget`.

| Example | Verdict | Dew-only API it reaches | Set, and not read by Dew's renderer | Instances walked |
| :--- | :--- | :--- | :--- | ---: |
| `aether/calculator` | error | `desktop.Widget` (mount), Aether, which detects its host (runs on both) | did not load: calculator: runtime error: examples/aether/calculator/roblox_packages/.pesde/spektr+aether/0.0.0-fc1c826161741b9ca17f57c8a87e956e58117d46/aether/src/host/Host:187: Aether.Host.DataModel found a conforming DataModel but no host services: `Host.Text` needs a synchronous measurer and `Host.Clock` needs a frame source, and this environment offers neither `TextService`/`RunService` nor a `dew` global. | 0 |
| `aether/contextmenu` | error | `desktop.Widget` (mount), Aether, which detects its host (runs on both), `desktop` | did not load: contextmenu: runtime error: examples/aether/contextmenu/roblox_packages/.pesde/spektr+aether/0.0.0-fc1c826161741b9ca17f57c8a87e956e58117d46/aether/src/host/Host:187: Aether.Host.DataModel found a conforming DataModel but no host services: `Host.Text` needs a synchronous measurer and `Host.Clock` needs a frame source, and this environment offers neither `TextService`/`RunService` nor a `dew` global... | 0 |
| `aether/dialog-demo` | Roblox-ahead | `desktop.Widget` (mount), Aether, which detects its host (runs on both) | `Frame.BorderSizePixel`, `TextLabel.Font` (in part), `TextLabel.FontFace` (in part), `UICorner.CornerRadius` (in part), `UIStroke.Transparency` (in part) | 29 |
| `aether/pressable` | error | `DewRoot` (mount), a returned `mount` (mount), Aether, which detects its host (runs on both) | did not load: pressable: runtime error: examples/aether/pressable/roblox_packages/.pesde/spektr+aether/0.0.0-fc1c826161741b9ca17f57c8a87e956e58117d46/aether/src/host/Host:187: Aether.Host.DataModel found a conforming DataModel but no host services: `Host.Text` needs a synchronous measurer and `Host.Clock` needs a frame source, and this environment offers neither `TextService`/`RunService` nor a `dew` global. | 0 |
| `aether/slider-demo` | Roblox-ahead | `desktop.Widget` (mount), Aether, which detects its host (runs on both) | `Frame.BorderSizePixel`, `UICorner.CornerRadius` (in part) | 5 |
| `aether/styled-pressable` | Dew-only | `desktop.Widget` (mount), Aether, which detects its host (runs on both), `services` | `Frame.BorderSizePixel`, `UICorner.CornerRadius` (in part) | 11 |
| `aether/styled-pressable-recipe` | Dew-only | `desktop.Widget` (mount), Aether, which detects its host (runs on both), `services` | `Frame.BorderSizePixel`, `UICorner.CornerRadius` (in part) | 10 |
| `aether/timetracker` | error | `desktop.Widget` (mount), Aether, which detects its host (runs on both), `desktop.Audio`, `desktop.Notifications`, `desktop.Time` | did not load: timetracker: runtime error: examples/aether/timetracker/roblox_packages/.pesde/spektr+aether/0.0.0-fc1c826161741b9ca17f57c8a87e956e58117d46/aether/src/host/Host:187: Aether.Host.DataModel found a conforming DataModel but no host services: `Host.Text` needs a synchronous measurer and `Host.Clock` needs a frame source, and this environment offers neither `TextService`/`RunService` nor a `dew` global... | 0 |
| `aether/workspace` | error | Aether, which detects its host (runs on both), `desktop.Overlay`, permission `overlay` | did not load: workspace: runtime error: examples/aether/workspace/roblox_packages/.pesde/spektr+aether/0.0.0-fc1c826161741b9ca17f57c8a87e956e58117d46/aether/src/host/Host:187: Aether.Host.DataModel found a conforming DataModel but no host services: `Host.Text` needs a synchronous measurer and `Host.Clock` needs a frame source, and this environment offers neither `TextService`/`RunService` nor a `dew` global. | 0 |
| `host/basic-widget` | Roblox-ahead | `desktop.Widget` (mount) | `UICorner.CornerRadius` (in part) | 4 |
| `host/experimental-features` | Dew-only | `desktop.Widget` (mount), `Frame.BlendingMode` | none | 10 |
| `host/standalone` | Roblox-ahead | `DewRoot` (mount) | `UICorner.CornerRadius` (in part), `UIStroke.Thickness` (in part) | 7 |
| `host/widget-behaviors/click-through` | Roblox-ahead | `desktop.Widget` (mount) | `UICorner.CornerRadius` (in part) | 4 |
| `host/widget-behaviors/draggable` | Dew-only | `desktop.Widget` (mount), `desktop.Clock` | `UICorner.CornerRadius` (in part) | 4 |
| `host/widget-behaviors/keep-on-screen` | Roblox-ahead | `desktop.Widget` (mount) | `UICorner.CornerRadius` (in part) | 4 |
| `host/widget-behaviors/save-position` | Roblox-ahead | `desktop.Widget` (mount) | `UICorner.CornerRadius` (in part) | 4 |
| `host/widget-behaviors/snap-to-edges` | Roblox-ahead | `desktop.Widget` (mount) | `UICorner.CornerRadius` (in part) | 4 |
| `widgets/flex-item-demo` | Roblox-ahead | `desktop.Widget` (mount) | `UICorner.CornerRadius` (in part), `UIPadding.PaddingBottom` (in part), `UIPadding.PaddingLeft` (in part), `UIPadding.PaddingRight` (in part), `UIPadding.PaddingTop` (in part) | 68 |
| `widgets/flex-list-demo` | Roblox-ahead | `desktop.Widget` (mount) | `UICorner.CornerRadius` (in part), `UIPadding.PaddingBottom` (in part), `UIPadding.PaddingLeft` (in part), `UIPadding.PaddingRight` (in part), `UIPadding.PaddingTop` (in part) | 57 |
| `widgets/grid-layout-demo` | Roblox-ahead | `desktop.Widget` (mount) | `UICorner.CornerRadius` (in part), `UIPadding.PaddingBottom` (in part), `UIPadding.PaddingLeft` (in part), `UIPadding.PaddingRight` (in part), `UIPadding.PaddingTop` (in part), `UIStroke.Thickness` (in part), `UIStroke.Transparency` (in part) | 80 |
| `widgets/nameplate` | Dew-only | `desktop.Widget` (mount), `desktop.Clipboard`, `desktop.Storage`, `desktop.Time` | `ImageLabel.ScaleType` (in part), `UICorner.CornerRadius` (in part) | 13 |

Read in part, in the renderer's own words:

- `ImageLabel.ScaleType`: Stretch, Fit and Crop are drawn; Slice and Tile fall back to Stretch
- `TextLabel.Font`: a family Dew ships, or finds in a local Studio install, is drawn in its own face; any other is drawn in the nearest shipped face
- `TextLabel.FontFace`: a family Dew ships, or finds in a local Studio install, is drawn in its own face; any other is drawn in the nearest shipped face
- `UICorner.CornerRadius`: the offset rounds the corners; the scale is dropped
- `UIPadding.PaddingBottom`: the offset insets the content; the scale is discarded
- `UIPadding.PaddingLeft`: the offset insets the content; the scale is discarded
- `UIPadding.PaddingRight`: the offset insets the content; the scale is discarded
- `UIPadding.PaddingTop`: the offset insets the content; the scale is discarded
- `UIStroke.Thickness`: around glyphs, drawn as copies of the text stamped out to the thickness, so joins are always round
- `UIStroke.Transparency`: around glyphs, the stamped copies overlap, so a translucent outline reads more solid than the engine's

<!-- END GENERATED: examples compatibility -->
