//! When the on-screen keyboard comes up. The app gives a field the focus as a window or tool
//! opens (Add parameter's name, a rectangle's dimension boxes) so a hardware keyboard can type
//! at once; on a touch screen that also brought up the on-screen keyboard, over the dialog or
//! the sketch (issue #6). Here the keyboard comes up only when a finger taps a field: a field
//! focused any other way keeps the focus (a hardware keyboard still types into it) without the
//! on-screen keyboard. Once it is up it stays up while a field keeps the focus, so Enter moving
//! to the next field doesn't close it.

/// Whether the on-screen keyboard is allowed up, carried from frame to frame.
#[derive(Default)]
pub struct KeyboardGate {
    up: bool,
}

impl KeyboardGate {
    /// After the app's UI: let egui show the keyboard for the focused field only after a tap
    /// on that field.
    pub fn frame(&mut self, ctx: &egui::Context) {
        if ctx.output(|o| o.ime.is_none()) {
            self.up = false;
            return;
        }
        let tapped = tapped_field(ctx);
        self.up |= tapped;
        ctx.output_mut(|o| {
            if !self.up {
                o.ime = None;
            } else if let Some(ime) = o.ime.as_mut()
                && tapped
            {
                // Shown again after the keyboard's own ⌄ hid it: egui asks for it only when a
                // field gains the focus.
                ime.should_interrupt_composition = true;
            }
        });
    }
}

/// A tap on the focused field this frame: a click (a press let go where it began) on it, where it
/// was before this frame. A field that appeared under the finger this frame (a dimension box
/// where a rectangle starts) wasn't tapped.
fn tapped_field(ctx: &egui::Context) -> bool {
    let Some(focused) = ctx.memory(|m| m.focused()) else {
        return false;
    };
    let Some(before) = ctx.read_response(focused) else {
        return false;
    };
    ctx.input(|i| i.pointer.primary_clicked() && i.pointer.interact_pos().is_some_and(|p| before.rect.contains(p)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIELD: egui::Rect = egui::Rect::from_min_max(egui::pos2(10.0, 10.0), egui::pos2(110.0, 30.0));

    struct Window {
        ctx: egui::Context,
        gate: KeyboardGate,
        text: String,
        time: f64,
    }

    impl Window {
        fn new() -> Self {
            Self { ctx: egui::Context::default(), gate: KeyboardGate::default(), text: String::new(), time: 0.0 }
        }

        /// One frame of a window with one field; `focus` gives it the focus as the app does when a
        /// dialog opens. Whether egui asks for the on-screen keyboard.
        fn frame(&mut self, events: Vec<egui::Event>, focus: bool) -> bool {
            self.time += 0.1;
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 300.0))),
                time: Some(self.time),
                events,
                ..Default::default()
            };
            let (gate, text) = (&mut self.gate, &mut self.text);
            let mut out = self.ctx.run_ui(input, |ui| {
                let field = ui.put(FIELD, egui::TextEdit::singleline(text));
                if focus {
                    field.request_focus();
                }
                gate.frame(ui.ctx());
            });
            out.textures_delta.clear();
            out.platform_output.ime.is_some()
        }
    }

    fn touch(pos: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE },
        ]
    }

    /// A field focused as its window opens has the focus without the on-screen keyboard; a tap on
    /// it brings the keyboard up, and it stays up while the field keeps the focus.
    #[test]
    fn the_keyboard_comes_up_for_a_tapped_field_not_a_window_opening() {
        let mut w = Window::new();
        w.frame(Vec::new(), false);
        assert!(!w.frame(Vec::new(), true), "focused as the window opens");
        assert!(w.ctx.memory(|m| m.focused().is_some()), "the field keeps the focus for a hardware keyboard");
        assert!(!w.frame(Vec::new(), false));
        let on = FIELD.center();
        assert!(!w.frame(touch(on, true), false));
        assert!(w.frame(touch(on, false), false), "tapped");
        assert!(w.frame(Vec::new(), false), "stays up");
        assert!(w.frame(vec![egui::Event::Text("4".into())], false));
        assert_eq!(w.text, "4");
    }

    /// A field that isn't focused comes up with the keyboard on the first tap; a tap elsewhere
    /// takes the focus and the keyboard away; a tap that begins off the field (a drag onto it)
    /// isn't a tap on it.
    #[test]
    fn a_tap_off_the_field_is_not_a_tap_on_it() {
        let mut w = Window::new();
        w.frame(Vec::new(), false);
        let on = FIELD.center();
        w.frame(touch(on, true), false);
        assert!(w.frame(touch(on, false), false));
        let off = egui::pos2(300.0, 200.0);
        w.frame(touch(off, true), false);
        assert!(!w.frame(touch(off, false), false));
        w.frame(Vec::new(), true);
        w.frame(touch(off, true), false);
        assert!(!w.frame(
            vec![
                egui::Event::PointerMoved(on),
                egui::Event::PointerButton { pos: on, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE }
            ],
            true
        ));
    }
}
