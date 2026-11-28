# Changelog

All notable changes to YARD are documented in this file. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

`hemtt publish` uses the entry for the current version (`addons/main/script_version.hpp`) as the Steam Workshop change note.

## [Unreleased]

## [3.0.0] - 2026-11-28

First release of YARD, based on ACRE2 2.14.0.

### Added

- Built-in voice: the Arma server hosts the voice server, so no TeamSpeak or plugin is needed. Set the public address in the YARD Voice Server settings and open UDP port 9987.
- Voice activation with noise suppression and speech detection, or push-to-talk for direct speech.
- Microphone and speaker selection, a microphone test, a mute key (Ctrl+Shift+M), a push-to-talk release delay, and hearing yourself on your other radios.
- A YARD Voice page on the map: connection, latency, packet loss and who you're hearing.
- Linux dedicated server support.
- `acre_api_fnc_getAudioDevices`, `acre_api_fnc_getMicrophoneLevel` and `acre_api_fnc_openMicrophoneTest`.

### Changed

- One extension, `yard_x64.dll` / `yard_x64.so`, replaces ACRE2's signal extension and TeamSpeak plugin.
- The signal map debug tool works without ACRE2's developer tools.

### Fixed

- Longley-Rice (ITM) used placeholder `cos`/`sin` functions that returned -1.
- Longley-Rice (ITM) was given antenna heights above sea level instead of above the ground. ITM signals are now much weaker than in ACRE2.
- LOS Multipath results depended on what had been computed before.

### Removed

- TeamSpeak channel switching and the other TeamSpeak-only settings.
