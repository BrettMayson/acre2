# Installation

YARD carries voice itself: there is no TeamSpeak or other voice program to install. The Arma server hosts voice (see [Voice Server](voice-server.md)); players just join.

YARD replaces ACRE2: don't load both.

### Download

- Download the [latest release](https://github.com/BrettMayson/yard/releases/latest) and extract the `.zip`. It contains the `@yard` folder.
- Put `@yard` in your Arma 3 folder (replacing any older `@yard`).
- Load it with the Arma 3 Launcher (`Mods → ... → Add watched local folder`, or add `@yard` as a local mod), together with [CBA A3](https://steamcommunity.com/sharedfiles/filedetails/?id=450814997). For a shortcut or server, add `@CBA_A3;@yard` to the `-mod` parameter.

A Steam Workshop item is coming.

### Microphone and speakers

YARD uses your system's default microphone and output device unless you pick others. Everything is in the in-game addon options (`Options → Addon Options → YARD`):

- **Microphone** / **Speakers / headphones**: pick a device from the list, or *System default*.
- **Direct speech activation**: *Voice activation* (default) or *Push to talk*. With push to talk, bind the *Direct Speech PTT* key under `Options → Controls → Configure Addons → YARD`. Radio, intercom, Zeus and god mode PTT keys always transmit, whatever this is set to.
- **Voice activation open / close level**: direct speech starts when your microphone gets louder than the open level, and stops a moment after it drops below the close level.
- **Noise suppression** (on by default): removes steady background noise, such as fans, hum and room noise, from your microphone before it's sent.
- **Voice activation: speech only** (on by default): voice activation only opens on sound that seems to be speech, so typing, mouse clicks and other loud noises don't transmit. Push-to-talk keys are not affected.
- **Push-to-talk release delay** (0.2 s by default): how long any push-to-talk key (direct, radio, intercom, Zeus, god mode) keeps transmitting after you let go, so the end of your last word isn't cut off.
- **Hear yourself on other radios** (on by default): while you transmit, your other radios tuned to the same channel play your own voice, and so do loudspeaker radios near you. The radio you're transmitting on doesn't.

To mute your microphone, press `Ctrl+Shift+M` (*Mute Microphone (toggle)*, rebindable under `Options → Controls → Configure Addons → YARD`). While muted nothing is transmitted, push-to-talk included, and an icon shows in the top right corner (move it with `Options → Game → Layout`). Other players see a muted icon next to your name in the Players screen.

The easiest way to set the levels is the **microphone test**: `Escape → Options → YARDMicrophone Test` (also linked from the *YARD Voice* page on the map). It asks you to stay silent, type without talking, then talk, about 5 seconds each, and suggests levels that ignore your background and keyboard noise. Nothing is transmitted while it runs. If your typing is as loud as your voice and *speech only* is off, it suggests push to talk instead.

### Connection status

The map has an **YARD Voice** page (in the diary, on the left) showing whether you're connected, your latency, packet loss each way, bandwidth, who you're hearing, and your microphone settings.

### Configuration

You can configure YARD features to your needs. YARD uses the [CBA Settings System](https://github.com/CBATeam/CBA_A3/wiki/CBA-Settings-System), accessible via the in-game configuration. This applies to [server](https://github.com/CBATeam/CBA_A3/wiki/CBA-Settings-System#server-settings), mission and client settings.

Server administrators can get available settings using the [export/import features](https://github.com/CBATeam/CBA_A3/wiki/CBA-Settings-System#export-and-import-function). The options then need to go into [CBA's userconfig or an addon](https://github.com/CBATeam/CBA_A3/wiki/CBA-Settings-System#userconfig). Server settings can be configured to override specific mission settings.


### Troubleshooting

- **WARNING: YARD IS NOT CONNECTED TO THE VOICE SERVER!**  
The server's voice port isn't reachable, or your firewall blocks it. YARD retries on its own. If it persists, let your server administrator know (the server's RPT log says why), and check that outgoing UDP to the voice server's port (default 9987) is allowed.

- **Nobody hears me / I transmit all the time**  
Run the microphone test, or switch *Direct speech activation* to *Push to talk*. Check *Microphone* is the device you talk into.

- **Radio beeps not audible on dedicated server**  
Make sure your `server.cfg` has `"b64"` whitelisted in `allowedLoadFileExtensions[]` and `allowedPreprocessFileExtensions[]` if you are using that. Let your server administrator know if you have no idea what this means.
