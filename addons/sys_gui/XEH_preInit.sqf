#include "script_component.hpp"

ADDON = false;

PREP_RECOMPILE_START;
#include "XEH_PREP.hpp"
PREP_RECOMPILE_END;

if (hasInterface) then {
    ACRE_HOLD_OFF_ITEMRADIO_CHECK = false;

    DVAR(ACRE_CustomVolumeControl) = nil;
    GVAR(stanceCache) = "";

    GVAR(volumeLevel) = 0.5;
    GVAR(volumeOpen) = false;
};

ADDON = true;
