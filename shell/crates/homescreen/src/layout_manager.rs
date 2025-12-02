use std::collections::HashMap;
use std::marker::PhantomData;

use crate::config::{GridConfig, WindowConfig};
use gpui::*;

#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub struct WidgetId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridRect {
    pub col: usize,
    pub row: usize,
    pub w: usize,
    pub h: usize,
}

impl GridRect {
    /// Check if this rect intersects with another rect
    pub fn intersects(&self, other: &GridRect) -> bool {
        let self_right = self.col + self.w;
        let self_bottom = self.row + self.h;
        let other_right = other.col + other.w;
        let other_bottom = other.row + other.h;

        !(self_right <= other.col
            || self.col >= other_right
            || self_bottom <= other.row
            || self.row >= other_bottom)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PixelRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Debug)]
pub struct LayoutNode {
    pub widget_id: WidgetId,
    pub rect: GridRect,
}

#[derive(Clone, Debug, Default)]
pub struct LayoutMutation {
    pub moves: Vec<(WidgetId, GridRect)>,
    pub page_changes: Vec<(WidgetId, usize)>,
}

#[derive(Clone, Debug)]
pub struct LayoutPage {
    pub config: GridConfig,
    pub nodes: HashMap<WidgetId, LayoutNode>,
}

impl LayoutPage {
    pub fn new(config: GridConfig) -> Self {
        Self {
            config,
            nodes: HashMap::new(),
        }
    }

    pub fn add_widget(&mut self, id: WidgetId, rect: GridRect) {
        self.nodes.insert(
            id,
            LayoutNode {
                widget_id: id,
                rect,
            },
        );
    }

    pub fn remove_widget(&mut self, id: WidgetId) {
        self.nodes.remove(&id);
    }

    pub fn get_node(&self, id: WidgetId) -> Option<&LayoutNode> {
        self.nodes.get(&id)
    }

    pub fn get_node_mut(&mut self, id: WidgetId) -> Option<&mut LayoutNode> {
        self.nodes.get_mut(&id)
    }

    /// Check if a rect is occupied by any widget
    pub fn is_occupied(&self, rect: GridRect) -> Option<WidgetId> {
        self.is_occupied_excluding(rect, None)
    }

    /// Check if a rect is occupied, optionally ignoring a specific widget
    pub fn is_occupied_excluding(
        &self,
        rect: GridRect,
        ignore_id: Option<WidgetId>,
    ) -> Option<WidgetId> {
        for node in self.nodes.values() {
            if let Some(id) = ignore_id {
                if node.widget_id == id {
                    continue;
                }
            }
            if node.rect.intersects(&rect) {
                return Some(node.widget_id);
            }
        }
        None
    }

    /// Check if a rect fits within the page bounds
    pub fn is_in_bounds(&self, rect: GridRect) -> bool {
        rect.col + rect.w <= self.config.cols && rect.row + rect.h <= self.config.rows
    }
}

#[derive(Debug, Clone)]
pub struct LayoutManager<S: LayoutSolver> {
    pub pages: Vec<LayoutPage>,
    pub active_page: usize,

    pub window_config: WindowConfig,
    pub grid_config: GridConfig,

    _solver: PhantomData<S>,
}

impl<S: LayoutSolver> LayoutManager<S> {
    pub fn new(
        pages: Vec<LayoutPage>,
        active_page: usize,
        window_config: WindowConfig,
        grid_config: GridConfig,
    ) -> Self {
        Self {
            pages,
            active_page,
            window_config,
            grid_config,
            _solver: PhantomData,
        }
    }

    pub fn set_grid_config(&mut self, grid_config: GridConfig) {
        self.grid_config = grid_config
    }
    pub fn set_window_config(&mut self, window_config: WindowConfig) {
        self.window_config = window_config
    }
    pub fn set_active_page(&mut self, index: usize) {
        self.active_page = index
    }

    // Getter functions for pages
    pub fn get_page(&self, index: usize) -> Option<&LayoutPage> {
        self.pages.get(index)
    }

    pub fn get_page_mut(&mut self, index: usize) -> Option<&mut LayoutPage> {
        self.pages.get_mut(index)
    }

    pub fn get_active_page(&self) -> &LayoutPage {
        &self.pages[self.active_page]
    }

    pub fn get_active_page_mut(&mut self) -> &mut LayoutPage {
        &mut self.pages[self.active_page]
    }

    pub fn get_active_page_index(&self) -> usize {
        self.active_page
    }

    pub fn get_pages(&self) -> &[LayoutPage] {
        &self.pages
    }

    // Getter functions for widgets
    pub fn get_widget(&self, widget_id: WidgetId) -> Option<(usize, &LayoutNode)> {
        for (idx, page) in self.pages.iter().enumerate() {
            if let Some(node) = page.get_node(widget_id) {
                return Some((idx, node));
            }
        }
        None
    }

    pub fn get_widget_mut(&mut self, widget_id: WidgetId) -> Option<(usize, &mut LayoutNode)> {
        for (idx, page) in self.pages.iter_mut().enumerate() {
            if let Some(node) = page.get_node_mut(widget_id) {
                return Some((idx, node));
            }
        }
        None
    }

    pub fn get_widget_page(&self, widget_id: WidgetId) -> Option<usize> {
        self.pages
            .iter()
            .position(|page| page.get_node(widget_id).is_some())
    }

    // Getter functions for configuration
    pub fn get_grid_config(&self) -> &GridConfig {
        &self.grid_config
    }

    pub fn get_window_config(&self) -> &WindowConfig {
        &self.window_config
    }

    // Unified drag handling: same-page OR cross-page
    pub fn on_drag_hover(
        &mut self,
        page_number: Option<usize>,
        widget_id: WidgetId,
        pixel_x: f32,
        pixel_y: f32,
    ) -> LayoutMutation {
        // Determine which page we're dragging to
        let to_page = page_number.unwrap_or(self.active_page);

        // Find which page the widget is currently on
        let from_page = self
            .pages
            .iter()
            .position(|page| page.get_node(widget_id).is_some())
            .unwrap_or(self.active_page);

        // Get the target page
        if to_page >= self.pages.len() {
            return LayoutMutation::default();
        }
        let page = &self.pages[to_page];

        // Convert pixel coordinates to grid rect
        let proposed_rect = self.pixel_to_grid_rect(page, pixel_x, pixel_y, widget_id);

        // Preview the drag operation
        self.preview_drag(from_page, to_page, widget_id, proposed_rect)
    }

    /// Convert GridRect to pixel bounds for rendering
    pub fn grid_to_pixel_bounds(&self, page: &LayoutPage, grid_rect: GridRect) -> Bounds<Pixels> {
        let window_width = self.window_config.width;
        let window_height = self.window_config.height;
        let padding = window_width * self.grid_config.padding_percent;
        let gap = window_width * self.grid_config.gap_percent;

        let cols = page.config.cols;
        let rows = page.config.rows;

        let available_width = window_width - (2.0 * padding);
        let available_height = window_height - (2.0 * padding);

        let total_h_gaps = (cols as f32 - 1.0) * gap;
        let total_v_gaps = (rows as f32 - 1.0) * gap;

        let cell_width = (available_width - total_h_gaps) / cols as f32;
        let cell_height = (available_height - total_v_gaps) / rows as f32;

        // Calculate position
        let x = padding + (grid_rect.col as f32 * (cell_width + gap));
        let y = padding + (grid_rect.row as f32 * (cell_height + gap));

        // Calculate size
        let w = (grid_rect.w as f32 * cell_width) + ((grid_rect.w - 1) as f32 * gap);
        let h = (grid_rect.h as f32 * cell_height) + ((grid_rect.h - 1) as f32 * gap);

        Bounds {
            origin: point(px(x), px(y)),
            size: size(px(w), px(h)),
        }
    }

    // Convert pixel -> GridRect
    fn pixel_to_grid_rect(
        &self,
        page: &LayoutPage,
        pixel_x: f32,
        pixel_y: f32,
        widget_id: WidgetId,
    ) -> GridRect {
        // Get the widget's current size (we preserve the size during drag)
        let (widget_w, widget_h) = if let Some(node) = page.get_node(widget_id) {
            (node.rect.w, node.rect.h)
        } else {
            // Default to 1x1 if widget not found
            (1, 1)
        };

        // Calculate cell dimensions
        let window_width = self.window_config.width;
        let window_height = self.window_config.height;

        let padding = window_width * self.grid_config.padding_percent;
        let gap = window_width * self.grid_config.gap_percent;

        // Available space after accounting for padding on both sides
        let available_width = window_width - (2.0 * padding);
        let available_height = window_height - (2.0 * padding);

        // Calculate cell size (total gaps = (cols - 1) * gap)
        let total_h_gaps = (page.config.cols as f32 - 1.0) * gap;
        let total_v_gaps = (page.config.rows as f32 - 1.0) * gap;

        let cell_width = (available_width - total_h_gaps) / page.config.cols as f32;
        let cell_height = (available_height - total_v_gaps) / page.config.rows as f32;

        // Convert pixel position to grid coordinates
        // Account for padding and gaps
        let rel_x = pixel_x - padding;
        let rel_y = pixel_y - padding;

        // Find which cell this pixel falls into
        let mut col = 0;
        let mut row = 0;

        let mut x_acc = 0.0;
        for c in 0..page.config.cols {
            let next_x = x_acc + cell_width + if c < page.config.cols - 1 { gap } else { 0.0 };
            if rel_x < next_x {
                col = c;
                break;
            }
            x_acc = next_x;
        }

        let mut y_acc = 0.0;
        for r in 0..page.config.rows {
            let next_y = y_acc + cell_height + if r < page.config.rows - 1 { gap } else { 0.0 };
            if rel_y < next_y {
                row = r;
                break;
            }
            y_acc = next_y;
        }

        GridRect {
            col,
            row,
            w: widget_w,
            h: widget_h,
        }
    }

    fn preview_drag(
        &mut self,
        from_page: usize,
        to_page: usize,
        widget_id: WidgetId,
        proposed_rect: GridRect,
    ) -> LayoutMutation {
        // Use the solver to calculate the drag preview
        // The solver will handle collision detection and widget displacement
        let target_page = if from_page == to_page {
            None
        } else {
            Some(to_page)
        };

        S::calculate_drag_preview(&self.pages, widget_id, target_page, proposed_rect)
    }

    pub fn transfer_widget(
        &mut self,
        widget_id: WidgetId,
        from_page: usize,
        to_page: usize,
        target_rect: GridRect,
    ) -> LayoutMutation {
        // Validate page indices
        if from_page >= self.pages.len() || to_page >= self.pages.len() {
            return LayoutMutation::default();
        }

        // If transferring to the same page, just preview the drag
        if from_page == to_page {
            return self.preview_drag(from_page, to_page, widget_id, target_rect);
        }

        // Get the widget from the source page
        let widget_node = if let Some(node) = self.pages[from_page].get_node(widget_id) {
            node.clone()
        } else {
            return LayoutMutation::default();
        };

        // Remove from source page
        self.pages[from_page].remove_widget(widget_id);

        // Add to target page temporarily to get the mutation
        self.pages[to_page].add_widget(widget_id, target_rect);

        // Calculate the drag preview for the target page
        let mutation =
            S::calculate_drag_preview(&self.pages, widget_id, Some(to_page), target_rect);

        // Remove from target page (we'll add it back when apply_mutation is called)
        self.pages[to_page].remove_widget(widget_id);

        // Restore to source page
        self.pages[from_page].add_widget(widget_id, widget_node.rect);

        mutation
    }

    pub fn on_resize_hover(
        &self,
        widget_id: WidgetId,
        new_w_px: f32,
        new_h_px: f32,
    ) -> LayoutMutation {
        // Find which page the widget is on
        let (page_idx, current_rect) = {
            let mut found = None;
            for (idx, page) in self.pages.iter().enumerate() {
                if let Some(node) = page.get_node(widget_id) {
                    found = Some((idx, node.rect));
                    break;
                }
            }
            if let Some(f) = found {
                f
            } else {
                return LayoutMutation::default();
            }
        };

        let page = &self.pages[page_idx];

        // Calculate cell dimensions
        let window_width = self.window_config.width;
        let window_height = self.window_config.height;

        let padding = window_width * self.grid_config.padding_percent;
        let gap = window_width * self.grid_config.gap_percent;

        let available_width = window_width - (2.0 * padding);
        let available_height = window_height - (2.0 * padding);

        let total_h_gaps = (page.config.cols as f32 - 1.0) * gap;
        let total_v_gaps = (page.config.rows as f32 - 1.0) * gap;

        let cell_width = (available_width - total_h_gaps) / page.config.cols as f32;
        let cell_height = (available_height - total_v_gaps) / page.config.rows as f32;

        // Convert pixel size to grid size
        let new_w_cells = ((new_w_px + gap) / (cell_width + gap)).ceil() as usize;
        let new_h_cells = ((new_h_px + gap) / (cell_height + gap)).ceil() as usize;

        // Clamp to at least 1x1
        let new_w_cells = new_w_cells.max(1);
        let new_h_cells = new_h_cells.max(1);

        // Create new rect with the resized dimensions
        let new_rect = GridRect {
            col: current_rect.col,
            row: current_rect.row,
            w: new_w_cells,
            h: new_h_cells,
        };

        // Use the solver to calculate resize preview
        S::calculate_resize_preview(&self.pages, widget_id, Some(page_idx), new_rect)
    }

    pub fn apply_mutation(&mut self, mutation: LayoutMutation) {
        // Apply page changes first (widget transfers between pages)
        for (widget_id, to_page_idx) in &mutation.page_changes {
            // Find which page the widget is currently on
            let from_page_idx = self
                .pages
                .iter()
                .position(|page| page.get_node(*widget_id).is_some());

            if let Some(from_idx) = from_page_idx {
                if from_idx != *to_page_idx && *to_page_idx < self.pages.len() {
                    // Get the widget's node before removing
                    if let Some(node) = self.pages[from_idx].get_node(*widget_id) {
                        let rect = node.rect;

                        // Remove from source page
                        self.pages[from_idx].remove_widget(*widget_id);

                        // Add to target page (will be updated by moves below)
                        self.pages[*to_page_idx].add_widget(*widget_id, rect);
                    }
                }
            }
        }

        // Apply position/size changes
        for (widget_id, new_rect) in &mutation.moves {
            // Find which page the widget is on
            for page in &mut self.pages {
                if let Some(node) = page.get_node_mut(*widget_id) {
                    node.rect = *new_rect;
                    break;
                }
            }
        }
    }
}

pub trait LayoutSolver {
    fn calculate_drag_preview(
        pages: &[LayoutPage],
        moving_id: WidgetId,
        target_page: Option<usize>,
        target_pos: GridRect,
    ) -> LayoutMutation;

    fn calculate_resize_preview(
        pages: &[LayoutPage],
        resizing_id: WidgetId,
        target_page: Option<usize>,
        new_size: GridRect,
    ) -> LayoutMutation;

    fn compaction_pass(page: &LayoutPage) -> LayoutMutation;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test solver with unimplemented methods for testing layout_manager functions
    struct TestSolver;

    impl LayoutSolver for TestSolver {
        fn calculate_drag_preview(
            _pages: &[LayoutPage],
            _moving_id: WidgetId,
            _target_page: Option<usize>,
            _target_pos: GridRect,
        ) -> LayoutMutation {
            unimplemented!()
        }

        fn calculate_resize_preview(
            _pages: &[LayoutPage],
            _resizing_id: WidgetId,
            _target_page: Option<usize>,
            _new_size: GridRect,
        ) -> LayoutMutation {
            unimplemented!()
        }

        fn compaction_pass(_page: &LayoutPage) -> LayoutMutation {
            unimplemented!()
        }
    }

    #[test]
    fn test_grid_rect_intersects() {
        let rect1 = GridRect {
            col: 0,
            row: 0,
            w: 2,
            h: 2,
        };

        let rect2 = GridRect {
            col: 1,
            row: 1,
            w: 2,
            h: 2,
        };

        let rect3 = GridRect {
            col: 3,
            row: 3,
            w: 1,
            h: 1,
        };

        // rect1 and rect2 overlap
        assert!(rect1.intersects(&rect2));
        assert!(rect2.intersects(&rect1));

        // rect1 and rect3 don't overlap
        assert!(!rect1.intersects(&rect3));
        assert!(!rect3.intersects(&rect1));
    }

    #[test]
    fn test_grid_rect_intersects_edge_cases() {
        // Adjacent rects (touching edges) should not intersect
        let rect1 = GridRect {
            col: 0,
            row: 0,
            w: 2,
            h: 2,
        };

        let rect2 = GridRect {
            col: 2,
            row: 0,
            w: 2,
            h: 2,
        };

        assert!(!rect1.intersects(&rect2));

        // Same rect should intersect with itself
        assert!(rect1.intersects(&rect1));
    }

    #[test]
    fn test_layout_page_add_remove_widget() {
        let mut page = LayoutPage::new(GridConfig {
            cols: 4,
            rows: 4,
            ..Default::default()
        });

        let widget1 = WidgetId(1);
        let rect = GridRect {
            col: 0,
            row: 0,
            w: 2,
            h: 2,
        };

        // Add widget
        page.add_widget(widget1, rect);
        assert!(page.get_node(widget1).is_some());
        assert_eq!(page.get_node(widget1).unwrap().rect, rect);

        // Remove widget
        page.remove_widget(widget1);
        assert!(page.get_node(widget1).is_none());
    }

    #[test]
    fn test_layout_page_is_occupied() {
        let mut page = LayoutPage::new(GridConfig {
            cols: 4,
            rows: 4,
            ..Default::default()
        });

        let widget1 = WidgetId(1);
        page.add_widget(
            widget1,
            GridRect {
                col: 0,
                row: 0,
                w: 2,
                h: 2,
            },
        );

        // Check if a rect that overlaps is occupied
        let occupied = page.is_occupied(GridRect {
            col: 1,
            row: 1,
            w: 1,
            h: 1,
        });
        assert_eq!(occupied, Some(widget1));

        // Check if an unoccupied rect returns None
        let not_occupied = page.is_occupied(GridRect {
            col: 3,
            row: 3,
            w: 1,
            h: 1,
        });
        assert_eq!(not_occupied, None);
    }

    #[test]
    fn test_layout_page_is_occupied_excluding() {
        let mut page = LayoutPage::new(GridConfig {
            cols: 4,
            rows: 4,
            ..Default::default()
        });

        let widget1 = WidgetId(1);
        let widget2 = WidgetId(2);

        page.add_widget(
            widget1,
            GridRect {
                col: 0,
                row: 0,
                w: 2,
                h: 2,
            },
        );

        page.add_widget(
            widget2,
            GridRect {
                col: 2,
                row: 2,
                w: 2,
                h: 2,
            },
        );

        // Check occupation while excluding widget1
        let test_rect = GridRect {
            col: 0,
            row: 0,
            w: 1,
            h: 1,
        };

        // Should find widget1 if we don't exclude it
        assert_eq!(page.is_occupied_excluding(test_rect, None), Some(widget1));

        // Should not find widget1 if we exclude it
        assert_eq!(page.is_occupied_excluding(test_rect, Some(widget1)), None);
    }

    #[test]
    fn test_layout_page_is_in_bounds() {
        let page = LayoutPage::new(GridConfig {
            cols: 4,
            rows: 4,
            ..Default::default()
        });

        // Rect that fits exactly
        assert!(page.is_in_bounds(GridRect {
            col: 0,
            row: 0,
            w: 4,
            h: 4,
        }));

        // Rect at corner
        assert!(page.is_in_bounds(GridRect {
            col: 3,
            row: 3,
            w: 1,
            h: 1,
        }));

        // Out of bounds (width exceeds)
        assert!(!page.is_in_bounds(GridRect {
            col: 0,
            row: 0,
            w: 5,
            h: 4,
        }));

        // Out of bounds (height exceeds)
        assert!(!page.is_in_bounds(GridRect {
            col: 0,
            row: 0,
            w: 4,
            h: 5,
        }));

        // Out of bounds (position + size exceeds)
        assert!(!page.is_in_bounds(GridRect {
            col: 3,
            row: 3,
            w: 2,
            h: 2,
        }));

        // Out of bounds (starting position exceeds)
        assert!(!page.is_in_bounds(GridRect {
            col: 4,
            row: 0,
            w: 1,
            h: 1,
        }));
    }

    #[test]
    fn test_apply_mutation_moves() {
        let page_config = GridConfig {
            cols: 4,
            rows: 4,
            ..Default::default()
        };
        let page = LayoutPage::new(page_config);

        let window_config = WindowConfig {
            width: 400.0,
            height: 400.0,
        };

        let grid_config = GridConfig {
            cols: 4,
            rows: 4,
            padding_percent: 0.0,
            gap_percent: 0.0,
        };

        let mut manager =
            LayoutManager::<TestSolver>::new(vec![page], 0, window_config, grid_config);

        let widget1 = WidgetId(1);
        let widget2 = WidgetId(2);

        manager.pages[0].add_widget(
            widget1,
            GridRect {
                col: 0,
                row: 0,
                w: 1,
                h: 1,
            },
        );

        manager.pages[0].add_widget(
            widget2,
            GridRect {
                col: 1,
                row: 0,
                w: 1,
                h: 1,
            },
        );

        // Create a mutation that moves both widgets
        let mutation = LayoutMutation {
            moves: vec![
                (
                    widget1,
                    GridRect {
                        col: 2,
                        row: 2,
                        w: 1,
                        h: 1,
                    },
                ),
                (
                    widget2,
                    GridRect {
                        col: 3,
                        row: 3,
                        w: 1,
                        h: 1,
                    },
                ),
            ],
            page_changes: vec![],
        };

        manager.apply_mutation(mutation);

        // Verify the moves were applied
        let node1 = manager.pages[0].get_node(widget1).unwrap();
        assert_eq!(node1.rect.col, 2);
        assert_eq!(node1.rect.row, 2);
        assert_eq!(node1.rect.w, 1);
        assert_eq!(node1.rect.h, 1);

        let node2 = manager.pages[0].get_node(widget2).unwrap();
        assert_eq!(node2.rect.col, 3);
        assert_eq!(node2.rect.row, 3);
        assert_eq!(node2.rect.w, 1);
        assert_eq!(node2.rect.h, 1);
    }

    #[test]
    fn test_apply_mutation_page_transfer() {
        let page_config = GridConfig {
            cols: 4,
            rows: 4,
            ..Default::default()
        };
        let page1 = LayoutPage::new(page_config.clone());
        let page2 = LayoutPage::new(page_config);

        let window_config = WindowConfig {
            width: 400.0,
            height: 400.0,
        };

        let grid_config = GridConfig {
            cols: 4,
            rows: 4,
            padding_percent: 0.0,
            gap_percent: 0.0,
        };

        let mut manager =
            LayoutManager::<TestSolver>::new(vec![page1, page2], 0, window_config, grid_config);

        let widget_id = WidgetId(1);
        manager.pages[0].add_widget(
            widget_id,
            GridRect {
                col: 0,
                row: 0,
                w: 1,
                h: 1,
            },
        );

        // Create a mutation that transfers widget to page 1
        let mutation = LayoutMutation {
            moves: vec![(
                widget_id,
                GridRect {
                    col: 1,
                    row: 1,
                    w: 1,
                    h: 1,
                },
            )],
            page_changes: vec![(widget_id, 1)],
        };

        manager.apply_mutation(mutation);

        // Verify widget is no longer on page 0
        assert!(manager.pages[0].get_node(widget_id).is_none());

        // Verify widget is on page 1 with correct position
        let node = manager.pages[1].get_node(widget_id).unwrap();
        assert_eq!(node.rect.col, 1);
        assert_eq!(node.rect.row, 1);
        assert_eq!(node.rect.w, 1);
        assert_eq!(node.rect.h, 1);
    }

    #[test]
    fn test_apply_mutation_with_both_moves_and_transfers() {
        let page_config = GridConfig {
            cols: 4,
            rows: 4,
            ..Default::default()
        };
        let page1 = LayoutPage::new(page_config.clone());
        let page2 = LayoutPage::new(page_config);

        let window_config = WindowConfig {
            width: 400.0,
            height: 400.0,
        };

        let grid_config = GridConfig {
            cols: 4,
            rows: 4,
            padding_percent: 0.0,
            gap_percent: 0.0,
        };

        let mut manager =
            LayoutManager::<TestSolver>::new(vec![page1, page2], 0, window_config, grid_config);

        let widget1 = WidgetId(1);
        let widget2 = WidgetId(2);

        manager.pages[0].add_widget(
            widget1,
            GridRect {
                col: 0,
                row: 0,
                w: 1,
                h: 1,
            },
        );

        manager.pages[0].add_widget(
            widget2,
            GridRect {
                col: 1,
                row: 0,
                w: 1,
                h: 1,
            },
        );

        // Transfer widget1 to page 1 and move both widgets
        let mutation = LayoutMutation {
            moves: vec![
                (
                    widget1,
                    GridRect {
                        col: 2,
                        row: 2,
                        w: 1,
                        h: 1,
                    },
                ),
                (
                    widget2,
                    GridRect {
                        col: 3,
                        row: 3,
                        w: 1,
                        h: 1,
                    },
                ),
            ],
            page_changes: vec![(widget1, 1)],
        };

        manager.apply_mutation(mutation);

        // Verify widget1 is on page 1 with correct position
        assert!(manager.pages[0].get_node(widget1).is_none());
        let node1 = manager.pages[1].get_node(widget1).unwrap();
        assert_eq!(node1.rect.col, 2);
        assert_eq!(node1.rect.row, 2);

        // Verify widget2 is still on page 0 with updated position
        let node2 = manager.pages[0].get_node(widget2).unwrap();
        assert_eq!(node2.rect.col, 3);
        assert_eq!(node2.rect.row, 3);
    }

    #[test]
    fn test_layout_manager_basic_setup() {
        let page_config = GridConfig {
            cols: 4,
            rows: 4,
            ..Default::default()
        };
        let page = LayoutPage::new(page_config);

        let window_config = WindowConfig {
            width: 400.0,
            height: 400.0,
        };

        let grid_config = GridConfig {
            cols: 4,
            rows: 4,
            padding_percent: 0.0,
            gap_percent: 0.0,
        };

        let manager = LayoutManager::<TestSolver>::new(vec![page], 0, window_config, grid_config);

        assert_eq!(manager.pages.len(), 1);
        assert_eq!(manager.active_page, 0);
        assert_eq!(manager.window_config.width, 400.0);
        assert_eq!(manager.grid_config.cols, 4);
    }

    #[test]
    fn test_set_active_page() {
        let page_config = GridConfig {
            cols: 4,
            rows: 4,
            ..Default::default()
        };
        let page1 = LayoutPage::new(page_config.clone());
        let page2 = LayoutPage::new(page_config);

        let window_config = WindowConfig {
            width: 400.0,
            height: 400.0,
        };

        let grid_config = GridConfig {
            cols: 4,
            rows: 4,
            padding_percent: 0.0,
            gap_percent: 0.0,
        };

        let mut manager =
            LayoutManager::<TestSolver>::new(vec![page1, page2], 0, window_config, grid_config);

        assert_eq!(manager.active_page, 0);

        manager.set_active_page(1);
        assert_eq!(manager.active_page, 1);
    }
}
