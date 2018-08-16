/*
 * Author: ACRE2Team
 * Gets the volume for the given radio.
 *
 * Arguments:
 * 0: Radio ID <STRING>
 *
 * Return Value:
 * Volume value between 0 and 1. -1 if error <Number>
 *
 * Example:
 * [ARGUMENTS] call acre_api_fnc_getRadioVolume
 *
 * Public: Yes
 */
#include "script_component.hpp"

params ["_radioId"];

if (isNil "_radioId") exitWith { -1 };

if (!([_radioId] call EFUNC(sys_data,isRadioInitialized))) exitWith { -1 };

[_radioId] call EFUNC(sys_radio,getRadioVolume);
