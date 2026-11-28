# Feature List

An "at a glance" overview of what YARD does. YARD inherits ACRE2's radios and simulation, built up by the ACRE2 Team over many years, and adds its own voice system. See [YARD and ACRE2](../yard-vs-acre.md) for what differs.

### High-Level Summary

- Provides realistic sounding radio and direct voice communications for ArmA3
- Signal loss and degradation is done using REAL WORLD ITU Terrain loss modeling and signal propagation techniques. We model everything from mapping terrain and creating loss maps to the antenna's, power and frequencies of the radios
- 'Regular' talking can travel over 100 meters in ideal terrain
- Direct speaking occlusion, buildings and objects block your voice
- AI direct speaking detection; AI can also hear you talking and alert to you
- All radios are modeled and SIMULATED to their real world counterparts
- Easy to use, only a few buttons are required to play
- Babel feature & Radio configurations allows for immersive TVT and brings radios/communication in as a gameplay mechanic
- Extensive and documented API and example set for easy use
- Template setups for milsim community operations
- All vehicles have realistic attenuation, meaning different positions hear inside/outside differently
- Vehicle intercom system; Vehicles with intercoms automatically have crew communicate with them. Additionally some vehicles like tanks and IFVs have Infantry Telephones
- Vehicle racks; Vehicles can now have radio racks mounted that allow increasing the transmitting power of mounted radios. It is integrated into vehicle intercom functionality
- In-game speakers; you can put your radio on 'speaker' mode and everyone can listen. If you drop the radio, it keeps playing!
- Multiple Push-To-Talk keys, for assigning different radios to different hotkeys
- Preset naming and descriptions for radios that support it. Includes name showing in hints
- External radios; radios in other player's inventory can be accessed and used
- Built-in voice: no TeamSpeak or other VoIP program needed

### Radio Communications

- ITU Signal Loss and Terrain Modelling
- Radio Interfaces
- Easy to Use
- API/Pre-programming for milsim groups
- Antennas and Components
- Ground spike antenna


## Direct Communications

- Babel (TVT Languages)
- Distance and Scaling
- Building/object Occlusion
- Vehicle Intercoms and Infantry Telephones
- Vehicle Attenuation
- AI Hears you
- Possibility to send voice and text messages to groups of players without attenuation (God Mode)

### TVT

#### Example TVT Gameplay

You can have a TVT with 2 sides. Each side speaks its own babel language, except for some translators. Each side also has their own radio configurations. This would mean that even if you stole another sides radio, you can hear them transmit and hear what they are saying - but not understand it. You'll need to get the radio to your translator to actually leverage that radio. Signal loss can play into this as well - maybe you can triangulate or discover enemies passed on their radio communications.

#### Babel

Babel provides a method for a mission maker to give each side or individual players their own languages. If you can't understand a language, that persons speaking sounds garbled and reversed. People have described it sounding like pig latin. This allows for whole new levels of gameplay where you can hear people or steal their radios, but not understand them. You can also include translators who speak both languages, extending possible mission scenarios.

#### TVT Radio Setups

ACRE2 has always supported assigning different radios and frequencies to different sides, and made radios physical objects. This means that the radio's configuration is tracked with the object. If a radio is dropped, or collected off a dead player, the other player sees that radios configuration and can use it. This, combined with babel, allows for many different TVT scenarios and added levels of great gameplay.


## Creator DLC Compatibility

YARD carries ACRE2's compatibility components for several Creator DLCs. These include terrain data for signal calculations, because the terrains are published encrypted and can't be read on map load. Compatibility loads automatically with the Creator DLC.

| Creator DLC | Compatibility |
| --- | --- |
| Global Mobilization | Terrain data for signal calculations |
| S.O.G. Prairie Fire | Terrain data for signal calculations. AN/PRC-77 in the co-op campaign |
| Western Sahara | Terrain data for signal calculations |
| CSLA Iron Curtain | Terrain data for signal calculations |
| Spearhead 1944 | Terrain data for signal calculations. AN/PRC-77 in radio backpacks. Customised radio racks, including command vehicles with additional radios |

> [!NOTE]
> The Creator DLC compatibilities were contributed to ACRE2 by their authors and other contributors.
