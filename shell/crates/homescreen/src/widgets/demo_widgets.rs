use crate::layout_manager::WidgetId;
use crate::widgets::Widget;
use gpui::*;

pub struct DemoWidget {
    id: WidgetId,
    bounds: Bounds<Pixels>,
    text: String,
    emoji: String,
    color: Hsla,
}

impl DemoWidget {
    pub fn new(id: WidgetId, text: impl Into<String>, emoji: impl Into<String>, color: Hsla) -> Self {
        Self {
            id,
            bounds: Bounds::default(),
            text: text.into(),
            emoji: emoji.into(),
            color,
        }
    }
}

impl Widget for DemoWidget {
    fn widget_id(&self) -> WidgetId {
        self.id
    }

    fn render(&mut self, _window: &mut Window, _cx: &mut App) -> AnyElement {
        div()
            .size_full()
            .bg(self.color)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .child(
                div()
                    .text_2xl()
                    .text_color(rgb(0xffffff))
                    .child(self.emoji.clone()),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0xffffff))
                    .child(self.text.clone()),
            )
            .into_any_element()
    }

    fn set_bounds(&mut self, bounds: Bounds<Pixels>) {
        self.bounds = bounds;
    }
}
