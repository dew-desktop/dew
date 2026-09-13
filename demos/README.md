# Demos

Each directory here is an application that exercises Aether inside Dew, so
`framework-coverage` can report how much of the framework is demonstrated rather
than merely shipped.

**A demo installs its own framework.** It carries a `pesde.toml` naming Aether
and vide by commit, and requires them through the redirect `pesde install`
writes beside it. The host injects nothing, and Dew's own `[dependencies]` is
empty and gated -- see `.artifacts/project/milestones/8_.../decisions/adr-008`.

A demo returns:

| field | what it is |
| :--- | :--- |
| `Session` | what the host drives, from `Aether.Desktop.Mount` |
| `Width`, `Height` | the surface to paint |
| `Script` | the interaction to run, as data |
| `Aether` | the table the tool wraps to record what was used |
| `Measure` | mounts the same tree with a given Aether, for the coverage pass |

Optional readers a demo may also export -- `Presses`, `Hovered`, `Ticks` -- are
printed by the tool. They exist because a static differential cannot say whether
the input never arrived or the feature ignored it, and those are two bugs with
one symptom.

`Measure` exists because `Desktop.Mount` takes a builder rather than a tree: vide
refuses `derive` and `effect` outside a stable scope, so the proxy is handed into
the scope rather than wrapped around a finished tree.
