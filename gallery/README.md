# The gallery

Scenes that show the property surface doing something, rendered to images a
person looks at.

`datamodel-surface` reports what the host ACCEPTS -- 139 of 139 in-scope
properties. That says none of them is rejected. It does not say any of them
draws anything. This is what says that.

    cargo run --bin gallery-coverage               # both numbers
    cargo run --bin gallery-coverage -- --render   # write the PNGs too
    cargo run --bin gallery-coverage -- --scenes   # every scene, by pillar
    cargo run --bin gallery-coverage -- --missing  # what nothing shows yet
    cargo run --bin gallery-coverage -- --no-diff  # skip the differential pass

`--render` writes to `gallery/renders/<pillar>/`, which is gitignored. Pass a
path after it to write somewhere else. It does NOT default into `target/`:
that belongs to cargo, and `cargo clean` deleted the whole gallery twice on the
day this was written.

## Two numbers, and the second is the one that matters

    DEMONSTRATED: 32 of 139   a scene SETS the property
    DIFFERENTIAL: 22 of 139   changing it MOVED PIXELS

`DEMONSTRATED` is satisfied by a property the renderer ignores entirely: setting
it changes nothing and nobody looks. `DIFFERENTIAL` renders the scene twice --
once with one property changed -- and asks whether the image moved. That cannot
be satisfied by a property that does nothing.

The difference is not academic. Switching `BackgroundColor3` off in the renderer
leaves `DEMONSTRATED` at 32 and drops `DIFFERENTIAL` from 22 to 10.

**A variant that changes nothing is reported, not swallowed.** It is the most
useful line in the report: the property reached the host and did not reach the
pixels. Three currently do, and each is a real gap rather than a scene defect --
`TextWrapped`, `TextTransparency` and `UIListLayout.HorizontalAlignment`.

Some properties cannot move a pixel on their own -- `Name`, `Parent`, `Active`,
`InputSink` -- and are excused in `CANNOT_DIFFER` with a stated reason. A reason,
not a name: a bare entry is indistinguishable from something nobody got round to.

## Adding a scene

Drop a `.luau` file in `gallery/scenes/<pillar>/`. It is picked up
automatically; there is nothing to register.

```luau
return {
    name = "TextLabel: colour, size and alignment",
    shows = "The same font at four alignments and transparencies.",
    surface = { width = 300, height = 160 },
    tree = {
        class = "Frame",
        name = "Backdrop",
        props = { Size = { "UDim2", 1, 0, 1, 0 } },
        children = {
            { class = "TextLabel", name = "Heading", props = { Text = "hello" } },
        },
    },
}
```

Add `variants` to have properties checked differentially:

```luau
    variants = {
        { node = "Heading", prop = "TextColor3", value = { "Color3", 1, 0, 0 } },
        { node = "Heading", prop = "TextSize", value = 32 },
    },
```

Each one repaints the scene with that property changed on the node of that
`name`, and compares. Choose a value that should visibly differ from the base --
a variant that cannot show a difference reports the property as inert and is
indistinguishable from a genuine gap.

Values use the same typed encoding the conformance cases use:
`{ "UDim2", 0, 100, 0, 48 }`, `{ "Color3", 0.3, 0.62, 0.94 }`,
`{ "Enum", "TextXAlignment", "Center" }`, `{ "ColorSequence", c1, c2 }`.

**`shows` is required.** One sentence, saying what to look for in the image. It
is the caption the generated index prints under the thumbnail, and it is the
only prose in this gallery that grows as the gallery does -- everything
navigable is derived from it, so there is one place to write it and no second
copy to drift.

## The pillars

One directory level under `scenes/`, each an area of behaviour a person can hold
in their head.

| Pillar | What belongs there |
| :--- | :--- |
| `geometry` | position, size, anchor, automatic sizing, constraints |
| `paint` | background, border, corners, stroke, gradient, clipping, z-order |
| `text` | font, alignment, wrapping, scaling, rich text, placeholders |
| `image` | `Image`, slice and tile, `ScaleType`, rect, tint, button states |
| `layout` | the `UI*Layout` family, flex, cell sizing, page transitions |
| `scrolling` | canvas, scrollbars, elastic behaviour, insets |
| `interaction` | `Active`, `Interactable`, `InputSink`, `Modal`, `Selected` |

**Not grouped by class, deliberately.** Properties cross classes --
`BackgroundColor3` is on every `GuiObject`, and every scene here already spans
two to four of them -- so a class tree forces arbitrary choices about where a
shared property's scene lives. Per-class coverage is a number the tool computes;
do not encode in a directory tree what a tool can derive.

A scene sitting loose in `scenes/` is reported as `ungrouped` rather than
refused. The report lists every pillar directory including the empty ones,
because a pillar with no scenes is the most useful line in it.

## What a scene is not

**A conformance case.** They share the format and the renderer and nothing else.
A scene carries no `provenance`, no `verifiedAgainst`, no `expect` and no
`pixels`, and `decode_scene` REFUSES a file carrying any of them rather than
ignoring it.

A scene asserts nothing about Roblox. It shows what this host draws. If a file
wants to state a belief about engine behaviour it is a conformance case and
belongs in `aether/conformance/cases`.

The reasoning is
[ADR-005](../.artifacts/project/milestones/5_the_surface_demonstrates_itself/decisions/adr-005-the-gallery-is-not-a-conformance-runner.md):
conformance is evidence and wants to be narrow and falsifiable; a gallery wants
breadth and wants to look good. Merged, the incentives resolve the wrong way and
the ugly informative cases get sanded off.

## Scenes show gaps rather than hiding them

Two scenes currently demonstrate that something does NOT work, and say so in
their header:

- `text/text_labels_and_alignment.luau` -- `TextWrapped` grows the element's box
  but the painted string is one clipped line. The display list carries no
  wrapped flag and `fill_text` does no line breaking.
- `paint/frame_backgrounds_and_corners.luau` -- `UICorner` with a scale radius
  does not round, because `corner_radius` returns the offset and drops the
  scale.

A scene edited to look correct is a scene that lies, and the gallery exists to
catch exactly the class of gap that the tests and the conformance suite cannot
see: the ones where geometry agrees and pixels do not.

## In CI

The render runs in the Linux job. A scene that stops rendering fails the build,
because a gallery nobody re-renders is a folder of screenshots that rots.

The coverage number is printed and not gated on a threshold. Gating it would
make an honest early figure a build failure; the number moving is a sprint's job
to report.
