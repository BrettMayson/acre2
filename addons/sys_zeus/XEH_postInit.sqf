#include "script_component.hpp"
#include "\a3\editor_f\Data\Scripts\dikCodes.h"

private _category = format ["ACRE2 %1", localize "str_a3_cfghints_curator_curator_displayname"];

[_category, "ZeusTalkFromCamera", [LLSTRING(SpeakFromCamera), LLSTRING(SpeakFromCamera_Description)],
    { call FUNC(handleZeusSpeakPress) },
    { call FUNC(handleZeusSpeakPressUp) },
[DIK_GRAVE, [false, false, false]]] call CBA_fnc_addKeybind;

if (hasInterface) then {
    if (isClass (configFile >> "CfgPatches" >> "ace_interact_menu")) then {
        private _spectatorEars = [
            QGVAR(spectatorEars),
            LLSTRING(SpectatorEars),
            "",
            { call EFUNC(sys_core,spectatorOn) },
            { GVAR(zeusCanSpectate) && {!ACRE_IS_SPECTATOR} }
        ] call ace_interact_menu_fnc_createAction;
        [["ACE_ZeusActions", "ACRE_Zeus"], _spectatorEars] call ace_interact_menu_fnc_addActionToZeus;

        private _zeusEars = [
            QGVAR(zeusEars),
            LLSTRING(ZeusEars),
            "",
            { call EFUNC(sys_core,spectatorOff) },
            { GVAR(zeusCanSpectate) && {ACRE_IS_SPECTATOR} }
        ] call ace_interact_menu_fnc_createAction;
        [["ACE_ZeusActions", "ACRE_Zeus"], _zeusEars] call ace_interact_menu_fnc_addActionToZeus;
    };

    // Virtual Zeus
    if (player isKindOf "VirtualCurator_F") then {
        // Used to store radios
        player addBackpack "B_carryall_mcamo";
    };
};
