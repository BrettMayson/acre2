// COMPONENT should be defined in the script_component.hpp and included BEFORE this hpp

#define MAINPREFIX idi
#define PREFIX acre

#include "\idi\acre\addons\main\script_version.hpp"

// Versions are MAJOR.MINOR.PATCHLVL: `hemtt publish` matches CHANGELOG.md entries by that, and
// its changelog parser only accepts three-part versions. BUILD stays for the version macros.
#ifndef BUILD
    #define BUILD 0
#endif

#define VERSION         MAJOR.MINOR
#define VERSION_STR     MAJOR.MINOR.PATCHLVL.BUILD
#define VERSION_AR      MAJOR,MINOR,PATCHLVL,BUILD
#define VERSION_PLUGIN  MAJOR.MINOR.PATCHLVL.BUILD

// MINIMAL required version for the Mod. Components can specify others..
#define REQUIRED_VERSION 2.14
#define REQUIRED_CBA_VERSION {3,19,0}

#ifdef COMPONENT_BEAUTIFIED
    #define COMPONENT_NAME QUOTE(YARD - COMPONENT_BEAUTIFIED)
#else
    #define COMPONENT_NAME QUOTE(YARD - COMPONENT)
#endif
