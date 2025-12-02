use crate::layout_manager::WidgetId;
use gpui::*;

pub mod demo_widgets;

pub trait Widget {
    fn widget_id(&self) -> WidgetId;

    fn render(&mut self, window: &mut Window, cx: &mut App) -> AnyElement;

    fn set_bounds(&mut self, bounds: Bounds<Pixels>) {
        let _ = bounds;
    }
}
