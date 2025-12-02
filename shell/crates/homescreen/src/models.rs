use crate::{
    config::{GridConfig, HomescreenConfig},
    layout_manager::{LayoutManager, LayoutPage, WidgetId},
    solver::AndroidLayoutSolver,
    widgets::{demo_widgets::DemoWidget, Widget},
};
use gpui::*;

pub struct HomescreenState {
    layout_manager: LayoutManager<AndroidLayoutSolver>,
    pub widgets: Vec<Box<dyn Widget>>,
}

impl HomescreenState {
    pub fn new(config: &HomescreenConfig) -> Self {
        // Create default pages based on config
        let page_config = GridConfig {
            cols: config.grid.cols,
            rows: config.grid.rows,
            ..Default::default()
        };

        // Create 3 pages
        let mut page1 = LayoutPage::new(page_config.clone());
        let mut page2 = LayoutPage::new(page_config.clone());
        let mut page3 = LayoutPage::new(page_config);

        // Create demo widgets for page 1
        let widgets: Vec<Box<dyn Widget>> = vec![
            // Page 1 widgets
            Box::new(DemoWidget::new(
                WidgetId(1),
                "Clock",
                "🕐",
                rgb(0x3b82f6).into(),
            )),
            Box::new(DemoWidget::new(
                WidgetId(2),
                "Calendar",
                "📅",
                rgb(0xef4444).into(),
            )),
            Box::new(DemoWidget::new(
                WidgetId(3),
                "Weather",
                "☀️",
                rgb(0x10b981).into(),
            )),
            Box::new(DemoWidget::new(
                WidgetId(4),
                "Music",
                "🎵",
                rgb(0x8b5cf6).into(),
            )),
            Box::new(DemoWidget::new(
                WidgetId(5),
                "Notes",
                "📝",
                rgb(0xf59e0b).into(),
            )),
            // Page 2 widgets
            Box::new(DemoWidget::new(
                WidgetId(6),
                "Photos",
                "📷",
                rgb(0xec4899).into(),
            )),
            Box::new(DemoWidget::new(
                WidgetId(7),
                "Messages",
                "💬",
                rgb(0x06b6d4).into(),
            )),
            Box::new(DemoWidget::new(
                WidgetId(8),
                "Email",
                "📧",
                rgb(0xf97316).into(),
            )),
            Box::new(DemoWidget::new(
                WidgetId(9),
                "Settings",
                "⚙️",
                rgb(0x64748b).into(),
            )),
            // Page 3 widgets
            Box::new(DemoWidget::new(
                WidgetId(10),
                "Fitness",
                "🏃",
                rgb(0x22c55e).into(),
            )),
            Box::new(DemoWidget::new(
                WidgetId(11),
                "Maps",
                "🗺️",
                rgb(0x0ea5e9).into(),
            )),
            Box::new(DemoWidget::new(
                WidgetId(12),
                "News",
                "📰",
                rgb(0x991b1b).into(),
            )),
            Box::new(DemoWidget::new(
                WidgetId(13),
                "Stocks",
                "📈",
                rgb(0x15803d).into(),
            )),
        ];

        // Add widgets to page 1 with simple grid positions
        use crate::layout_manager::GridRect;
        page1.add_widget(
            WidgetId(1),
            GridRect {
                col: 0,
                row: 0,
                w: 1,
                h: 1,
            },
        );
        page1.add_widget(
            WidgetId(2),
            GridRect {
                col: 1,
                row: 0,
                w: 1,
                h: 1,
            },
        );
        page1.add_widget(
            WidgetId(3),
            GridRect {
                col: 2,
                row: 0,
                w: 2,
                h: 2,
            },
        );
        page1.add_widget(
            WidgetId(4),
            GridRect {
                col: 0,
                row: 1,
                w: 1,
                h: 1,
            },
        );
        page1.add_widget(
            WidgetId(5),
            GridRect {
                col: 1,
                row: 1,
                w: 1,
                h: 1,
            },
        );

        // Add widgets to page 2
        page2.add_widget(
            WidgetId(6),
            GridRect {
                col: 0,
                row: 0,
                w: 2,
                h: 2,
            },
        );
        page2.add_widget(
            WidgetId(7),
            GridRect {
                col: 2,
                row: 0,
                w: 1,
                h: 1,
            },
        );
        page2.add_widget(
            WidgetId(8),
            GridRect {
                col: 3,
                row: 0,
                w: 1,
                h: 1,
            },
        );
        page2.add_widget(
            WidgetId(9),
            GridRect {
                col: 2,
                row: 1,
                w: 2,
                h: 1,
            },
        );

        // Add widgets to page 3
        page3.add_widget(
            WidgetId(10),
            GridRect {
                col: 0,
                row: 0,
                w: 1,
                h: 1,
            },
        );
        page3.add_widget(
            WidgetId(11),
            GridRect {
                col: 1,
                row: 0,
                w: 2,
                h: 2,
            },
        );
        page3.add_widget(
            WidgetId(12),
            GridRect {
                col: 3,
                row: 0,
                w: 1,
                h: 2,
            },
        );
        page3.add_widget(
            WidgetId(13),
            GridRect {
                col: 0,
                row: 1,
                w: 1,
                h: 1,
            },
        );

        let layout_manager =
            LayoutManager::new(vec![page1, page2, page3], 0, config.window.clone(), config.grid.clone());

        Self {
            layout_manager,
            widgets,
        }
    }

    pub fn layout_manager(&self) -> &LayoutManager<AndroidLayoutSolver> {
        &self.layout_manager
    }

    pub fn layout_manager_mut(&mut self) -> &mut LayoutManager<AndroidLayoutSolver> {
        &mut self.layout_manager
    }

    pub fn get_widget_mut(&mut self, id: WidgetId) -> Option<&mut Box<dyn Widget>> {
        self.widgets.iter_mut().find(|w| w.widget_id() == id)
    }
}
