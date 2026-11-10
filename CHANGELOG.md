# Changelog

## 1.5.2

### New
- **Settings floats.** The window has no frame or background any more: only its glass panels show, over your desktop. Drag it by the top card, which has its own minimize and close buttons. Clicks in the empty space between the panels reach whatever is behind.
- **Look opens in its own panel**, like Skin. Settings shows your style and size in one row; **Change** (or Ctrl+L) opens a glass panel with Style, Wheel direction, Size and Depth. Only one panel is open at a time.
- **Liquid Glass in your own colour.** A slider under Liquid Glass in the skin panel tints the glass any colour. It stays see-through, with its blur, light and depth; **Original** puts the clear glass back.
- **Freaky Random is easier to follow.** The next songs curl round under the song in front and earlier ones over the top, so the next song is always the closest card. Each notch glides every card one place along a curve, the song in front glows softly, and pointing at a card lifts it and shows its name. Every card stays on screen, wherever the wheel opens.
- **Claude-ish, a new skin:** warm cream paper, a serif title, terracotta buttons and a spark that slowly turns. An unofficial fan skin.
- **Bare Knuckle goes further into its basement-fight mood:** green fluorescent light and a swinging orange bulb, a bloodied hand wrap, a typed rules note and a bar of pink soap on the card, and a flicker of old film every time the song changes.

### Fixed
- **Skins you add yourself show their pictures, textures and fonts.** Only their colours applied before: the app couldn't find any file in the skins folder.
- Ctrl+K and Ctrl+L in Settings leave a text box you're typing in alone.

## 1.5.1

### New
- **Volume on the wheel.** Scroll over the play/pause button to turn the playing app up or down, 5% a notch. Desktop apps change only their own slider in Windows' volume mixer. YouTube and YouTube Music change only that tab's player, not the whole browser. SoundCloud's volume only moves with its own slider, and Skip Dat says so.
- **Jump within a song.** Scroll over the song playing to go forward or back 10 seconds a notch. This works where the player allows it: Spotify (through Windows, or the Spotify connection), YouTube, YouTube Music and most desktop apps. Skip Dat tells you when an app doesn't allow it.
- **Song buttons under the play controls:**
  - **Like** adds the song to Spotify's Liked Songs, or presses Like on YouTube, YouTube Music and SoundCloud. It never unlikes: a song that's already liked stays liked, and the heart shows it.
  - **Copy link** copies the open.spotify.com link of a Spotify song, or the page of the song playing in your browser.
  - **Play next** puts a Spotify song straight after the one playing, without restarting anything.
  - **Continue on Spotify** finds the song from YouTube, YouTube Music, SoundCloud or another app on Spotify, starts it at the same moment, and pauses the original only once Spotify is playing. If Spotify can't find or start it, nothing is paused.
  - A button the playing app can't do is dimmed, and clicking it says why. A double click counts once.
- **Freaky Random, a new carousel style.** The song in front is big and sharp, and the rest float around it at random depths, blurred by distance. Every time it opens the songs land somewhere new, bursting out from behind the front card; closing flies them back in.
- **Settings is liquid glass.** On Windows 11 the window is see-through frosted glass: what's behind it shows, blurred, and the panels keep every word easy to read.
- **The skin picker opens when you want it.** Settings shows your skin in one row; **Change** (or Ctrl+K) opens a glass panel with every skin, and Esc, the X or a click outside closes it.
- **Skins can decorate the whole player.** Up to eight pictures around the cards, the play button or the wheel, with gentle animations, plus drifting particles. A picture can swing from where it hangs, move at its own speed, and play again when the song changes. See SKINS.md.
- **New skins:**
  - **Bubble Gum:** an 8-bit gum wrapper whose bubble slowly blows up and pops, and pops straight away when you skip.
  - **Quicksilver:** liquid mercury along the card's edge, with a drip that gathers and falls.
  - **Bare Knuckle:** a damp basement wall: concrete, duct tape, a bar of pink soap and a swinging bare bulb.
- **Every built-in skin got finer art and calmer motion.** Pixel Party's torch is animated again (and smoother), Lime Pod's antenna sways from its base, LCD Night's battery charges, Brushed Chrome has real screws and a breathing LED, and Liquid Glass catches a glint of light.
- With Windows' "Show animations" off, skin pictures hold still.

### Fixed
- **Volume for Spotify from the Microsoft Store.** Its sound comes from helper processes, which Windows' volume mixer now finds by Spotify's package name.

### Note
- **Spotify users: press Reconnect once** under Spotify in Settings. Like and Continue on Spotify need permission to read and save your Liked Songs.

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
