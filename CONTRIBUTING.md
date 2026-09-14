# Contributing

Start with [docs/contributing/guidelines.md](docs/contributing/guidelines.md).
It is the index, and it carries the short version of everything below it.

Three things worth knowing before the first pull request:

- **One observable goal per branch**, and the prefix says which kind of work it
  is. A branch that needs "and" to describe it is two branches.
- **`main` is always green.** Every commit on it compiles, passes its checks, and
  runs.
- **Plain ASCII**, in commits, pull requests and documentation alike.

## Start here

```sh
lune run scripts/setup.luau
```

It checks the tools, points git at `.githooks/pre-push`, and says which examples
still need their own packages installed. Run it again whenever something looks
wrong; it reports rather than assumes and changes nothing that is already set.

The hook runs the commit subject check, the ASCII and boundary gates, and every
command the CLI promises, in under three seconds. It is worth having: the cheap
CI job gates the expensive ones, so a commit subject with the wrong scope costs a
full round trip and reports nothing about your code. Those gates run from the
same scripts CI runs, so a pass here is a pass there.

Run the CLI checks on their own with:

```sh
lune run scripts/smoke.luau
```

If you are writing a widget rather than changing the host, you probably want
[docs/applet_contract.md](docs/applet_contract.md) instead.
