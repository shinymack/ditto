use gpui_kit::component::Theme;
use gpui_kit::gpui::App;
use std::borrow::Cow;

pub const UI_FONT: &str = "Noto Sans";
pub const MONO_FONT: &str = "JetBrains Mono";

/// Register the bundled static faces once at startup, before any inputs are created.
///
/// Unmodified upstream files, with their OFL licenses in `assets/fonts`:
/// - notofonts/noto-fonts, ffebf8c1ee449e544955a7e813c54f9b73848eac,
///   hinted/ttf/NotoSans/NotoSans-Regular.ttf
/// - JetBrains/JetBrainsMono, v2.304, fonts/ttf/JetBrainsMono-Regular.ttf
pub fn init(cx: &mut App) {
    cx.text_system()
        .add_fonts(vec![
            Cow::Borrowed(include_bytes!("../assets/fonts/NotoSans-Regular.ttf")),
            Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf")),
        ])
        .expect("Failed to register bundled fonts");

    let theme = Theme::global_mut(cx);
    theme.font_family = UI_FONT.into();
    theme.mono_font_family = MONO_FONT.into();
    Theme::sync_base(cx);
}
