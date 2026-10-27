# What's new in the Android port

Each port release is an upstream SolidCraft release plus the port's own changes, named with a letter
until the next upstream release (0.24.0a, then 0.24.0b…). APKs:
[releases](https://github.com/deveshrai/Solidcraft-Android-Port/releases). What is still wrong or
missing: [android-known-issues.md](android-known-issues.md).

- [0.24.0a](#0240a): upstream 0.24.0, the MCP server, thread fixes
- [0.23.0b](#0230b): finger-scrolled menus
- [0.23.0a](#0230a): gears, upstream 0.23.0
- [0.22.0a](#0220a): helical B-rep threads, upstream 0.22.0
- [0.21.0a](#0210a): the first lettered release, upstream 0.21.0

## 0.24.0a

**Upstream 0.24.0** (and 0.23.1), merged: among much else, a Weldments tab, the 3D sketch arc, personal Hole
presets, the navigation bar's view commands, the Clay themes, left- and right-handed threads and holes,
perspective while orbiting, and design format 39.

**New in the port:**

- **Settings › MCP server.** One switch, and Claude Code (or any MCP client) in Termux on the device, or on
  a computer on the same Wi-Fi, models in the app while you watch: the page shows the server's address and
  one command to paste, each with Copy; Advanced has the port, a `claude mcp add` command, the access
  token and starting with the app. Every request needs the token; **This device** can't be reached from
  the network. All of upstream's MCP tools work, the port's gears and helical threads too
  ([android-mcp.md](android-mcp.md)).

  ![Settings › MCP server on a Galaxy Z Fold](screenshots/android/mcp-settings-fold.gif)

- **Tested live on a Galaxy Z Fold over Wi-Fi**: a cassette housing with five threaded M2 screws, built and
  checked to fit through the server, 121 operations in 105 s, no failures
  ([android-mcp-live-test.md](android-mcp-live-test.md)).
- **Helical threads take upstream's Thread direction.** Upstream 0.24.0 added right- and left-handed
  threads; the port's helical B-rep threads now use that row instead of their own Left-handed box. Designs
  saved by earlier port releases with a left-handed helical thread open unchanged; `left_handed: true`
  in scripts still means left.
- **Copy reaches Android's clipboard**, everywhere in the app.

**Fixed:**

- A helical thread tapped into a boss under a hollow shell was refused ("the groove removes … mm³, not
  …"), although it was right.
- An internal ISO thread with a clearance in too small a hole was made too shallow for its bolt, silently;
  it now says which drill works ("drill 1.85 mm, or make the clearance smaller").
- Back with Settings open sent the app to the background instead of closing Settings.

**Design files:** format 40 (upstream's 39 plus the helical threads). Upstream releases still can't open
port designs; export STEP to share.

## 0.23.0b

- **Ribbon menus scroll with a finger.** Upstream 0.23.0 made menu rows draggable onto the ribbon, so a
  finger could no longer scroll a long menu, and on a foldable **Model › Create** stopped at Web: Gear and
  the rest were out of reach. A finger now scrolls; a mouse still pins.
- Tested on a Galaxy Z Fold: gears, helical threads, STEP and STL, Open and Save As; a known-issues page.

## 0.23.0a

- **Gears**: ten involute types with bore, DIN 6885 keyway and hub, previewed and editable after
  ([android-gears.md](android-gears.md)); smooth twisted and scaled sweeps.
- **Upstream 0.23.0**, merged.
- **Save As** names the tab after what was typed in the picker.

## 0.22.0a

- **Helical B-rep threads**: exact threads, custom pitch on any diameter, clearance, offsets
  ([android-threads.md](android-threads.md)).
- **Upstream 0.22.0**, merged, with its Render workspace working on the device.

## 0.21.0a

- **Upstream 0.21.0**, merged; the first release named with a letter.
- Back closes what is open; a user guide for touch, mouse and keyboard, files and foldables
  ([android-touch.md](android-touch.md)).
