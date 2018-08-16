/*
 * Author: ACRE2Team
 * Checks whether the provided unit is currently speaking, either on radio or direct.
 *
 * Arguments:
 * 0: Unit <OBJECT>
 *
 * Return Value:
 * Is unit speaking <BOOLEAN>
 *
 * Example:
 * _isSpeaking = [player] call acre_api_fnc_isSpeaking;
 *
 * Public: Yes
 */
#include "script_component.hpp"

params ["_unit"];

_unit in EGVAR(sys_core,speakers)
