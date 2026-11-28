# Contributing to YARD

## Reporting an issue

Make sure YARD really is the cause: reproduce the issue with only `@CBA_A3` and `@yard` on a newly created mission. Removing `@yard` from the mod list doesn't show that YARD is the culprit. If the issue only happens with a third-party mod, report it to that mod's author.

Then [open an issue](https://github.com/BrettMayson/yard/issues/new/choose) and fill in the template, including your client and server RPT files and `yard_ext.log`. Say whether it happens on a release or a development build (with the commit hash).

Voice problems: include what the *YARD Voice* page on the map shows, and the server's RPT (it logs why the voice server couldn't start or why players can't connect).

## Requesting a feature

Search the [issues](https://github.com/BrettMayson/yard/issues) first, and comment on an existing request rather than opening a new one.

## Pull requests

- Read [Building](https://brettmayson.github.io/yard/development/building.html) and the [SQF coding guidelines](https://brettmayson.github.io/yard/development/coding-guidelines-sqf.html).
- The extension is Rust, in `rust/`: run `cargo fmt`, `cargo clippy` and `cargo test` (see [Building the Extension](https://brettmayson.github.io/yard/development/building-extensions.html)).
- Keep ACRE2's API (`acre_api_fnc_*`, `ACRE_*` classnames, configs) compatible: missions and mods made for ACRE2 rely on it.
- Title pull requests `System - Add|Fix|Improve|Change|Make|Remove ...`.
