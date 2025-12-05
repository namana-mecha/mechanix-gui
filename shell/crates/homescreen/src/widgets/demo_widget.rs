use crate::widgets::HomescreenWidget;
use gpui::*;

pub struct DemoWidget;
impl HomescreenWidget for DemoWidget {
    fn render(&self) -> gpui::AnyElement {
        div().size_full().bg(rgb(0xffffff)).into_any_element()
    }
}
