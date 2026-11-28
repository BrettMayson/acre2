# YARD

Radio and voice communications for Arma 3, with voice built in. No TeamSpeak, no plugin, no voice program to install: the Arma server hosts voice and players just join.

YARD is a modified version of ACRE2 and keeps its API, so missions and mods made for ACRE2 work with it. **Don't load YARD and ACRE2 together.**

Requires **CBA A3**.

## Features

- Simulated radios: AN/PRC-343, AN/PRC-148, AN/PRC-152, AN/PRC-117F, AN/PRC-77, SEM 52 SL, SEM 70 and BF-888S
- Radio signal over the terrain, with antenna patterns and several propagation models
- Vehicle racks, vehicle intercoms and infantry phones
- Voice occlusion by buildings and objects, and vehicle sound attenuation
- Babel: give sides or players their own languages
- Radios with external speakers, and sharing radios with other players
- Zeus and spectator integration
- Creator DLC compatibility: Global Mobilization, S.O.G. Prairie Fire, Western Sahara, CSLA Iron Curtain, Spearhead 1944

## Voice

- Voice activation with noise suppression and speech detection, or push-to-talk
- Pick your microphone and speakers in the addon options
- A microphone test that suggests voice activation levels
- A mute key, a push-to-talk release delay, and hearing yourself on your other radios
- Encrypted, and only sent to players who can hear it

## For server administrators

Set the voice server's public address in **Addon Options → YARD Voice Server** (server settings) and open **UDP port 9987**. That's all. Linux dedicated servers are supported.

## Coming from ACRE2

The radios, signal simulation and gameplay are ACRE2's. Your ACRE2 missions, settings and keybinds carry over.

## Links

- Documentation: [brettmayson.github.io/yard](https://brettmayson.github.io/yard/)
- Voice server setup: [brettmayson.github.io/yard/user/voice-server.html](https://brettmayson.github.io/yard/user/voice-server.html)
- YARD and ACRE2: [brettmayson.github.io/yard/yard-vs-acre.html](https://brettmayson.github.io/yard/yard-vs-acre.html)
- Source and issues: [github.com/BrettMayson/yard](https://github.com/BrettMayson/yard)

YARD is based on ACRE2 by the ACRE2 Team (IDI-Systems), and is not affiliated with or endorsed by them. Licensed under the GNU GPL v3.
