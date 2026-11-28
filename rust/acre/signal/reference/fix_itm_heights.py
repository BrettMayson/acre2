#!/usr/bin/env python3
"""Applies the ITM antenna height fix to the C++ longleyRice.cpp given as the argument.

ITM's point_to_point takes each antenna's height above ground level, transmitter first (the
terrain profile runs from the transmitter). The C++ passed the positions' heights above sea
level, receiver first. This is the same change as the Rust port (models/itm.rs) and the
upstream ACRE pull request; generate.sh applies it so the golden data matches.
"""
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
source = path.read_text()

anchor = "    if (useITWOM) {\n"
heights = """    // ITM expects antenna heights above ground level, transmitter first (the terrain profile runs from tx to rx).
    // Clamped to ITM's valid range: an antenna slightly below the interpolated terrain would otherwise give NaN.
    const float64_t txHeight = (std::min)((std::max)(static_cast<float64_t>(tx_pos.z - _map->elevation(tx_pos.x, tx_pos.y)), 0.5), 3000.0);
    const float64_t rxHeight = (std::min)((std::max)(static_cast<float64_t>(rx_pos.z - _map->elevation(rx_pos.x, rx_pos.y)), 0.5), 3000.0);

"""
old_args = "itmElevations.data(), static_cast<float64_t>(rx_pos.z), static_cast<float64_t>(tx_pos.z),"
new_args = "itmElevations.data(), txHeight, rxHeight,"

if source.count(anchor) != 1 or source.count(old_args) != 2:
    sys.exit(f"{path}: unexpected contents, not patched")
source = source.replace(anchor, heights + anchor).replace(old_args, new_args)
path.write_text(source)
