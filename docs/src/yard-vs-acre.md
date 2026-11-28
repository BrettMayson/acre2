# YARD and ACRE2

YARD is a modified version of [ACRE2](https://github.com/IDI-Systems/acre2) by the ACRE2 Team (IDI-Systems). It is not affiliated with or endorsed by the ACRE2 Team. The name is a nod to its origin: an acre and a yard are both units of measure.

The radios, the signal simulation, the intercoms, babel and the rest of the gameplay come from ACRE2. What YARD changes is underneath: how voice gets from one player to another, and the extension that runs it.

## At a glance

| | ACRE2 | YARD |
| --- | --- | --- |
| Voice | TeamSpeak 3 with the ACRE2 plugin | Built in: the Arma server hosts the voice server |
| Players need | TeamSpeak 3, the plugin, and the right TeamSpeak server and channel | Nothing beyond the mod |
| Server needs | A TeamSpeak server | One open UDP port and its public address in the CBA settings |
| Direct speech | TeamSpeak's activation | Voice activation (with noise suppression and speech detection) or push-to-talk |
| Extensions | `acre_x64.dll` (C++), the TeamSpeak plugin | One extension, `yard_x64.dll` / `yard_x64.so` (Rust) |
| Linux dedicated servers | Supported (the server needs no extension) | Supported: the server hosts voice with `yard_x64.so` (included) |
| API, classnames, configs | `acre_api_fnc_*`, `ACRE_*`, `CfgAcreComponents` | The same |
| Mod folder | `@acre2` | `@yard` |

## Compatibility

YARD keeps ACRE2's API, so missions and mods made for ACRE2 work with it:

- The `acre_api_fnc_*` functions, their arguments and return values. None were removed.
- The `ACRE_*` item and vehicle classnames, `CfgAcreComponents`, the vehicle attenuation, intercom and rack configs, and the `acre_*` CfgPatches, so addons that require ACRE2's addons load.
- The CBA events and the `acre_*` variables.
- CBA setting names, so existing settings files and mission settings still apply.
- Keybinds. They show under *YARD* in the controls menu, but keep ACRE2's IDs, so binds carry over.

> [!IMPORTANT]
> YARD replaces ACRE2. Don't load both: they use the same addon names.

## Voice

ACRE2 used TeamSpeak 3: every player ran TeamSpeak with the ACRE2 plugin in the same channel, and each plugin decided what its player should hear. YARD carries voice itself:

- **The Arma server hosts the voice server.** There's nothing extra to install or run. Admins set the server's public address and open one UDP port (default 9987). See [Voice Server](user/voice-server.md).
- **Players connect automatically** with a one-time ticket from the Arma server, so only players on that server can join its voice. Voice is encrypted.
- **The server routes voice**: direct speech goes to players close enough to hear it, radio traffic to players with a radio on that channel. Everyone else never receives it, so bandwidth doesn't grow with the size of the server the way TeamSpeak's did.
- **Radio effects, babel and positional sound** are applied by each listener, as before. The radio and babel effects are the ACRE2 ones, ported and tested against the originals. Positional sound replaces Microsoft's X3DAudio with a reimplementation of its behaviour, so stereo placement may differ slightly.

New for players:

- Pick your microphone and speakers in the addon options.
- Direct speech by **voice activation** (default) or **push-to-talk**. Noise suppression and speech-only activation are on by default, so fans and typing don't transmit.
- A **microphone test** (`Escape → Options → YARD Microphone Test`) that suggests voice activation levels.
- A **mute key** (`Ctrl+Shift+M`), a configurable **push-to-talk release delay**, and an option to **hear yourself on your other radios** tuned to the same channel.
- A **YARD Voice** page on the map: connection, latency, packet loss and who you're hearing.

Settings that only made sense with TeamSpeak are gone: TeamSpeak channel switching (`ts3ChannelSwitch`, `ts3ChannelName`, `ts3ChannelPassword`) and `unmuteClients`. `acre_api_fnc_isVOIPConnected` now reports the connection to the voice server.

New API functions: `acre_api_fnc_getAudioDevices`, `acre_api_fnc_getMicrophoneLevel`, `acre_api_fnc_openMicrophoneTest`.

## Signal

The signal models are ported from ACRE2's C++ extension and tested against it. Arcade, LOS Simple and LOS Multipath give the same results as ACRE2, except for one fix:

- **LOS Multipath** kept the terrain peaks it had found around every transmitter position it had ever computed, so results depended on what had been computed before. YARD uses the peaks around the current transmitter.

**Longley-Rice (ITM)** had two bugs in ACRE2, both fixed in YARD and reported upstream ([#1435](https://github.com/IDI-Systems/acre2/pull/1435), [#1436](https://github.com/IDI-Systems/acre2/pull/1436)):

- Its line-of-sight attenuation used placeholder `cos` and `sin` functions that always returned -1.
- It was given the antennas' heights above sea level instead of above the ground, so a radio on a hill was modelled as a mast as tall as the hill.

Both made ITM over-optimistic. Expect ITM signals in YARD to be much weaker than in ACRE2: typically around 20 dB, and more on hilly terrain. ITM is still marked experimental.

The signal map debug tool now works anywhere (it used to depend on a tool only the ACRE2 developers had).

## For server administrators

- Set the **public address** in `Addon Options → YARD Voice Server` (server settings), or `YARD_VOICE_ADDRESS`. The port defaults to 9987 (`YARD_VOICE_PORT`).
- Open that UDP port in the firewall.
- Linux dedicated servers are supported. They need `yard_x64.so` to host voice; it's included in the release.
- Logs go to `yard_ext.log` next to the RPT (`YARD_LOG=debug` for more). The older `ACRE2_VOICE_*` and `ACRE2_LOG` names still work.

## For modders

- Nothing changes for addons that use ACRE2's API or configs.
- The extension is `yard` (`"yard" callExtension […]`), not `acre`. Nothing in the public API called the extensions directly.
- Terrains whose WRP can't be read still need a `.fakewrp` (see [Terrain Files](frameworks/terrain-files.md)); the `fakewrp` tool replaces ACRE2's `fakewrpgen`, and `wav2b64` replaces `Wav2B64.exe`.
