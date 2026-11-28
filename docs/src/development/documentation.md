# Documentation

This site is built with [mdBook](https://rust-lang.github.io/mdBook/) from the `docs/` folder, and published to GitHub Pages by the [docs workflow](https://github.com/BrettMayson/yard/blob/main/.github/workflows/docs.yml) on every push to `main`.

## Building locally

```sh
cargo install mdbook
cd docs
mdbook serve --open
```

`mdbook serve` rebuilds and reloads the page as you edit.

## Writing pages

- Pages are Markdown in `docs/src/`. Add new pages to `docs/src/SUMMARY.md`, which is the sidebar.
- Link to other pages with relative `.md` paths (`../user/installation.md`); mdBook checks them and turns them into links to the HTML pages.
- Images go in `docs/src/images/`.
- Callouts use GitHub's syntax: `> [!NOTE]`, `> [!TIP]`, `> [!IMPORTANT]`, `> [!WARNING]`.

## Functions List

[Functions List](../frameworks/functions-list.md) is generated from the headers of the public functions in `addons/api`:

```sh
python3 docs/tools/document_functions.py api --output docs/src/frameworks/functions-list.md
```

The docs workflow regenerates it on every build. `--debug` (as CI runs it) only checks the headers.
