#include "script_component.hpp"
/*
 * Author: ACRE2Team
 * Calls a signal command of the extension (`signal:<command>`: terrain, antennas, signal
 * strength, signal map).
 *
 * Arguments:
 * 0: Command, without the "signal:" group <STRING>
 * 1: Parameters <ARRAY>
 * 2: Run in the background: the result is passed to the callback later <BOOL><NUMBER> (default: false)
 * 3: Callback, called with [callback arguments, result] <CODE> (optional)
 * 4: Callback arguments <ANY> (optional)
 *
 * Return Value:
 * The result (nil when run in the background or on failure) <ANY>
 *
 * Example:
 * ["antenna", ["ACRE_123CM_VHF_TNC", "\idi\acre\addons\sys_antenna\binary\Harris_123cm_Whip_gain.aba"]] call acre_sys_core_fnc_callExt
 *
 * Public: No
 */

params ["_command", ["_params", []], ["_threaded", false], ["_callBack", {}], ["_callBackArgs", []]];

private _function = format ["signal:%1", _command];

if (_threaded isNotEqualTo false) exitWith {
    // The extension passes this slot back with the result (see the ExtensionCallback handler).
    private _index = if (GVAR(threadedExtFree) isEqualTo []) then {
        GVAR(threadedExtCalls) pushBack []
    } else {
        GVAR(threadedExtFree) deleteAt (count GVAR(threadedExtFree) - 1)
    };
    GVAR(threadedExtCalls) set [_index, [_callBackArgs, _callBack]];
    (EXTENSION_NAME callExtension [_function, [_index] + _params]) params ["_result", "_returnCode", "_errorCode"];
    if (_returnCode != 0 || {_errorCode != 0}) then {
        WARNING_4("Extension call %1 %2 failed (%3): %4",_function,_params,_returnCode,_result);
        GVAR(threadedExtCalls) set [_index, nil];
        GVAR(threadedExtFree) pushBack _index;
    };
    nil
};

(EXTENSION_NAME callExtension [_function, _params]) params ["_result", "_returnCode", "_errorCode"];
if (_returnCode != 0 || {_errorCode != 0}) exitWith {
    WARNING_4("Extension call %1 %2 failed (%3): %4",_function,_params,_returnCode,_result);
    nil
};
if (_result isEqualTo "") exitWith { nil };
(parseSimpleArray format ["[%1]", _result]) select 0
