# Terrain Files (fakewrp)

YARD computes radio signal over the terrain. When a mission starts it finds the terrain's `.wrp` file in the game's loaded PBOs and reads its heights. Some terrains can't be read that way, usually because they're published encrypted. For those, YARD needs a `.fakewrp`: a file with just the terrain's heights and peaks, referenced from `CfgAcreWorlds`.

YARD's Creator DLC compatibility addons ship `.fakewrp` files for the Creator DLC terrains.

Without terrain data, signal falls back to the *Arcade* model (no terrain).

## Making a fakewrp

The `fakewrp` tool (built with the extension, see [Building the Extension](../development/building-extensions.md)) makes one either from a WRP it can read, or from heights dumped in game.

### From a WRP

```sh
fakewrp convert path/to/terrain.wrp terrain.fakewrp
```

### From heights dumped in game

Run this in the debug console on the terrain (it takes a while on big terrains), then paste the clipboard into a file, `heights.csv`:

```sqf
private _mapSize = worldSize;
private _gridSize = 4096;
private _interval = _mapSize / _gridSize;
private _heights = [];
for "_y" from 0 to (_mapSize - _interval) step _interval do {
    for "_x" from 0 to (_mapSize - _interval) step _interval do {
        _heights pushBack getTerrainHeightASL [_x, _y];
    };
};
copyToClipboard (_heights joinString ",");
```

Then:

```sh
fakewrp generate --world-size <worldSize> --map-size 4096 heights.csv terrain.fakewrp
```

## Using it

Put the `.fakewrp` in your addon and point YARD at it:

```cpp
class CfgAcreWorlds {
    class YourWorldName {
        wrp = "\your\addon\path\terrain.fakewrp";
    };
};
```

The class name is the terrain's `worldName`.
