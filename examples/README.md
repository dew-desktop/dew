# Examples

Not mods. An applet is discovered from `applets/`, carries a `dew.toml`, and is granted
capabilities by its manifest. These are the smallest programs that demonstrate one
thing each, run directly.

## `standalone/`

A Dew applet with no framework at all.

```sh
cargo run --manifest-path host/Cargo.toml -- \
    --script examples/standalone/app.luau --size 360x220 --snapshot out.png
```

Every line of it would build the same tree inside Roblox: `Instance.new`,
property assignment, `Parent`, and the vocabulary. Nothing is imported.

It exercises the whole DataModel path end to end -- the reflection database
refusing a class or property that does not exist, the vocabulary types, `Enum`
resolved from that same database, the layout pass, and the rasteriser. **If it
renders, the path from Luau to pixels is alive.**

It parents into `DewRoot` rather than `game`. `Host.detect()` in Aether keys on
`typeof(game) == "Instance"`, so installing a `game` global before the services
and the member surface are behind it would flip every Aether mod in the same
binary onto the Roblox branch and break it.
