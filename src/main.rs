mod api;
mod app;
mod model;
mod oauth;
mod preferences;
mod theme;

use gpui_kit::{component::Root, *};

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx| {
            gpui_kit::init(cx);
            theme::apply(false, cx);
            cx.bind_keys([
                KeyBinding::new("secondary-q", app::Quit, None),
                KeyBinding::new("secondary-r", app::Refresh, Some("AsanaWorkspace")),
                KeyBinding::new("secondary-k", app::SearchTasks, Some("AsanaWorkspace")),
                KeyBinding::new("secondary-n", app::NewTask, Some("AsanaWorkspace")),
                KeyBinding::new("secondary-s", app::SaveTask, Some("AsanaWorkspace")),
                KeyBinding::new("escape", app::CloseDetail, Some("AsanaWorkspace")),
            ]);
            // Window dimensions are a native platform boundary, not content spacing.
            let options = WindowOptions {
                app_id: Some("jp.ktutumi.asana-gpui".into()),
                window_bounds: Some(WindowBounds::centered(size(px(1440.), px(900.)), cx)),
                window_min_size: Some(size(px(1000.), px(680.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Asana GPUI".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            cx.activate(true);
            cx.spawn(async move |cx| {
                cx.open_window(options, |window, cx| {
                    let view = cx.new(|cx| app::WorkspaceApp::new(window, cx));
                    let weak = view.downgrade();
                    window.on_window_should_close(cx, move |_, cx| {
                        weak.update(cx, |this, cx| this.quit(cx)).is_err()
                    });
                    cx.new(|cx| Root::new(view, window, cx))
                })
                .expect("Could not open the Asana GPUI window");
            })
            .detach();
        });
}
