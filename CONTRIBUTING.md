# Contributing

Start with [docs/contributing/guidelines.md](docs/contributing/guidelines.md).
It is the index, and it carries the short version of everything below it.

Three things worth knowing before the first pull request:

- **One observable goal per branch**, and the prefix says which kind of work it
  is. A branch that needs "and" to describe it is two branches.
- **`main` is always green.** Every commit on it compiles, passes its checks, and
  runs.
- **Plain ASCII**, in commits, pull requests and documentation alike.

## Before your first push

```sh
git config core.hooksPath .githooks
```

That points git at `.githooks/pre-push`, which runs the commit subject check, the
ASCII and boundary gates, and every command the CLI promises. It takes under
three seconds when nothing needs rebuilding.

It is worth the one line. The cheap CI job gates the expensive ones, so a commit
subject with the wrong scope costs a full round trip and reports nothing about
your code. All three of those gates run from the same scripts CI runs, so a pass
here is a pass there.

Run the CLI checks on their own with:

```sh
lune run scripts/smoke.luau
```

If you are writing a widget rather than changing the host, you probably want
[docs/applet_contract.md](docs/applet_contract.md) instead.
