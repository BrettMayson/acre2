<p align="center">
    <img src="extras/logos/yard-128.png" alt="YARD" width="128">
</p>

<h1 align="center">YARD</h1>

<p align="center">
    <a href="https://github.com/BrettMayson/acre2/blob/master/LICENSE">
        <img src="https://img.shields.io/badge/License-GPLv3-red.svg?style=flat-square" alt="License">
    </a>
</p>

<p align="center">
    <sup><strong>Requires the latest version of <a href="https://github.com/CBATeam/CBA_A3/releases">CBA A3</a>.<br/></strong></sub>
</p>

**YARD** is a full fledged communications suite for Arma 3, allowing realistic radio and voice communications. It carries voice itself, hosted by the Arma server ([setup](docs/wiki/user/voice-server.md)): no TeamSpeak or other VoIP program needed.

YARD is a modified version of [ACRE2](https://github.com/IDI-Systems/acre2) by the ACRE2 Team (IDI-Systems), changed in 2026 to replace TeamSpeak 3 with a built-in voice server. It is not affiliated with or endorsed by the ACRE2 Team. YARD keeps ACRE2's API (`acre_api_fnc_*`, `ACRE_*` classnames, configs), so missions and mods made for ACRE2 work with it. It replaces ACRE2: don't load both.

The project is entirely **open-source** and all contributions are welcome. Like ACRE2, it is licensed under the GNU General Public License ([GPLv3](LICENSE)). Some folders contain a separate LICENSE file for items licensed differently.

### Features

- Accurate simulations of AN/PRC-343, AN/PRC-77, AN/PRC-148, AN/PRC-117F, AN/PRC-152, SEM 52 SL, SEM 70, BF-888S.
- Vehicle racks AN/VRC-64, AN/VRC-103, AN/VRC-110, AN/VRC-111, SEM90.
- Deployable antennas.
- Real time voice occlusion for world objects, including buildings.
- Babel language system that allows you to implement virtual languages on other players.
- Multi-channel audio, allowing people on the radio to be heard locally as well.
- Radios with external speakers.
- Possibility to share owned radios with other players.
- Highly detailed radio signal simulation.
- Zeus and spectator modes integration.
- Multiple signal propagation models: Arcade, LOS Single path, LOS Multipath (default) and, experimentally, the Longley-Rice (ITM) model.
- A highly customizable and documented framework for the creation of radios.
- A highly customizable and documented framework for implementing different game modes (persistent world, replays, logging, external data storage, external radio control).
- A highly customizable and documented framework for implementing intercom systems and infantry phones.
- A highly customizable and documented framework for implementing vehicle racks and their integration into the intercom system.
- Customizable sound configs that allow traditional patching of sounds played in YARD via Arma addons.
- Customizable radio presets, allowing mission makers to define radio presets as well as an API to take advantage of (as an example see the ACRE2 implementation in the [TMF], which works with YARD as well(https://github.com/TMF3/TMF)).
- Enhanced and easily configurable vehicle sound occlusion.
- Ground spike antenna for improved signal quality.

### Guides

If you installed YARD but have trouble understanding how it all works, or where to start, read this first:
- [Voice server setup](docs/wiki/user/voice-server.md)
- [ACRE2 documentation](https://acre2.idi-systems.com): most of it applies to YARD, except anything about TeamSpeak
