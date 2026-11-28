# Releasing

A release goes to two places: a GitHub release (built by CI from a tag) and the Steam Workshop (uploaded with [`hemtt publish`](https://hemtt.dev/commands/publish.html) from a machine running Steam).

## 1. Version and changelog

1. Set the version in `addons/main/script_version.hpp` (`MAJOR`, `MINOR`, `PATCHLVL`). Versions have three parts: there's no `BUILD` there, because `hemtt publish` looks the version up in the changelog, and its parser only accepts `x.y.z`.
2. In `CHANGELOG.md`, move the notes under `[Unreleased]` into a new `## [x.y.z] - YYYY-MM-DD` section. It follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this section becomes the Workshop change note.
3. Commit, tag `vx.y.z` and push the tag. The [release workflow](https://github.com/BrettMayson/yard/blob/main/.github/workflows/release.yml) builds the mod and the extension and drafts a GitHub release with the zip.

## 2. The extension

`hemtt publish` packs whatever extension binaries are in the repository root, so put the release ones there first: `yard_x64.dll`, `yard_x64.so` and `extras/wav2b64.exe`. Either:

- run `./build-extension.sh` (Linux, cross-compiles the Windows DLL), or
- take them from the zip the release workflow built.

> [!WARNING]
> Without them the Workshop item has no extension, and YARD can't connect to voice or compute signal.

## 3. Steam Workshop

The Workshop page is configured in [`.hemtt/publish.toml`](https://github.com/BrettMayson/yard/blob/main/.hemtt/publish.toml): title, visibility, tags, CBA as a required item, the description (`workshop/description.md`, converted from Markdown to Steam BBCode) and the images in `workshop/`.

1. Check everything first, without connecting to Steam:

   ```sh
   hemtt publish --dry-run
   ```

   It builds the release and prints the title, tags, images, description and change note that would be sent. Check that `.hemttout/release` contains `yard_x64.dll` and `yard_x64.so`.

2. With Steam running and logged in to the account that owns the item:

   ```sh
   hemtt publish
   ```

The first time, HEMTT asks to create the Workshop item and writes its ID to `meta.cpp` (there is no `meta.cpp` until then). **Commit `meta.cpp`**: later runs update that item. Steam keeps a new item hidden until its owner has accepted the [Workshop legal agreement](https://steamcommunity.com/sharedfiles/workshoplegalagreement).

`tags`, `required_items` and `additional_previews` in `publish.toml` replace what's on the Workshop page on every publish, so change them there, not on the page.

## 4. GitHub release

Publish the draft release the workflow created, with the changelog section as its notes.

> [!NOTE]
> The DLLs aren't BattlEye-signed or whitelisted yet, so servers running BattlEye block the extension.
