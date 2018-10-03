class CfgVehicles {
    class ACE_ZeusActions {
        class ACRE_ZeusEars {
            displayName = CSTRINGS(acreEars);
            condition = QUOTE(true);
            statment = "";
            class ACRE_SpectatorEars {
                displayName = CSTRINGS(spectatorEars);
                condition = QUOTE(!ACRE_IS_SPECTATOR);
                statement = QUOTE([true] call EFUNC(api,setSpectator));
            };
            class ACRE_ZeusEars {
                displayName = CSTRINGS(zeusEars);
                condition = QUOTE(ACRE_IS_SPECTATOR);
                statement = QUOTE([false] call EFUNC(api,setSpectator));
            };
        };
    };
};
