# Using SolidCraft on Android

A guide to the Android app with touch, a mouse and keyboard, and files. The app is the full
desktop SolidCraft; this covers what is different on a phone or tablet. Building and how the port
works: [android.md](android.md).

Screenshots are from a Galaxy Z Fold (inner screen, landscape, unless noted) running
SolidCraft 0.21.0a.

- [Touch](#touch)
- [Modelling by touch](#modelling-by-touch)
- [Mouse and keyboard](#mouse-and-keyboard)
- [Typing values](#typing-values)
- [Files](#files)
- [Back, Home and rotation](#back-home-and-rotation)
- [Phones and foldables](#phones-and-foldables)
- [Limits](#limits)

## Touch

| Gesture | Does |
|---------|------|
| Tap | Select what is under the finger (a face, an edge, a body, a profile); press any button or menu |
| One-finger drag on the model | Orbit the view (over bodies too; a tap still selects) |
| Two-finger drag | Pan the view with the fingers |
| Pinch | Zoom about the point between the fingers |
| Drag a dialog's arrow | Change its value (Extrude distance, Fillet radius…) |
| Home button by the axis triad | Return to the home view |

Once two fingers are down the touch stays a pan or pinch until every finger lifts, so lifting
one finger first does not start an orbit. While a sketch is being edited or a sketch tool is on,
a one-finger drag draws or drags geometry as the mouse does, not orbit; the view bar at the bottom
(orbit, pan, zoom buttons) still moves the view then.

| One finger: orbit | Pinch: zoom | Two fingers: pan |
|---|---|---|
| ![One finger orbiting the view](screenshots/android/05-touch-orbit.jpg) | ![A pinch zooming in](screenshots/android/06-touch-pinch.jpg) | ![Two fingers panning](screenshots/android/07-touch-pan.jpg) |

## Modelling by touch

A block with a rounded edge, by finger only:

1. **New Sketch**, then tap a plane (here XY at the origin). The view turns to look straight at it.

   ![A new sketch on the XY plane](screenshots/android/01-sketch-on-plane.jpg)

2. **Rectangle**, then tap two opposite corners. Horizontal and vertical constraints are added
   as you go. **Finish Sketch** (bottom right) when done.

   ![A rectangle drawn with two taps](screenshots/android/02-rectangle-two-taps.jpg)

3. **Extrude**. The profile you just drew is already picked ("1 selected"); tapping it again would
   unpick it. Drag the arrow, or type a distance, and **OK**.

   ![The Extrude dialog](screenshots/android/03-extrude-dialog.jpg)

   ![The extruded block](screenshots/android/04-extruded-block.jpg)

4. **Fillet**, tap an edge, and drag the radius arrow with your finger (anywhere along it, not
   just the tip). **OK**.

   ![A fillet radius dragged by finger](screenshots/android/08-fillet-finger-drag.jpg)

   ![The filleted block](screenshots/android/09-filleted.jpg)

Commands not on the ribbon are a search away: tap **Search commands** (top right) and type.

## Mouse and keyboard

A USB or Bluetooth mouse and keyboard work as on the desktop: hover highlights what a click will
pick, the navigation preset's buttons move the view (default: right drag orbits, middle drag pans,
Shift+middle orbits), the wheel zooms to the cursor, right-click opens the context menu, and
keyboard shortcuts work as on the desktop (tested: Ctrl+Z, Ctrl+Y, Ctrl+S, Ctrl+O, E for Extrude,
S for search, Esc).

| Mouse: an edge selected | Keyboard: command search |
|---|---|
| ![An edge selected with a mouse](screenshots/android/10-mouse-select-edge.jpg) | ![Command search typed on a keyboard](screenshots/android/11-command-search.jpg) |

## Typing values

Tap a field (a dialog's value, command search, a rename) and Android's on-screen keyboard comes
up; type, and tap outside or press Back to put it away. It covers the lower part of the screen,
so a dialog low on the screen may sit under it; a hardware keyboard avoids that.

![Typing on the on-screen keyboard](screenshots/android/12-on-screen-keyboard.jpg)

## Files

**File** opens the menu: Open, Insert (STEP, mesh, design), Save, Save As, and Export (STL, 3MF,
STEP, OBJ, DXF of the view, URDF, JSON). Each opens Android's file picker, so files can come from
and go to the device, an SD card, Google Drive or any storage app.

![The File menu](screenshots/android/13-file-menu.jpg)

- **Save** (the disk icon, or Ctrl+S) on a design opened from a file writes back to that file,
  wherever it is. The app remembers this across restarts.
- **Export STL / 3MF** asks which bodies first (tap them in the view or the Browser).
- **Insert STEP / Insert Mesh** brings a part in as a component next to what is there.

![A STEP part and an STL mesh inserted](screenshots/android/14-step-and-stl-inserted.jpg)

## Back, Home and rotation

- **Back** (the gesture or button) closes what is open: a dialog, a menu, a sketch tool, command
  search, the on-screen keyboard. With nothing open it leaves the app as Home does; your designs
  stay open.
- **Home** and coming back: the app is as you left it.
- **Rotation**: the layout follows; portrait works, with a narrower ribbon.

![The app in portrait](screenshots/android/15-portrait.jpg)

## Phones and foldables

On a narrow screen (a phone, a foldable's cover screen) the ribbon folds its panels into menus:
**Create ▾**, **Modify ▾**, **Arrange ▾**, with the full list of commands in each. Folding or
unfolding keeps the app and the design.

| Cover screen | Create menu |
|---|---|
| ![The app on a cover screen](screenshots/android/16-cover-screen.jpg) | ![The Create menu on the cover screen](screenshots/android/17-cover-create-menu.jpg) |

## Limits

- **Modelled threads** with **Geometry: Faceted** cut a faceted groove (flat triangles to within
  0.5% of the pitch), as on the desktop; the threaded body's curved faces become facets too. On
  the Fold an M20 × 20 mm thread took under a second. **Helical B-rep** threads keep the body exact,
  and **Custom** threads take any diameter and a clearance: see [android-threads.md](android-threads.md).
  A cosmetic thread (Modeled off) keeps the body exact.

  ![A modelled thread close up](screenshots/android/18-modelled-thread.jpg)

- Autosave recovery is not available on Android yet: save your work.
- Ribbon labels are shortened at high screen densities ("Cham…" for Chamfer); the tooltip and
  command search have the full names.
- Other apps can't open files in SolidCraft ("Open with") yet: use File › Open.
