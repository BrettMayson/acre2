#include "script_component.hpp"
/*
 * Author: ACRE2Team
 * Get the internal state of a radio
 *
 * Arguments:
 * 0: Radio ID <STRING>
 *
 * Return Value:
 * Internal Radio Data <ARRAY>
 *
 * Example:
 * ["ACRE_PRC148_ID_1"] call acre_api_fnc_getRadioData;
 *
 * Public: Yes
 */

params ["_radioId"];

private _baseRadio = [_radioId] call EFUNC(api,getBaseRadio);
private _radioData = HASH_GET(EGVAR(sys_data,radioData), _radioId) call EFUNC(sys_data,serialize);
[_baseRadio, _radioData]
