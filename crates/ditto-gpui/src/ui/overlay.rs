use base64::prelude::*;
use ditto_core::db::ClipboardItem;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::slider::{Slider, SliderEvent, SliderState, SliderValue};
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::*;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::icons::{ICON_CLOSE, ICON_IMAGE, ICON_SEARCH, ICON_SETTINGS, LOGO_PNG};
use super::theme::ThemeColors;
use crate::search::{format_size, get_relative_time_at, get_simple_hash};
use crate::state::AppState;
#[derive(Clone)]
pub struct CardItemView {
    pub id: i64,
    pub is_image: bool,
    pub preview_text: String,
    pub relative_time: String,
}

pub struct DittoOverlayView {
    pub state: AppState,
    pub query: String,
    pub selected_index: usize,
    pub filtered_items: Vec<ClipboardItem>,
    pub card_views: Vec<CardItemView>,
    pub search_input: Entity<InputState>,
    pub opacity_slider: Entity<SliderState>,
    pub history_limit_input: Entity<InputState>,
    pub focus_handle: FocusHandle,
    pub list_scroll_handle: ScrollHandle,
    pub settings_scroll_handle: ScrollHandle,
    pub in_settings: bool,
    pub theme_colors: ThemeColors,
    pub shown_at: Instant,
    pub snapshot_time: chrono::DateTime<chrono::Utc>,
    pub cached_preview_image: Option<(i64, Arc<Image>)>,
    pub last_nav_time: Instant,
    _subscriptions: Vec<Subscription>,
}
fn build_card_views(
    items: &[ClipboardItem],
    snapshot_time: chrono::DateTime<chrono::Utc>,
) -> Vec<CardItemView> {
    items
        .iter()
        .map(|item| {
            let is_img = item.content.starts_with("data:image/png;base64,");
            let preview_text = if is_img {
                "Image Clip".to_string()
            } else {
                let first_line = item.content.lines().next().unwrap_or(&item.content);
                if first_line.chars().count() > 32 {
                    format!("{}...", first_line.chars().take(32).collect::<String>())
                } else {
                    first_line.to_string()
                }
            };
            let relative_time = get_relative_time_at(&item.created_at, snapshot_time);
            CardItemView {
                id: item.id,
                is_image: is_img,
                preview_text,
                relative_time,
            }
        })
        .collect()
}

impl DittoOverlayView {
    pub fn new(state: AppState, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let config = state.config.read().clone();
        let theme_colors = ThemeColors::from_config(&config);
        let focus_handle = cx.focus_handle();
        let list_scroll_handle = ScrollHandle::new();
        let settings_scroll_handle = ScrollHandle::new();
        let shown_at = Instant::now();

        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Browse clipboard history..."));

        let initial_items = state.search("", config.max_items);

        let sub_input = cx.subscribe_in(
            &search_input,
            window,
            |this, state, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    let val = state.read(cx).value();
                    this.query = val.to_string();
                    this.update_filter();
                    this.selected_index = 0;
                    this.list_scroll_handle.scroll_to_item(0);
                    cx.notify();
                }
            },
        );

        // Concrete Opacity Slider (matches Tauri <input type="range" min="20" max="100">)
        let opacity_slider = cx.new(|_| {
            SliderState::new()
                .min(20.0)
                .max(100.0)
                .step(1.0)
                .default_value(config.opacity as f32)
        });

        let sub_opacity = cx.subscribe(&opacity_slider, |this, _, event: &SliderEvent, cx| {
            if let SliderEvent::Change(SliderValue::Single(single)) = event {
                let pct = single.round() as usize;
                if this.state.config.read().opacity != pct {
                    let mut cfg = this.state.config.read().clone();
                    cfg.opacity = pct;
                    this.theme_colors = ThemeColors::from_config(&cfg);
                    let _ = this.state.save_config(cfg);
                    cx.notify();
                }
            }
        });

        // Concrete History Limit Text Input (matches Tauri <input type="number" min="1" max="5000">)
        let history_limit_input =
            cx.new(|cx| InputState::new(window, cx).default_value(config.max_items.to_string()));

        let sub_limit = cx.subscribe_in(
            &history_limit_input,
            window,
            |this, state, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    let val_str = state.read(cx).value();
                    if let Ok(num) = val_str.trim().parse::<usize>() {
                        if num >= 1 && num <= 5000 {
                            let mut cfg = this.state.config.read().clone();
                            cfg.max_items = num;
                            let _ = this.state.save_config(cfg);
                            this.update_filter();
                            cx.notify();
                        }
                    }
                }
            },
        );

        // Focus search input initially
        search_input.update(cx, |input, cx| {
            input.focus(window, cx);
        });

        // Native blur-to-dismiss observer (matches Tauri 1:1)
        let sub_activation = cx.observe_window_activation(window, move |this, window, cx| {
            if !window.is_window_active() {
                let persistent = this.state.config.read().persistent_window;
                let elapsed = this.shown_at.elapsed();
                if !persistent && elapsed >= Duration::from_millis(400) {
                    this.hide_overlay(window, cx);
                }
            }
        });

        let snapshot_time = chrono::Utc::now();
        let card_views = build_card_views(&initial_items, snapshot_time);

        Self {
            state,
            query: String::new(),
            selected_index: 0,
            filtered_items: initial_items,
            card_views,
            search_input,
            opacity_slider,
            history_limit_input,
            focus_handle,
            list_scroll_handle,
            settings_scroll_handle,
            in_settings: false,
            theme_colors,
            shown_at,
            snapshot_time,
            cached_preview_image: None,
            last_nav_time: Instant::now(),
            _subscriptions: vec![sub_input, sub_activation, sub_opacity, sub_limit],
        }
    }

    pub fn update_filter(&mut self) {
        self.snapshot_time = chrono::Utc::now();
        let max_items = self.state.config.read().max_items;
        self.filtered_items = self.state.search(&self.query, max_items);
        if self.selected_index >= self.filtered_items.len() {
            self.selected_index = self.filtered_items.len().saturating_sub(1);
        }
        self.card_views = build_card_views(&self.filtered_items, self.snapshot_time);
    }

    pub fn select_next(&mut self, cx: &mut Context<Self>) {
        if !self.filtered_items.is_empty() && self.selected_index + 1 < self.filtered_items.len() {
            self.selected_index += 1;
            self.list_scroll_handle.scroll_to_item(self.selected_index);
            cx.notify();
        }
    }

    pub fn select_prev(&mut self, cx: &mut Context<Self>) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.list_scroll_handle.scroll_to_item(self.selected_index);
            cx.notify();
        }
    }

    pub fn copy_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.copy_index(self.selected_index, window, cx);
    }

    pub fn copy_index(&mut self, idx: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(item) = self.filtered_items.get(idx) {
            let content = item.content.clone();
            let _ = ditto_core::clipboard::set_text(&content);
            let _ = self.state.insert_clip(&content);
            self.selected_index = 0;
            self.update_filter();
            self.hide_overlay(window, cx);
        }
    }

    pub fn delete_selected(&mut self, cx: &mut Context<Self>) {
        if let Some(item) = self.filtered_items.get(self.selected_index) {
            let id = item.id;
            let _ = self.state.delete_clip(id);
            self.update_filter();
            if self.selected_index >= self.filtered_items.len() {
                self.selected_index = self.filtered_items.len().saturating_sub(1);
            }
            self.list_scroll_handle.scroll_to_item(self.selected_index);
            cx.notify();
        }
    }

    pub fn hide_overlay(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let clear = self.state.config.read().escape_clears_search;
        if clear {
            self.query.clear();
            self.search_input.update(cx, |input, cx| {
                input.clean(window, cx);
            });
            self.selected_index = 0;
            self.update_filter();
        }

        // Save window position and hide via X11 UnmapWindow (keeps WGPU surface alive)
        #[cfg(target_os = "linux")]
        if let Some((x, y)) = crate::platform::hide_window() {
            if x > 10 && y > 10 {
                let mut cfg = self.state.config.read().clone();
                cfg.window_x = Some(x as f32);
                cfg.window_y = Some(y as f32);
                let _ = self.state.save_config(cfg);
            }
        }

        #[cfg(not(target_os = "linux"))]
        window.remove_window();
    }

    pub fn on_show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.shown_at = Instant::now();
        self.in_settings = false; // Always return to default clipboard window on show!
        self.update_filter();
        self.search_input.update(cx, |input, cx| {
            input.focus(window, cx);
        });
        cx.notify();
    }

    pub fn toggle_settings(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.in_settings = !self.in_settings;
        cx.notify();
    }

    fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = event.keystroke.key.as_str();
        let modifiers = &event.keystroke.modifiers;

        if modifiers.control || modifiers.platform {
            if key == "," {
                self.toggle_settings(window, cx);
                return;
            } else if key == "l" {
                let _ = self.state.clear_history();
                self.update_filter();
                cx.notify();
                return;
            }
        }

        if modifiers.alt {
            if let Ok(num) = key.parse::<usize>() {
                if num >= 1 && num <= 9 {
                    self.copy_index(num - 1, window, cx);
                    return;
                }
            }
        }

        match key {
            "escape" => {
                if self.in_settings {
                    self.in_settings = false;
                    cx.notify();
                } else {
                    self.hide_overlay(window, cx);
                }
            }
            "down" | "arrowdown" => {
                if !self.in_settings {
                    let now = Instant::now();
                    if now.duration_since(self.last_nav_time) >= Duration::from_millis(35) {
                        self.last_nav_time = now;
                        self.select_next(cx);
                    }
                }
            }
            "up" | "arrowup" => {
                if !self.in_settings {
                    let now = Instant::now();
                    if now.duration_since(self.last_nav_time) >= Duration::from_millis(35) {
                        self.last_nav_time = now;
                        self.select_prev(cx);
                    }
                }
            }
            "enter" => {
                if !self.in_settings {
                    self.copy_selected(window, cx);
                }
            }
            "delete" => {
                if !self.in_settings {
                    self.delete_selected(cx);
                }
            }
            _ => {}
        }
    }
}

impl Render for DittoOverlayView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme_colors.clone();
        let is_persistent = self.state.config.read().persistent_window;
        let is_paused = self.state.is_paused();

        let mut outer = div()
            .id("ditto-overlay-root")
            .track_focus(&self.focus_handle)
            // Intercept Escape action regardless of input focus!
            .on_action(
                cx.listener(|this, _: &gpui_kit::component::input::Escape, window, cx| {
                    if this.in_settings {
                        this.in_settings = false;
                        cx.notify();
                    } else {
                        this.hide_overlay(window, cx);
                    }
                }),
            )
            .on_key_down(cx.listener(Self::handle_key_down))
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(px(12.0))
            .border_1()
            .border_color(theme.border_card)
            .bg(theme.bg_surface)
            .text_color(theme.text_primary);

        // 1. Persistent Window Header (ONLY shown when NOT in settings and persistent_window == true!)
        // Drag handler isolated to title area so trackpad taps on close button register immediately!
        if is_persistent && !self.in_settings {
            let logo_img = Arc::new(Image {
                id: 42,
                format: ImageFormat::Png,
                bytes: LOGO_PNG.to_vec(),
            });

            outer = outer.child(
                h_flex()
                    .id("persistent-header")
                    .h(px(40.0))
                    .px(px(16.0))
                    .justify_between()
                    .items_center()
                    .bg(rgba(0x0a0a0cd9)) // rgba(10, 10, 12, 0.85)
                    .border_b_1()
                    .border_color(theme.border_card)
                    .child(
                        h_flex()
                            .id("persistent-header-drag-area")
                            .flex_1()
                            .h_full()
                            .items_center()
                            .gap(px(8.0))
                            .cursor_move()
                            .on_mouse_down(gpui_kit::MouseButton::Left, |_, window, _| {
                                window.start_window_move();
                            })
                            .child(
                                div()
                                    .size(px(18.0))
                                    .rounded(px(4.0))
                                    .overflow_hidden()
                                    .child(img(logo_img).size_full()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(theme.text_primary)
                                    .child("DITTO"),
                            ),
                    )
                    .child(
                        div()
                            .id("btn-close-header")
                            .size(px(24.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(6.0))
                            .hover(|s| s.bg(rgba(0xffffff0d)))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.hide_overlay(window, cx);
                            }))
                            .child(
                                svg()
                                    .data(ICON_CLOSE.as_bytes())
                                    .size(px(14.0))
                                    .text_color(theme.text_secondary),
                            ),
                    ),
            );
        }

        // 2. Body View: Settings Screen or 2-Column Clipboard List
        if self.in_settings {
            let settings_view = self.render_settings_screen(window, cx);
            outer.child(settings_view)
        } else {
            let active_item = self.filtered_items.get(self.selected_index).cloned();
            let left_col = self.render_left_column(&theme, is_paused, window, cx);
            let right_col = self.render_right_column(&theme, active_item, window, cx);

            outer.child(
                h_flex()
                    .flex_1()
                    .size_full()
                    .overflow_hidden()
                    .child(left_col)
                    .child(right_col),
            )
        }
    }
}

impl DittoOverlayView {
    /// Renders the 320px left column (fixed-width, non-shrinking, non-expanding)
    fn render_left_column(
        &mut self,
        theme: &ThemeColors,
        is_paused: bool,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let history_len = self.filtered_items.len();
        let selected_idx = self.selected_index;

        // Search Header Container (NO start_window_move so clicking doesn't close window!)
        let search_header = h_flex()
            .id("search-header-container")
            .p(px(16.0))
            .border_b_1()
            .border_color(theme.border_card)
            .items_center()
            .child(
                h_flex()
                    .w_full()
                    .h(px(38.0))
                    .px(px(10.0))
                    .gap(px(8.0))
                    .items_center()
                    .rounded(px(8.0))
                    .bg(rgba(0xffffff08))
                    .border_1()
                    .border_color(theme.border_card)
                    .child(
                        svg()
                            .data(ICON_SEARCH.as_bytes())
                            .size(px(16.0))
                            .text_color(theme.text_secondary),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(Input::new(&self.search_input).appearance(false)),
                    ),
            );

        // Subheader (Item count + Live/Paused interactive toggle + Settings gear)
        let subheader = h_flex()
            .id("subheader-status-bar")
            .h(px(34.0))
            .px(px(16.0))
            .py(px(6.0))
            .border_b_1()
            .border_color(theme.border_card)
            .justify_between()
            .items_center()
            .text_xs()
            .text_color(theme.text_secondary)
            .child(format!("{} Items", history_len))
            .child(
                h_flex()
                    .gap(px(12.0))
                    .items_center()
                    // Interactive Live / Paused toggle button (replicates Tauri 1:1)
                    .child(
                        div()
                            .id("btn-toggle-pause")
                            .cursor_pointer()
                            .px(px(6.0))
                            .py(px(2.0))
                            .rounded(px(4.0))
                            .hover(|s| s.bg(rgba(0xffffff14)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                let next = !this.state.is_paused();
                                this.state.set_paused(next);
                                cx.notify();
                            }))
                            .child(
                                h_flex()
                                    .gap(px(6.0))
                                    .items_center()
                                    .child(div().size(px(6.0)).rounded_full().bg(if is_paused {
                                        rgb(0xf59e0b)
                                    } else {
                                        rgb(0x22c55e)
                                    }))
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(if is_paused {
                                                rgb(0xf59e0b)
                                            } else {
                                                rgb(0xffffff)
                                            })
                                            .child(if is_paused { "Paused" } else { "Live" }),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("btn-open-settings")
                            .cursor_pointer()
                            .p(px(2.0))
                            .rounded(px(4.0))
                            .hover(|s| s.bg(rgba(0xffffff14)))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.toggle_settings(window, cx);
                            }))
                            .child(
                                svg()
                                    .data(ICON_SETTINGS.as_bytes())
                                    .size(px(14.0))
                                    .text_color(theme.text_secondary),
                            ),
                    ),
            );

        // Scrollable History Cards List with proper padding to avoid right gap
        let mut list_cards = v_flex()
            .id("history-list-cards")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.list_scroll_handle)
            .pl(px(12.0))
            .pr(px(16.0))
            .py(px(12.0))
            .gap(px(8.0));

        for (idx, item) in self.card_views.iter().enumerate() {
            let is_selected = idx == selected_idx;
            let is_img = item.is_image;
            let relative_time = item.relative_time.clone();
            let preview_text = item.preview_text.clone();

            let card = h_flex()
                .id(idx)
                .p(px(10.0))
                .rounded(px(8.0))
                .justify_between()
                .items_center()
                .cursor_pointer()
                .border_1()
                .border_color(if is_selected {
                    theme.accent_primary
                } else {
                    theme.border_card
                })
                .bg(if is_selected {
                    rgba(0xffffff0d)
                } else {
                    rgba(0xffffff05)
                })
                .hover(|s| {
                    if is_selected {
                        s
                    } else {
                        s.bg(rgba(0xffffff0a))
                    }
                })
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.selected_index = idx;
                    this.copy_index(idx, window, cx);
                }))
                .child(
                    h_flex()
                        .gap(px(10.0))
                        .items_center()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .child(if is_img {
                            svg()
                                .data(ICON_IMAGE.as_bytes())
                                .size(px(14.0))
                                .text_color(if is_selected {
                                    theme.accent_primary
                                } else {
                                    theme.text_secondary
                                })
                                .into_any_element()
                        } else {
                            div()
                                .font_family("monospace")
                                .text_xs()
                                .font_weight(FontWeight::BOLD)
                                .text_color(if is_selected {
                                    theme.accent_primary
                                } else {
                                    theme.text_secondary
                                })
                                .child("T")
                                .into_any_element()
                        })
                        .child(
                            v_flex()
                                .gap(px(2.0))
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(theme.text_primary)
                                        .truncate()
                                        .child(preview_text),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.text_secondary)
                                        .truncate()
                                        .child(relative_time),
                                ),
                        ),
                );

            let card = if idx < 9 {
                card.child(
                    div()
                        .px(px(6.0))
                        .py(px(2.0))
                        .rounded(px(4.0))
                        .bg(rgba(0xffffff0d))
                        .border_1()
                        .border_color(theme.border_card)
                        .text_xs()
                        .font_family("monospace")
                        .text_color(theme.text_secondary)
                        .child(format!("⌥{}", idx + 1)),
                )
            } else {
                card
            };

            list_cards = list_cards.child(card);
        }

        // Relative container with persistent overlay scrollbar (flush right)
        let list_container = div()
            .relative()
            .flex_1()
            .overflow_hidden()
            .child(list_cards)
            .child(
                div()
                    .absolute()
                    .top(px(4.0))
                    .right(px(3.0))
                    .bottom(px(4.0))
                    .w(px(6.0))
                    .child(gpui_kit::component::scroll::Scrollbar::vertical(
                        &self.list_scroll_handle,
                    )),
            );

        v_flex()
            .w(px(320.0))
            .flex_none()
            .h_full()
            .bg(rgba(0x0a0a0c99)) // rgba(10, 10, 12, 0.60)
            .border_r_1()
            .border_color(theme.border_card)
            .child(search_header)
            .child(subheader)
            .child(list_container)
    }

    /// Renders the flexible right column (preview, metadata, shortcuts footer)
    fn render_right_column(
        &mut self,
        theme: &ThemeColors,
        active_item: Option<ClipboardItem>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> impl IntoElement {
        let content_view = if let Some(item) = &active_item {
            let is_image = item.content.starts_with("data:image/png;base64,");
            if is_image {
                let maybe_image = if let Some((cached_id, cached_img)) = &self.cached_preview_image
                {
                    if *cached_id == item.id {
                        Some(cached_img.clone())
                    } else {
                        None
                    }
                } else {
                    None
                };

                let image_data = match maybe_image {
                    Some(img) => Some(img),
                    None => {
                        let b64 = item.content.trim_start_matches("data:image/png;base64,");
                        if let Ok(bytes) = BASE64_STANDARD.decode(b64) {
                            let img_arc = Arc::new(Image {
                                id: item.id as u64,
                                format: ImageFormat::Png,
                                bytes,
                            });
                            self.cached_preview_image = Some((item.id, img_arc.clone()));
                            Some(img_arc)
                        } else {
                            None
                        }
                    }
                };

                if let Some(image_data) = image_data {
                    div()
                        .flex_1()
                        .p(px(20.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .w(px(420.0))
                                .h(px(260.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(8.0))
                                .overflow_hidden()
                                .border_1()
                                .border_color(theme.border_card)
                                .bg(rgba(0x00000033))
                                .child(
                                    img(image_data)
                                        .max_w(px(400.0))
                                        .max_h(px(240.0))
                                        .object_fit(gpui_kit::ObjectFit::Contain),
                                ),
                        )
                        .into_any_element()
                } else {
                    div()
                        .flex_1()
                        .p(px(20.0))
                        .text_xs()
                        .text_color(theme.text_secondary)
                        .child("Failed to decode base64 image data")
                        .into_any_element()
                }
            } else {
                v_flex()
                    .id("preview-content-scroll")
                    .flex_1()
                    .min_w_0()
                    .overflow_y_scroll()
                    .overflow_x_scroll()
                    .p(px(20.0))
                    .child(
                        div()
                            .font_family("monospace")
                            .text_sm()
                            .text_color(theme.text_primary)
                            .child(if item.content.len() > 30_000 {
                                let mut truncated =
                                    item.content.chars().take(30_000).collect::<String>();
                                truncated.push_str("\n\n... [Content truncated for preview]");
                                truncated
                            } else {
                                item.content.clone()
                            }),
                    )
                    .into_any_element()
            }
        } else {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(theme.text_secondary)
                .child("No clipboard items found")
                .into_any_element()
        };

        // Metadata section with generous bottom padding for shortcuts hints
        let metadata_section = if let Some(item) = &active_item {
            let is_image = item.content.starts_with("data:image/png;base64,");
            let mime_str = if is_image { "image/png" } else { "text/plain" };
            let size_str = format_size(item.content.len());
            let copied_at = item.created_at.clone();
            let checksum = get_simple_hash(&item.content);

            v_flex()
                .h(px(154.0))
                .px(px(20.0))
                .pt(px(14.0))
                .pb(px(14.0))
                .gap(px(8.0))
                .bg(rgb(0x0f0f12))
                .border_t_1()
                .border_color(theme.border_card)
                .child(
                    v_flex()
                        .gap(px(3.0))
                        .text_xs()
                        .child(self.meta_row("Mime", mime_str, theme))
                        .child(self.meta_row("Size", &size_str, theme))
                        .child(self.meta_row("Copied at", &copied_at, theme))
                        .child(self.meta_row("Checksum", &checksum, theme)),
                )
                .child(
                    h_flex()
                        .gap(px(16.0))
                        .pt(px(6.0))
                        .pb(px(4.0))
                        .border_t_1()
                        .border_color(theme.border_card)
                        .items_center()
                        .text_xs()
                        .child(self.shortcut_hint("Enter", "Copy", theme))
                        .child(self.shortcut_hint("Esc", "Hide", theme))
                        .child(self.shortcut_hint("Ctrl+,", "Settings", theme)),
                )
        } else {
            v_flex()
                .h(px(154.0))
                .px(px(20.0))
                .pt(px(14.0))
                .pb(px(14.0))
                .justify_center()
                .bg(rgb(0x0f0f12))
                .border_t_1()
                .border_color(theme.border_card)
                .child(
                    h_flex()
                        .gap(px(16.0))
                        .items_center()
                        .text_xs()
                        .child(self.shortcut_hint("Esc", "Hide", theme))
                        .child(self.shortcut_hint("Ctrl+,", "Settings", theme)),
                )
        };

        v_flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_hidden()
            .bg(rgba(0x0c0c0e66)) // rgba(12, 12, 14, 0.40)
            .child(content_view)
            .child(metadata_section)
    }

    fn meta_row(&self, key: &str, val: &str, theme: &ThemeColors) -> impl IntoElement {
        h_flex()
            .gap(px(12.0))
            .child(
                div()
                    .w(px(90.0))
                    .text_color(theme.text_secondary)
                    .child(key.to_string()),
            )
            .child(
                div()
                    .font_family("monospace")
                    .text_color(theme.text_primary)
                    .child(val.to_string()),
            )
    }

    fn shortcut_hint(&self, key: &str, label: &str, theme: &ThemeColors) -> impl IntoElement {
        h_flex()
            .gap(px(6.0))
            .items_center()
            .child(
                div()
                    .px(px(6.0))
                    .py(px(1.0))
                    .rounded(px(4.0))
                    .bg(rgba(0xffffff0d))
                    .border_1()
                    .border_color(theme.border_card)
                    .font_family("monospace")
                    .text_xs()
                    .text_color(theme.text_secondary)
                    .child(key.to_string()),
            )
            .child(
                div()
                    .text_color(theme.text_secondary)
                    .child(label.to_string()),
            )
    }

    fn settings_card(
        &self,
        title: &'static str,
        desc: &'static str,
        theme: &ThemeColors,
        right: impl IntoElement,
    ) -> impl IntoElement {
        h_flex()
            .justify_between()
            .items_center()
            .p(px(12.0))
            .rounded(px(8.0))
            .bg(rgba(0x00000033))
            .border_1()
            .border_color(theme.border_card)
            .child(
                v_flex()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text_primary)
                            .child(title),
                    )
                    .child(div().text_xs().text_color(theme.text_secondary).child(desc)),
            )
            .child(right)
    }
    /// Renders the complete settings view (all 7 options matching Tauri App.tsx 1:1)
    /// Fully live with instant persistence on change, transparent acrylic glass surface!
    fn render_settings_screen(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let cfg = self.state.config.read().clone();
        let theme = self.theme_colors.clone();
        let cur_theme = cfg.theme.clone();
        let cur_opacity = cfg.opacity;
        let cur_max_items = cfg.max_items;
        let cur_persistent = cfg.persistent_window;
        let cur_esc = cfg.escape_clears_search;
        let ignored_apps = cfg.ignored_apps.clone();

        let logo_img = Arc::new(Image {
            id: 43,
            format: ImageFormat::Png,
            bytes: LOGO_PNG.to_vec(),
        });

        // Settings scroll body with right padding so scrollbar does NOT overlap boxes!
        let settings_body = v_flex()
            .id("settings-scroll-body")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.settings_scroll_handle)
            .gap(px(10.0))
            .pl(px(2.0))
            .pr(px(16.0))
            .py(px(4.0))
            // 1. Persistent Window Mode (Matches App.tsx line 228)
            .child(self.settings_card(
                "Persistent Window Mode",
                "If enabled, the window remains open when you click outside. It also displays a custom header with a title and close button.",
                &theme,
                self.render_toggle_pill("toggle-persistent-btn", cur_persistent, &theme, cx.listener(|this, _, _, cx| {
                    let mut cfg = this.state.config.read().clone();
                    cfg.persistent_window = !cfg.persistent_window;
                    let _ = this.state.save_config(cfg);
                    cx.notify();
                })),
            ))
            // 2. Escape Clears Search Query (Matches App.tsx line 254)
            .child(self.settings_card(
                "Escape Clears Search Query",
                "Pressing Escape clears search query first before closing window.",
                &theme,
                self.render_toggle_pill("toggle-esc-btn", cur_esc, &theme, cx.listener(|this, _, _, cx| {
                    let mut cfg = this.state.config.read().clone();
                    cfg.escape_clears_search = !cfg.escape_clears_search;
                    let _ = this.state.save_config(cfg);
                    cx.notify();
                })),
            ))
            // 3. History Size Limit (Text Input matching Tauri App.tsx line 282)
            .child(
                v_flex()
                    .p(px(12.0))
                    .gap(px(8.0))
                    .rounded(px(8.0))
                    .bg(rgba(0x00000033))
                    .border_1()
                    .border_color(theme.border_card)
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.text_primary)
                                    .child("History Size Limit"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_family("monospace")
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(theme.accent_primary)
                                    .child(format!("{} items", cur_max_items)),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap(px(8.0))
                            .items_center()
                            .child(
                                h_flex()
                                    .w(px(130.0))
                                    .h(px(32.0))
                                    .px(px(8.0))
                                    .items_center()
                                    .rounded(px(6.0))
                                    .bg(rgba(0xffffff08))
                                    .border_1()
                                    .border_color(theme.border_card)
                                    .child(
                                        Input::new(&self.history_limit_input)
                                            .appearance(false),
                                    ),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.text_secondary)
                                    .child("clips retained in database (1-5000)"),
                            ),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.text_secondary)
                                    .child("Maximum number of clipboard items retained in SQLite database."),
                            )
                            .child(
                                h_flex()
                                    .gap(px(4.0))
                                    .items_center()
                                    .child(self.render_stepper_btn("-50", cx.listener(|this, _, window, cx| {
                                        let mut cfg = this.state.config.read().clone();
                                        cfg.max_items = cfg.max_items.saturating_sub(50).max(10);
                                        let num = cfg.max_items;
                                        let _ = this.state.save_config(cfg);
                                        this.history_limit_input.update(cx, |input, cx| {
                                            input.replace_all(num.to_string(), window, cx);
                                        });
                                        this.update_filter();
                                        cx.notify();
                                    }), &theme))
                                    .child(self.render_stepper_btn("+50", cx.listener(|this, _, window, cx| {
                                        let mut cfg = this.state.config.read().clone();
                                        cfg.max_items = (cfg.max_items + 50).min(5000);
                                        let num = cfg.max_items;
                                        let _ = this.state.save_config(cfg);
                                        this.history_limit_input.update(cx, |input, cx| {
                                            input.replace_all(num.to_string(), window, cx);
                                        });
                                        this.update_filter();
                                        cx.notify();
                                    }), &theme)),
                            ),
                    ),
            )
            // 4. Window Opacity (Concrete Slider matching Tauri App.tsx line 298)
            .child(
                v_flex()
                    .p(px(12.0))
                    .gap(px(10.0))
                    .rounded(px(8.0))
                    .bg(rgba(0x00000033))
                    .border_1()
                    .border_color(theme.border_card)
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.text_primary)
                                    .child("Window Opacity"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_family("monospace")
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(theme.accent_primary)
                                    .child(format!("{}%", cur_opacity)),
                            ),
                    )
                    // Concrete interactive Slider
                    .child(
                        div()
                            .w_full()
                            .py(px(4.0))
                            .child(
                                Slider::new(&self.opacity_slider)
                                    .horizontal(),
                            ),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.text_secondary)
                                    .child("Set background transparency level of clipboard window."),
                            )
                            .child(
                                h_flex()
                                    .gap(px(4.0))
                                    .items_center()
                                    .child(self.render_stepper_btn("-5%", cx.listener(|this, _, window, cx| {
                                        let mut cfg = this.state.config.read().clone();
                                        cfg.opacity = cfg.opacity.saturating_sub(5).max(20);
                                        let op = cfg.opacity;
                                        this.theme_colors = ThemeColors::from_config(&cfg);
                                        let _ = this.state.save_config(cfg);
                                        this.opacity_slider.update(cx, |slider, cx| {
                                            slider.set_value(op as f32, window, cx);
                                        });
                                        cx.notify();
                                    }), &theme))
                                    .child(self.render_stepper_btn("+5%", cx.listener(|this, _, window, cx| {
                                        let mut cfg = this.state.config.read().clone();
                                        cfg.opacity = (cfg.opacity + 5).min(100);
                                        let op = cfg.opacity;
                                        this.theme_colors = ThemeColors::from_config(&cfg);
                                        let _ = this.state.save_config(cfg);
                                        this.opacity_slider.update(cx, |slider, cx| {
                                            slider.set_value(op as f32, window, cx);
                                        });
                                        cx.notify();
                                    }), &theme)),
                            ),
                    ),
            )
            // 5. Visual Theme Accent (Matches App.tsx line 321)
            .child(
                v_flex()
                    .p(px(12.0))
                    .gap(px(8.0))
                    .rounded(px(8.0))
                    .bg(rgba(0x00000033))
                    .border_1()
                    .border_color(theme.border_card)
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.text_primary)
                                    .child("Visual Theme Accent"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_family("monospace")
                                    .text_color(theme.accent_primary)
                                    .child(format!("Active: {}", cur_theme)),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap(px(6.0))
                            .items_center()
                            .child(self.custom_theme_btn("Dark", "dark", &cur_theme, &theme, cx))
                            .child(self.custom_theme_btn("Cyan", "cyan", &cur_theme, &theme, cx))
                            .child(self.custom_theme_btn("Emerald", "emerald", &cur_theme, &theme, cx))
                            .child(self.custom_theme_btn("Amber", "amber", &cur_theme, &theme, cx))
                            .child(self.custom_theme_btn("Rose", "rose", &cur_theme, &theme, cx))
                            .child(self.custom_theme_btn("Light Pure", "light-pure", &cur_theme, &theme, cx))
                            .child(self.custom_theme_btn("Nordic", "light-nordic", &cur_theme, &theme, cx))
                            .child(self.custom_theme_btn("Custom", "custom", &cur_theme, &theme, cx)),
                    ),
            )
            // 6. Custom Colors Picker Panel (Matches App.tsx line 354)
            .child(
                if cur_theme == "custom" {
                    h_flex()
                        .gap(px(10.0))
                        .p(px(12.0))
                        .rounded(px(8.0))
                        .bg(rgba(0x00000033))
                        .border_1()
                        .border_color(theme.border_card)
                        .child(self.color_preview_box("Accent", &cfg.custom_accent, &theme))
                        .child(self.color_preview_box("Primary Text", &cfg.custom_primary, &theme))
                        .child(self.color_preview_box("Secondary Text", &cfg.custom_secondary, &theme))
                        .into_any_element()
                } else {
                    div().into_any_element()
                },
            )
            // 7. Ignored Applications (Matches App.tsx line 410)
            .child(
                v_flex()
                    .p(px(12.0))
                    .gap(px(6.0))
                    .rounded(px(8.0))
                    .bg(rgba(0x00000033))
                    .border_1()
                    .border_color(theme.border_card)
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text_primary)
                            .child("Ignored Applications (Keywords)"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_secondary)
                            .child("Ignore clipboard entries containing these keywords (case-insensitive):"),
                    )
                    .child(
                        h_flex()
                            .gap(px(6.0))
                            .items_center()
                            .children(ignored_apps.iter().map(|app| {
                                div()
                                    .text_xs()
                                    .font_family("monospace")
                                    .px(px(8.0))
                                    .py(px(3.0))
                                    .rounded(px(4.0))
                                    .bg(rgba(0xffffff0d))
                                    .border_1()
                                    .border_color(theme.border_card)
                                    .child(app.clone())
                            })),
                    ),
            );

        // Relative container with persistent overlay scrollbar for settings (clean right gutter)
        let settings_container = div()
            .relative()
            .flex_1()
            .overflow_hidden()
            .child(settings_body)
            .child(
                div()
                    .absolute()
                    .top(px(4.0))
                    .right(px(3.0))
                    .bottom(px(4.0))
                    .w(px(6.0))
                    .child(gpui_kit::component::scroll::Scrollbar::vertical(
                        &self.settings_scroll_handle,
                    )),
            );

        v_flex()
            .id("settings-screen")
            .size_full()
            .p(px(20.0))
            .gap(px(14.0))
            .bg(theme.bg_surface) // Transparent acrylic glass surface matching main overlay!
            .text_color(theme.text_primary)
            .child(
                // Header (Draggable title area isolated from buttons so trackpad taps register immediately!)
                h_flex()
                    .id("settings-header")
                    .justify_between()
                    .items_center()
                    .border_b_1()
                    .border_color(theme.border_card)
                    .pb(px(12.0))
                    .child(
                        div()
                            .id("btn-back-to-clipboard")
                            .cursor_pointer()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(theme.border_card)
                            .bg(rgba(0xffffff0d))
                            .hover(|s| s.bg(rgba(0xffffff1a)).text_color(theme.text_primary))
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text_secondary)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.in_settings = false;
                                cx.notify();
                            }))
                            .child("← Back"),
                    )
                    .child(
                        h_flex()
                            .id("settings-header-drag-area")
                            .flex_1()
                            .h_full()
                            .items_center()
                            .justify_center()
                            .gap(px(8.0))
                            .cursor_move()
                            .on_mouse_down(gpui_kit::MouseButton::Left, |_, window, _| {
                                window.start_window_move();
                            })
                            .child(
                                div()
                                    .size(px(18.0))
                                    .rounded(px(4.0))
                                    .overflow_hidden()
                                    .child(img(logo_img).size_full()),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(theme.text_primary)
                                    .child("Ditto Settings"),
                            ),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.accent_primary)
                                    .bg(rgba(0xffffff0a))
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .rounded(px(4.0))
                                    .border_1()
                                    .border_color(theme.border_card)
                                    .child("v1.0.0"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_family("monospace")
                                    .text_color(theme.text_secondary)
                                    .bg(rgba(0xffffff0a))
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .rounded(px(4.0))
                                    .border_1()
                                    .border_color(theme.border_card)
                                    .child("config.json"),
                            )
                            .child(
                                div()
                                    .id("btn-close-settings-x")
                                    .cursor_pointer()
                                    .p(px(4.0))
                                    .rounded(px(6.0))
                                    .hover(|s| s.bg(rgba(0xffffff14)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.in_settings = false;
                                        cx.notify();
                                    }))
                                    .child(
                                        svg()
                                            .data(ICON_CLOSE.as_bytes())
                                            .size(px(14.0))
                                            .text_color(theme.text_secondary),
                                    ),
                            ),
                    ),
            )
            .child(settings_container)
    }

    fn color_preview_box(
        &self,
        label: &'static str,
        val: &str,
        theme: &ThemeColors,
    ) -> impl IntoElement {
        v_flex()
            .flex_1()
            .gap(px(4.0))
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_secondary)
                    .child(label),
            )
            .child(
                h_flex()
                    .gap(px(6.0))
                    .items_center()
                    .px(px(8.0))
                    .py(px(4.0))
                    .rounded(px(6.0))
                    .bg(theme.bg_base)
                    .border_1()
                    .border_color(theme.border_card)
                    .child(
                        div()
                            .size(px(14.0))
                            .rounded(px(3.0))
                            .bg(theme.accent_primary),
                    )
                    .child(
                        div()
                            .font_family("monospace")
                            .text_xs()
                            .text_color(theme.text_primary)
                            .child(val.to_string()),
                    ),
            )
    }

    fn render_stepper_btn(
        &self,
        label: &'static str,
        on_click: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
        theme: &ThemeColors,
    ) -> impl IntoElement {
        div()
            .id(label)
            .px(px(6.0))
            .py(px(4.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .text_xs()
            .font_family("monospace")
            .bg(rgba(0xffffff0a))
            .text_color(theme.text_secondary)
            .border_1()
            .border_color(theme.border_card)
            .hover(|s| s.bg(rgba(0xffffff1a)).text_color(theme.text_primary))
            .on_click(on_click)
            .child(label)
    }

    fn render_toggle_pill(
        &self,
        id: &'static str,
        enabled: bool,
        theme: &ThemeColors,
        on_click: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
    ) -> impl IntoElement {
        h_flex()
            .id(id)
            .w(px(36.0))
            .h(px(20.0))
            .rounded_full()
            .cursor_pointer()
            .items_center()
            .px(px(2.0))
            .bg(if enabled {
                theme.accent_primary
            } else {
                rgba(0xffffff26)
            })
            .on_click(on_click)
            .child(
                div()
                    .size(px(16.0))
                    .rounded_full()
                    .bg(rgb(0xffffff))
                    .ml(if enabled { px(16.0) } else { px(0.0) }),
            )
    }

    fn custom_theme_btn(
        &self,
        label: &'static str,
        name: &'static str,
        cur_theme: &str,
        theme: &ThemeColors,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_active = cur_theme == name;
        div()
            .id(format!("theme-btn-{}", name))
            .px(px(8.0))
            .py(px(4.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .text_xs()
            .font_weight(if is_active {
                FontWeight::BOLD
            } else {
                FontWeight::NORMAL
            })
            .bg(if is_active {
                theme.accent_primary
            } else {
                rgba(0xffffff0d)
            })
            .text_color(if is_active {
                rgb(0x000000)
            } else {
                theme.text_primary
            })
            .border_1()
            .border_color(if is_active {
                theme.accent_primary
            } else {
                theme.border_card
            })
            .hover(|s| if is_active { s } else { s.bg(rgba(0xffffff1a)) })
            .on_click(cx.listener(move |this, _, _, cx| {
                let mut cfg = this.state.config.read().clone();
                cfg.theme = name.to_string();
                this.theme_colors = ThemeColors::from_config(&cfg);
                let _ = this.state.save_config(cfg);
                cx.notify();
            }))
            .child(label)
    }
}
