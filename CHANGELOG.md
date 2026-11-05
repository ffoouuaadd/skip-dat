# Changelog

## 1.4.1

### Fixed
- **Spotify jumps straight to the song you pick**, like an ender pearl, without skipping through the songs in between. It starts the song inside the playlist or album you're playing, which then carries on as usual. Spotify can't start some lists (Liked Songs, artists, radio) at a particular song. For those, it plays your song followed by the songs shown after it, in the same order. This needs the optional Spotify connection in Settings. With shuffle on, Spotify may pick a new shuffle order after the jump.
- **The history button on the carousel could freeze Skip Dat**, so it's gone. "What was I playing?" is in Settings and the tray menu, and opening it from there no longer locks the app up.
- **The song before the current one is now the one you really played.** For Spotify it came from Spotify's "recently played", which leaves out anything heard for less than 30 seconds, so it often showed an older song. Picking an earlier Spotify song now goes straight to it too. Songs Skip Dat only skipped past on the way no longer count as played.

### New
- **Back to …** under the play/pause button shows the song you were playing before your last skip or jump. One click takes you back, and the next click returns you.

## 1.4.0

### Fixed
- **Skipping backwards could crash Skip Dat.** One failed step could leave a shared lock "poisoned", and the next thing to touch it closed the whole app. Locks now recover, and the steps that failed are fixed too. Skip Dat no longer presses Previous on players that have no previous song, and the first song in a queue stays put. YouTube "back" now opens the video played before instead of the browser's back page. Jumping in Spotify no longer replaces the playlist or album it's playing from. Any crash is now written to `skipdat.log`.
- **Jumping ahead no longer plays the songs in between.** Where the player can jump straight to a song (YouTube Music, YouTube playlists and mixes, SoundCloud), Skip Dat jumps there. Elsewhere it mutes only that app in Windows' volume mixer, skips, and unmutes once your song starts. It also unmutes after a crash.
- **Shuffle keeps its order.** After a jump, the queue is the same as before, minus the songs you passed. SoundCloud with shuffle on uses muted skips, because its direct jump reshuffles. If any site reorders its queue after a jump, Skip Dat notices and uses muted skips there from then on.

### New
- **Styles**: Wheel, Stack, Cover flow, Orbit (a tilted ring of songs round your cursor) and Compact (titles only).
- **Size**: Small, Medium and Large. The default, Medium, is about 15% smaller than 1.3; Large is 1.3's size.
- **Depth**: real blur and fading for songs away from the middle, each with its own slider.
- **Skins**: Liquid Glass, Lime Pod, LCD Night, Brushed Chrome and Pixel Party. You can also make your own from a folder with a `skin.json` and pictures. Skip Dat reloads them as you save. See SKINS.md.
- **What was I playing?**: the songs you listened to, with app, time and cover. Click one to play it again. It's kept only on your PC and has a Clear button.
- **Performance mode**: no blur, glass, shadows, animations or album art. In testing the open carousel used about 80% less processor time.
- **Buy me a coffee** button in Settings.

### Changed
- Skip Dat releases its graphics memory whenever the carousel is closed, so it uses less memory while idle.
- The browser companion is now 1.4.0. Reload it on your browser's extensions page after updating.
