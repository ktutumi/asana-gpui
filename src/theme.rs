use gpui_kit::{
    component::{Theme, ThemeMode},
    *,
};

pub fn apply(light: bool, cx: &mut App) {
    Theme::change(
        if light {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        },
        None,
        cx,
    );
    let theme = Theme::global_mut(cx);
    theme.font_family = if cfg!(target_os = "macos") {
        ".AppleSystemUIFont"
    } else {
        ".SystemUIFont"
    }
    .into();
    theme.font_size = px(14.);
    // Product palette: mirrors Asana's quiet Work surfaces, shared by every screen.
    if light {
        theme.colors.background = rgb(0xffffff).into();
        theme.colors.foreground = rgb(0x292a2d).into();
        theme.colors.sidebar = rgb(0xf8f8f9).into();
        theme.colors.muted = rgb(0xf2f3f5).into();
        theme.colors.border = rgb(0xe4e5e8).into();
        theme.colors.primary = rgb(0x546bd8).into();
    } else {
        theme.colors.background = rgb(0x1e1f21).into();
        theme.colors.foreground = rgb(0xf5f4f3).into();
        theme.colors.sidebar = rgb(0x252628).into();
        theme.colors.muted = rgb(0x303133).into();
        theme.colors.border = rgb(0x393a3d).into();
        theme.colors.primary = rgb(0xa3afff).into();
        theme.colors.primary_foreground = rgb(0x202434).into();
        theme.colors.muted_foreground = rgb(0xa2a3a6).into();
    }
    theme.colors.secondary = theme.colors.muted;
    theme.colors.secondary_foreground = theme.colors.foreground;
    theme.colors.sidebar_foreground = theme.colors.foreground;
    theme.colors.sidebar_accent = theme.colors.muted;
    Theme::sync_base(cx);
}
