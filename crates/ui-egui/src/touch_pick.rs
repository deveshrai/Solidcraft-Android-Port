//! SolidCraft (Android port): picking with a crosshair (port issue #2, asked for by a2nr). A
//! button at the end of the view bar turns it on; while it is on, a finger on the model aims a
//! crosshair a little above the fingertip instead of orbiting, and lifting the finger selects
//! what the crosshair is on. The button answers only a finger and the crosshair follows only a
//! finger: a mouse already picks precisely, so it works as it always has. Two fingers still pan
//! and pinch.

use egui::{Event, Painter, Pos2, Rect, Sense, Stroke, TouchPhase, pos2, vec2};

use crate::SolveApp;
use crate::navigation_bar::{Edge, Kind};
use crate::ui_kit;

/// How far (points) above the finger the crosshair sits: a fingertip covers about this much.
pub(crate) const RAISE: f32 = 60.0;
/// The gap (points) between the view bar and the button.
const GAP: f32 = 6.0;

/// Crosshair picking: whether it is on, and the finger aiming it.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Hold {
    /// The view bar's button is on.
    pub(crate) on: bool,
    /// A finger has touched the screen: the button is offered (always on Android).
    touched: bool,
    /// The last press was a finger's (a touch began with it), not a mouse button's.
    by_finger: bool,
    /// The crosshair and the finger under it, while a finger aims.
    pub(crate) aim: Option<(Pos2, Pos2)>,
    /// The finger lifted from the crosshair this frame: pick there.
    released: Option<Pos2>,
}

/// Note who pressed this frame: a finger's press comes with a touch, which may have begun a
/// frame earlier.
fn note_press(app: &mut SolveApp, ctx: &egui::Context) {
    let (pressed, finger) = ctx
        .input(|i| (i.pointer.any_pressed(), i.any_touches() || i.events.iter().any(|e| matches!(e, Event::Touch { phase: TouchPhase::Start, .. }))));
    app.viewport.touch_pick.touched |= finger;
    if pressed {
        app.viewport.touch_pick.by_finger = finger;
    }
}

/// Follow a finger on the model for one frame of [`crate::navigation::handle`], after the press
/// has chosen its navigation. `orbit_press`: this frame's press was a finger that would orbit.
/// The crosshair while a finger aims (and on the frame it lifts), which is then where the view
/// picks.
pub(crate) fn track(app: &mut SolveApp, ctx: &egui::Context, rect: Rect, orbit_press: bool, down: bool, finger: Option<Pos2>) -> Option<Pos2> {
    note_press(app, ctx);
    let hold = &mut app.viewport.touch_pick;
    hold.released = None;
    if orbit_press && hold.on && hold.by_finger {
        hold.aim = finger.map(|p| (crosshair(p, rect), p));
    }
    let (aim, at) = hold.aim?;
    if !down {
        hold.aim = None;
        hold.released = Some(aim);
        return Some(aim);
    }
    let at = finger.unwrap_or(at);
    let aim = crosshair(at, rect);
    hold.aim = Some((aim, at));
    app.viewport.primary_nav = None;
    app.viewport.orbit_pivot = None;
    app.viewport.orbit_from = None;
    Some(aim)
}

/// The crosshair for a finger at `at`: above it, kept in the view.
fn crosshair(at: Pos2, rect: Rect) -> Pos2 {
    pos2(at.x, (at.y - RAISE).max(rect.top() + 1.0))
}

/// A finger is aiming: egui's own long press (the context menu at 0.8 s) must not act on it.
pub(crate) fn holding(app: &SolveApp) -> bool {
    app.viewport.touch_pick.aim.is_some()
}

/// The finger lifted from the crosshair this frame, where it was: the view picks there.
pub(crate) fn released(app: &SolveApp) -> Option<Pos2> {
    app.viewport.touch_pick.released
}

/// Draw the crosshair: a ring with a cross whose middle is left open over what it picks, and a
/// line down to the finger.
pub(crate) fn paint(app: &SolveApp, painter: &Painter) {
    let Some((aim, at)) = app.viewport.touch_pick.aim else { return };
    let t = crate::theme::Tokens::view();
    let (ink, halo) = (t.accent, t.icons.marker_halo);
    let (ring, gap, arm) = (16.0, 5.0, 26.0);
    for (width, colour) in [(4.0, halo), (2.0, ink)] {
        painter.circle_stroke(aim, ring, Stroke::new(width, colour));
        cross(painter, aim, gap, arm, Stroke::new(width, colour));
    }
    painter.line_segment([pos2(aim.x, aim.y + arm), at], Stroke::new(1.0, ink.gamma_multiply(0.5)));
    painter.circle_filled(at, 3.0, ink.gamma_multiply(0.5));
}

/// Four arms from `gap` to `arm` out of `c`.
fn cross(painter: &Painter, c: Pos2, gap: f32, arm: f32, stroke: Stroke) {
    for (a, b) in [
        (pos2(c.x - arm, c.y), pos2(c.x - gap, c.y)),
        (pos2(c.x + gap, c.y), pos2(c.x + arm, c.y)),
        (pos2(c.x, c.y - arm), pos2(c.x, c.y - gap)),
        (pos2(c.x, c.y + gap), pos2(c.x, c.y + arm)),
    ] {
        painter.line_segment([a, b], stroke);
    }
}

/// Whether the view bar shows the button: on Android, or once a finger has touched the screen
/// (a touch screen elsewhere). A mouse alone has no use for it.
pub(crate) fn offered(app: &SolveApp) -> bool {
    cfg!(target_os = "android") || app.viewport.touch_pick.touched
}

/// Where the button sits: past the right end of the view bar `bar`, or before its left end when
/// the view `content` has no room there.
pub(crate) fn button_rect(bar: Rect, content: Rect) -> Rect {
    let side = bar.height();
    let right = Rect::from_min_size(pos2(bar.right() + GAP, bar.top()), vec2(side, side));
    if right.right() <= content.right() { right } else { Rect::from_min_size(pos2(bar.left() - GAP - side, bar.top()), vec2(side, side)) }
}

/// The view bar's crosshair button, at `rect` (see [`button_rect`]): on and off by a finger only.
pub(crate) fn button(app: &mut SolveApp, ui: &mut egui::Ui, rect: Rect, edge: Edge) {
    note_press(app, ui.ctx());
    let t = crate::theme::Tokens::get();
    let size = app.preferences.navigation_bar.size;
    ui_kit::navigation_surface(ui, rect, size, false);
    let inner = rect.shrink(ui_kit::NAV_BORDER);
    let response = ui.interact(inner, ui.id().with("touch_pick_button"), Sense::click());
    let on = app.viewport.touch_pick.on;
    let finger = app.viewport.touch_pick.by_finger;
    let pressed = response.is_pointer_button_down_on() && finger;
    let (fill, _) = ui_kit::navigation_visual(Kind::Toggle, on, false, pressed, false);
    if let Some(fill) = ui_kit::navigation_fill(&t, fill) {
        ui.painter().rect_filled(inner, ui_kit::NavigationMetrics::for_size(size).button_radius, fill);
    }
    let ink = ui_kit::navigation_ink(&t, fill);
    let (c, r) = (inner.center(), inner.height() * 0.2);
    ui.painter().circle_stroke(c, r, Stroke::new(1.5, ink));
    cross(ui.painter(), c, r * 0.4, r * 1.7, Stroke::new(1.5, ink));
    let label = app.tr("touch_pick.button").to_owned();
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, &label));
    ui_kit::navigation_tooltip(&response, rect, edge, &label, None);
    crate::scenario::publish_target("nav:touch_pick", inner);
    if response.clicked() && finger {
        let hold = &mut app.viewport.touch_pick;
        hold.on = !hold.on;
        hold.aim = None;
    }
}
