# Using SolidCraft on Android

A guide to the Android app with touch, a mouse and keyboard, and files. The app is the full
desktop SolidCraft; this covers what is different on a phone or tablet. Building and how the port
works: [android.md](android.md).

Screenshots are from a Galaxy Z Fold (inner screen, landscape, unless noted) running
SolidCraft 0.21.0a; the Threads and Render screenshots are from 0.22.0a, the Gears ones from a
Galaxy Tab running 0.22.0 with the gears.

- [Touch](#touch)
- [Modelling by touch](#modelling-by-touch)
- [Threads](#threads)
- [Gears](#gears)
- [Rendering](#rendering)
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
| Drag a menu or dialog up or down | Scroll it: a ribbon menu taller than the screen (Model › Create runs past the bottom on a foldable) or a tall dialog. A mouse drags a menu row onto the ribbon to pin it instead |
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

## Threads

**Create ▾ › Thread**, then tap the cylindrical face (a shaft, or the inside of a hole). The size
that fits is filled in (M20 on a 20 mm shaft). Off **Modeled** it is a cosmetic thread, drawn
but not cut; with **Modeled** on, **Geometry** chooses how the groove is made:

- **Faceted**: the original method, a groove of flat triangles; the body's curved faces become
  facets too.
- **Helical B-rep** (this port): a few exact, smooth helical faces; the rest of the body stays
  exact, so fillets, a second thread and STEP export all keep working. It starts and ends half a
  pitch in from the face's ends. An M20 × 20 mm took under two seconds on the Fold.

![A helical B-rep M20 thread](screenshots/android/19-helical-thread.jpg)

**Type: Custom** threads any shaft or hole as it is, whatever its diameter: type a **Pitch** (and
optionally a **Thread depth**, **Thread angle** or **Left-handed**). For parts that screw
together, **Clearance** moves the flanks and root into the material, typically 0.1–0.2 mm for a
3D print. Tap OK rather than pressing Enter in a field: Enter applies the dialog straight away.
To change a thread later, double-tap it in the **History** bar.

| Custom thread, clearance being set | The result |
|---|---|
| ![Editing a custom thread's clearance](screenshots/android/20-custom-thread-clearance.jpg) | ![A custom thread with clearance](screenshots/android/21-custom-thread-result.jpg) |

Everything about threads (each field, the groove's shape, the readouts, refusals and the
`solid.thread` script parameters): [android-threads.md](android-threads.md).

A design with a helical thread is saved in a newer file format than upstream SolidCraft's
release reads; desktop SolidCraft asks to be updated instead of opening it. Export STEP to take
such a part to other software.

## Gears

**Create ▾ › Gear** (or search *gear*) makes an involute gear as its own component: spur,
helical, herringbone, internal, rack, bevel, miter, worm, worm wheel or crossed helical (this
port). Pick a **Type**, a **Module** and **Teeth**; the view previews every change, and the
readouts show the diameters, the mate's centre distance and any undercut. **Bore and hub** adds a
bore, a DIN 6885 keyway and a hub. **OK** inserts it at the origin, axis up Z.

| The Gear dialog | Bore, hub and keyway |
|---|---|
| ![The Gear dialog and its readouts](screenshots/android/gears-tab-02-dialog.jpg) | ![A helical gear's bore, hub and keyway previewed](screenshots/android/gears-tab-03-bore-hub-keyway.jpg) |

To change a gear, double-tap it in the browser, or long-press it and choose **Edit Gear**; OK
rebuilds it in place, in one undo step. The values are saved with the design.

![The ten gear types](screenshots/android/gears-tab-08-all-types.jpg)

Everything about gears (each type, making pairs that mesh, the readouts and warnings, the
`gear.create` parameters and how it is built): [android-gears.md](android-gears.md).

## Rendering

**Workspace ▾ › Render** (top left) opens SolidCraft's Render workspace: studio lighting,
materials and image export, with a live preview. **Start Render** makes the image and **Export
completed image** saves it through the file picker. **Return to Design** goes back.

![The Render workspace](screenshots/android/22-render-workspace.jpg)

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

- **Open** names the tab after the design's own name, the one it was saved under. A design saved by
  0.22.0a may carry "Untitled (n)" from that release's Save As; **Save As** it once to rename it.
- On Samsung devices the window handle sits at the top centre, over the tab strip, and a tap there
  can open Samsung's window menu instead of the **+** tab button. **Ctrl+N** or **File › New
  Design** always opens a new tab.

![A STEP bracket inserted next to a helical M20, on a Galaxy Z Fold](screenshots/android/fold-10-step-inserted.jpg)

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

- **Faceted threads** (Geometry: Faceted) cut a groove of flat triangles (within 0.5% of the
  pitch), as on the desktop, and the threaded body's curved faces become facets too; on the Fold
  an M20 × 20 mm took under a second. Choose **Helical B-rep** ([Threads](#threads)) to keep the
  body exact.

  ![A modelled thread close up](screenshots/android/18-modelled-thread.jpg)

- Autosave recovery is not available on Android yet: save your work.
- Ribbon labels are shortened at high screen densities ("Cham…" for Chamfer); the tooltip and
  command search have the full names.
- Other apps can't open files in SolidCraft ("Open with") yet: use File › Open.
