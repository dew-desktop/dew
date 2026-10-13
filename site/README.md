# site

The documentation website. It renders the Markdown in [`docs/`](../docs) with
[Starlight](https://starlight.astro.build/); the content lives there, and this
directory holds only how it looks.

```sh
pnpm install
pnpm dev     # http://localhost:4321
pnpm build   # dist/, then fails on any link that does not resolve
```

See [Documentation](../CONTRIBUTING.md#documentation) in CONTRIBUTING.md.
