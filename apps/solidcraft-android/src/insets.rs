//! The part of the window the system covers: a desktop window's caption bar (Samsung DeX,
//! Android's desktop windowing) is drawn over the app, which Android 15+ lays out edge to edge.
//! egui keeps the app out of it as a safe-area inset, so the app's top bar (File, Settings,
//! Report a bug) sits below the caption instead of under it.

/// The top safe-area inset, in points, for a caption bar `caption_px` pixels high at
/// `pixels_per_point`, or `None` when it is the same as `last` (egui keeps the last one given).
pub fn caption_inset(caption_px: i32, pixels_per_point: f32, last: &mut Option<f32>) -> Option<f32> {
    let top = if caption_px > 0 && pixels_per_point.is_finite() && pixels_per_point > 0.0 { caption_px as f32 / pixels_per_point } else { 0.0 };
    if *last == Some(top) {
        return None;
    }
    *last = Some(top);
    Some(top)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In a DeX window the caption's height, in points, is kept clear at the top of the app;
    /// full screen (no caption) clears it again; an unchanged caption isn't sent again.
    #[test]
    fn a_desktop_windows_caption_keeps_the_app_below_it() {
        let mut last = None;
        assert_eq!(caption_inset(96, 2.0, &mut last), Some(48.0));
        assert!(caption_inset(96, 2.0, &mut last).is_none());
        assert_eq!(caption_inset(0, 2.0, &mut last), Some(0.0));
        assert!(caption_inset(0, 2.0, &mut last).is_none());
        for ppp in [0.0, -1.0, f32::NAN] {
            last = None;
            assert_eq!(caption_inset(96, ppp, &mut last), Some(0.0), "{ppp}");
        }
    }
}
