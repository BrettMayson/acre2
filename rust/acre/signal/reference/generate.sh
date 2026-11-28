#!/usr/bin/env bash
# Regenerates ../tests/golden/signal.txt from the original C++ signal code
# (extensions/src/ACRE2Arma), on a real terrain and the shipped antennas. Needs g++.
# The C++ is in git history once extensions/ is gone: check out a commit that has it.
# Usage: ./generate.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$here/../../../.."
src="$root/extensions/src/ACRE2Arma"
out="$here/../tests/golden"
build="$(mktemp -d)"
trap 'rm -rf "$build"' EXIT

mkdir -p "$build/wrp" "$build/map" "$build/antenna" "$build/models" "$build/pbo"
cp "$src"/common/{compressed_base.hpp,compressed_base.cpp,vector.hpp,shared.hpp,membuf.hpp,singleton.hpp} "$build/"
cp -r "$src/common/glm" "$build/glm"
cp "$src"/common/wrp/* "$build/wrp/"
cp "$src"/signal/map/* "$build/map/"
cp "$src"/signal/antenna/antenna.{hpp,cpp} "$build/antenna/"
cp "$src"/signal/models/* "$build/models/"
cp "$root/extensions/src/ACRE2Shared/Types.h" "$build/"

# Stand-ins for logging, Windows headers and the PBO loader.
cat > "$build/logging.hpp" <<'H'
#pragma once
#include <iostream>
struct null_log { template<class T> null_log &operator<<(const T &) { return *this; } };
#define LOG(level) null_log()
H
: > "$build/targetver.h"
cat > "$build/pbo/fileloader.hpp" <<'H'
#pragma once
#include <memory>
#include <istream>
namespace acre { namespace pbo {
    struct file_entry { std::istream &stream(); };
    typedef std::shared_ptr<file_entry> file_entry_p;
    struct fileloader {
        static fileloader &get() { static fileloader f; return f; }
        bool get_file(std::string, std::string, file_entry_p &) { return false; }
    };
} }
H
cat > "$build/compat.hpp" <<'H'
#pragma once
#include <cstddef>
#include <cstring>
#include <cmath>
#include <cstdint>
#include <algorithm>
#include <string>
typedef unsigned char byte;
namespace stdext { template<class T> T checked_array_iterator(T p, size_t) { return p; } }
#define register
H
# common/vector.hpp declares placeholder `acre::cos/sin/acos` templates that return -1.
# Unqualified calls inside namespace acre find them first, so the shipped ITM computed its
# line-of-sight term with cos = sin = -1. The Rust port uses the real functions; compare with
# the C++ as intended.
sed -i '/T \(acos\|cos\|sin\)(T n) { return -1; }/d' "$build/vector.hpp"
# ITM was given the antennas' heights above sea level (receiver first) instead of above ground
# (transmitter first). Apply the same fix as the Rust port.
python3 "$here/fix_itm_heights.py" "$build/models/longleyRice.cpp"
# MSVC-isms: <malloc.h> exists on glibc; `long` for the LZSS checksum is 4 bytes on Windows.
sed -i 's/in.read((char \*)&csr, sizeof(long));/in.read((char *)\&csr, 4);/' "$build/compressed_base.cpp"

g++ -O2 -std=c++17 -w -include "$build/compat.hpp" -I"$build" -I"$build/glm/.." \
    -DGLM_FORCE_PURE -o "$build/harness" \
    "$here/harness.cpp" "$build/compressed_base.cpp" "$build/wrp/landscape.cpp" \
    "$build/map/map.cpp" "$build/antenna/antenna.cpp" "$build/models/los_simple.cpp" \
    "$build/models/longleyRice.cpp"

mkdir -p "$out"
"$build/harness" "$root/addons/compat_sog/vn_the_bra.fakewrp" \
    "$root/addons/sys_antenna/binary/Harris_123cm_Whip_gain.aba" \
    "$root/addons/sys_antenna/binary/prc343_gain.aba" \
    "$out/signal.txt"
wc -l "$out/signal.txt"
