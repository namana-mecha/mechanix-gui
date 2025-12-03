use std::collections::HashMap;

use crate::layout_manager::{GridRect, LayoutMutation, LayoutPage, LayoutSolver, WidgetId};

/// Android-like layout solver that handles collision detection and widget movement
#[derive(Debug, Clone, Copy)]
pub struct AndroidLayoutSolver;

impl AndroidLayoutSolver {
    /// Find all widgets that collide with the given rect
    fn find_collisions(page: &LayoutPage, rect: GridRect, ignore_id: Option<WidgetId>) -> Vec<WidgetId> {
        page.nodes
            .values()
            .filter(|node| {
                if let Some(id) = ignore_id {
                    if node.widget_id == id {
                        return false;
                    }
                }
                node.rect.intersects(&rect)
            })
            .map(|node| node.widget_id)
            .collect()
    }

    /// Calculate distance between two grid positions (Manhattan distance)
    fn calculate_distance(rect1: GridRect, rect2: GridRect) -> usize {
        let col_diff = if rect1.col > rect2.col {
            rect1.col - rect2.col
        } else {
            rect2.col - rect1.col
        };
        let row_diff = if rect1.row > rect2.row {
            rect1.row - rect2.row
        } else {
            rect2.row - rect1.row
        };
        col_diff + row_diff
    }

    /// Find the closest available position for a widget, preferring positions near the original
    fn find_closest_available_position(
        page: &LayoutPage,
        widget_rect: GridRect,
        original_rect: GridRect,
        occupied: &HashMap<WidgetId, GridRect>,
        widget_id: WidgetId,
    ) -> Option<GridRect> {
        let mut best_position = None;
        let mut best_distance = usize::MAX;

        // Try all possible positions, row by row, column by column
        for row in 0..page.config.rows {
            for col in 0..page.config.cols {
                let test_rect = GridRect {
                    col,
                    row,
                    w: widget_rect.w,
                    h: widget_rect.h,
                };

                // Check if this position is in bounds
                if !page.is_in_bounds(test_rect) {
                    continue;
                }

                // Check if this position collides with any occupied widget
                let mut has_collision = false;
                for (other_id, other_rect) in occupied {
                    if *other_id == widget_id {
                        continue;
                    }
                    if test_rect.intersects(other_rect) {
                        has_collision = true;
                        break;
                    }
                }

                if !has_collision {
                    let distance = Self::calculate_distance(test_rect, original_rect);
                    if distance < best_distance {
                        best_distance = distance;
                        best_position = Some(test_rect);
                    }
                }
            }
        }

        best_position
    }

    /// Resolve collisions by removing overlapping widgets and finding new positions for them
    fn resolve_collisions(
        page: &LayoutPage,
        widget_id: WidgetId,
        new_rect: GridRect,
    ) -> Option<HashMap<WidgetId, GridRect>> {
        let mut moves = HashMap::new();

        // Find widgets that would collide with the new position
        let collisions = Self::find_collisions(page, new_rect, Some(widget_id));

        // Start with the moved widget in place
        moves.insert(widget_id, new_rect);

        // Track which positions are occupied (moved widget + non-colliding widgets)
        let mut occupied = HashMap::new();
        occupied.insert(widget_id, new_rect);

        // Add all non-colliding widgets to occupied positions
        for node in page.nodes.values() {
            if node.widget_id != widget_id && !collisions.contains(&node.widget_id) {
                occupied.insert(node.widget_id, node.rect);
            }
        }

        // Try to find new positions for each colliding widget
        for collision_id in collisions {
            if let Some(node) = page.get_node(collision_id) {
                let original_rect = node.rect;

                // Find the closest available position
                if let Some(new_position) = Self::find_closest_available_position(
                    page,
                    original_rect,
                    original_rect,
                    &occupied,
                    collision_id,
                ) {
                    moves.insert(collision_id, new_position);
                    occupied.insert(collision_id, new_position);
                } else {
                    // Can't find a position for this widget, so the move is invalid
                    return None;
                }
            }
        }

        Some(moves)
    }

    /// Compact widgets upward to fill gaps
    fn compact_upward(page: &LayoutPage, moves: &mut HashMap<WidgetId, GridRect>) {
        // Sort widgets by row, then by column (top-left to bottom-right)
        let mut sorted_widgets: Vec<_> = page.nodes.values().collect();
        sorted_widgets.sort_by_key(|node| (node.rect.row, node.rect.col));

        for node in sorted_widgets {
            let widget_id = node.widget_id;
            let current_rect = moves.get(&widget_id).copied().unwrap_or(node.rect);

            // Try to move up as much as possible
            let mut target_row = 0;

            // Find the highest position this widget can occupy
            while target_row < current_rect.row {
                let test_rect = GridRect {
                    col: current_rect.col,
                    row: target_row,
                    w: current_rect.w,
                    h: current_rect.h,
                };

                // Check if this position collides with any other widget
                let mut has_collision = false;
                for other_node in page.nodes.values() {
                    if other_node.widget_id == widget_id {
                        continue;
                    }

                    let other_rect = moves.get(&other_node.widget_id).copied().unwrap_or(other_node.rect);
                    if test_rect.intersects(&other_rect) {
                        has_collision = true;
                        // Move below this widget
                        target_row = other_rect.row + other_rect.h;
                        break;
                    }
                }

                if !has_collision {
                    break;
                }
            }

            // If we can move up, record the move
            if target_row < current_rect.row {
                moves.insert(
                    widget_id,
                    GridRect {
                        col: current_rect.col,
                        row: target_row,
                        w: current_rect.w,
                        h: current_rect.h,
                    },
                );
            }
        }
    }
}

impl LayoutSolver for AndroidLayoutSolver {
    fn calculate_drag_preview(
        pages: &[LayoutPage],
        moving_id: WidgetId,
        target_page: Option<usize>,
        target_pos: GridRect,
    ) -> LayoutMutation {
        let mut mutation = LayoutMutation::default();

        // Determine which page we're working with
        let page_idx = target_page.unwrap_or(0);
        if page_idx >= pages.len() {
            return mutation;
        }

        let page = &pages[page_idx];

        // Check if the target position is within bounds
        if !page.is_in_bounds(target_pos) {
            return mutation;
        }

        // Resolve collisions and find positions for displaced widgets
        if let Some(moves) = Self::resolve_collisions(page, moving_id, target_pos) {
            // Convert moves to mutation format
            mutation.moves = moves.into_iter().collect();

            // If moving to a different page, record the page change
            if let Some(target_page_idx) = target_page {
                mutation.page_changes.push((moving_id, target_page_idx));
            }
        }
        // If resolve_collisions returns None, the move is invalid, return empty mutation

        mutation
    }

    fn calculate_resize_preview(
        pages: &[LayoutPage],
        resizing_id: WidgetId,
        target_page: Option<usize>,
        new_size: GridRect,
    ) -> LayoutMutation {
        let mut mutation = LayoutMutation::default();

        // Determine which page we're working with
        let page_idx = target_page.unwrap_or(0);
        if page_idx >= pages.len() {
            return mutation;
        }

        let page = &pages[page_idx];

        // Check if the new size is within bounds
        if !page.is_in_bounds(new_size) {
            return mutation;
        }

        // Resolve collisions and find positions for displaced widgets
        if let Some(moves) = Self::resolve_collisions(page, resizing_id, new_size) {
            // Convert moves to mutation format
            mutation.moves = moves.into_iter().collect();
        }
        // If resolve_collisions returns None, the resize is invalid, return empty mutation

        mutation
    }

    fn compaction_pass(page: &LayoutPage) -> LayoutMutation {
        let mut mutation = LayoutMutation::default();
        let mut moves = HashMap::new();

        // Compact all widgets upward
        Self::compact_upward(page, &mut moves);

        // Convert moves to mutation format, filtering out widgets that didn't actually move
        mutation.moves = moves
            .into_iter()
            .filter(|(id, rect)| {
                if let Some(node) = page.get_node(*id) {
                    node.rect != *rect
                } else {
                    false
                }
            })
            .collect();

        mutation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GridConfig;
    use crate::layout_manager::LayoutPage;

    fn create_test_page() -> LayoutPage {
        LayoutPage::new(GridConfig { cols: 4, rows: 6, ..Default::default() })
    }

    #[test]
    fn test_drag_no_collision() {
        let mut page = create_test_page();

        // Place a widget at (0,0) size 1x1
        let widget1 = WidgetId(1);
        page.add_widget(widget1, GridRect { col: 0, row: 0, w: 1, h: 1 });

        // Drag it to (2,2) - no collisions
        let target = GridRect { col: 2, row: 2, w: 1, h: 1 };
        let mutation = AndroidLayoutSolver::calculate_drag_preview(&[page], widget1, Some(0), target);

        // Should move widget1 to (2,2)
        assert_eq!(mutation.moves.len(), 1);
        assert!(mutation.moves.iter().any(|(id, rect)| *id == widget1 && *rect == target));
    }

    #[test]
    fn test_drag_with_collision_pushes_down() {
        let mut page = create_test_page();

        // Place widget1 at (0,0) size 2x2
        let widget1 = WidgetId(1);
        page.add_widget(widget1, GridRect { col: 0, row: 0, w: 2, h: 2 });

        // Place widget2 at (0,2) size 2x2
        let widget2 = WidgetId(2);
        page.add_widget(widget2, GridRect { col: 0, row: 2, w: 2, h: 2 });

        // Drag widget1 to (0,1) - should push widget2 down
        let target = GridRect { col: 0, row: 1, w: 2, h: 2 };
        let mutation = AndroidLayoutSolver::calculate_drag_preview(&[page], widget1, Some(0), target);

        // widget1 should move to (0,1)
        assert!(mutation.moves.iter().any(|(id, rect)| *id == widget1 && rect.row == 1));

        // widget2 should be pushed down to (0,3)
        assert!(mutation.moves.iter().any(|(id, rect)| *id == widget2 && rect.row == 3));
    }

    #[test]
    fn test_drag_chain_collision() {
        let mut page = create_test_page();

        // Create a vertical chain of widgets
        let widget1 = WidgetId(1);
        page.add_widget(widget1, GridRect { col: 0, row: 0, w: 1, h: 1 });

        let widget2 = WidgetId(2);
        page.add_widget(widget2, GridRect { col: 0, row: 1, w: 1, h: 1 });

        let widget3 = WidgetId(3);
        page.add_widget(widget3, GridRect { col: 0, row: 2, w: 1, h: 1 });

        // Drag a new widget to (0,1) - should push widget2 and widget3 down
        let new_widget = WidgetId(4);
        let target = GridRect { col: 0, row: 1, w: 1, h: 1 };
        let mutation = AndroidLayoutSolver::calculate_drag_preview(&[page], new_widget, Some(0), target);

        // new_widget should be at (0,1)
        assert!(mutation.moves.iter().any(|(id, rect)| *id == new_widget && rect.row == 1));

        // widget2 should be pushed to (0,2)
        assert!(mutation.moves.iter().any(|(id, rect)| *id == widget2 && rect.row == 2));

        // widget3 should be pushed to (0,3)
        assert!(mutation.moves.iter().any(|(id, rect)| *id == widget3 && rect.row == 3));
    }

    #[test]
    fn test_resize_pushes_widgets() {
        let mut page = create_test_page();

        // Place widget1 at (0,0) size 1x1
        let widget1 = WidgetId(1);
        page.add_widget(widget1, GridRect { col: 0, row: 0, w: 1, h: 1 });

        // Place widget2 at (0,1) size 1x1
        let widget2 = WidgetId(2);
        page.add_widget(widget2, GridRect { col: 0, row: 1, w: 1, h: 1 });

        // Resize widget1 to 1x2 - should push widget2 down
        let new_size = GridRect { col: 0, row: 0, w: 1, h: 2 };
        let mutation = AndroidLayoutSolver::calculate_resize_preview(&[page], widget1, Some(0), new_size);

        // widget1 should be at (0,0) with new size
        assert!(mutation.moves.iter().any(|(id, rect)| *id == widget1 && rect.h == 2));

        // widget2 should be pushed to (0,2)
        assert!(mutation.moves.iter().any(|(id, rect)| *id == widget2 && rect.row == 2));
    }

    #[test]
    fn test_compaction_moves_widgets_up() {
        let mut page = create_test_page();

        // Place widget1 at (0,0) size 1x1
        let widget1 = WidgetId(1);
        page.add_widget(widget1, GridRect { col: 0, row: 0, w: 1, h: 1 });

        // Place widget2 at (0,3) size 1x1 - there's a gap
        let widget2 = WidgetId(2);
        page.add_widget(widget2, GridRect { col: 0, row: 3, w: 1, h: 1 });

        let mutation = AndroidLayoutSolver::compaction_pass(&page);

        // widget2 should be moved up to (0,1)
        assert!(mutation.moves.iter().any(|(id, rect)| *id == widget2 && rect.row == 1));

        // widget1 should not move
        assert!(!mutation.moves.iter().any(|(id, _)| *id == widget1));
    }

    #[test]
    fn test_compaction_with_multiple_gaps() {
        let mut page = create_test_page();

        // Place widgets with gaps
        let widget1 = WidgetId(1);
        page.add_widget(widget1, GridRect { col: 0, row: 0, w: 1, h: 1 });

        let widget2 = WidgetId(2);
        page.add_widget(widget2, GridRect { col: 1, row: 2, w: 1, h: 1 });

        let widget3 = WidgetId(3);
        page.add_widget(widget3, GridRect { col: 0, row: 4, w: 1, h: 1 });

        let mutation = AndroidLayoutSolver::compaction_pass(&page);

        // widget2 should move to row 0 (different column)
        assert!(mutation.moves.iter().any(|(id, rect)| *id == widget2 && rect.row == 0));

        // widget3 should move up to row 1
        assert!(mutation.moves.iter().any(|(id, rect)| *id == widget3 && rect.row == 1));
    }

    #[test]
    fn test_compaction_respects_stacking() {
        let mut page = create_test_page();

        // Create a proper stack with no gaps
        let widget1 = WidgetId(1);
        page.add_widget(widget1, GridRect { col: 0, row: 0, w: 2, h: 2 });

        let widget2 = WidgetId(2);
        page.add_widget(widget2, GridRect { col: 0, row: 2, w: 2, h: 1 });

        let mutation = AndroidLayoutSolver::compaction_pass(&page);

        // No widgets should move - already compacted
        assert_eq!(mutation.moves.len(), 0);
    }

    #[test]
    fn test_drag_out_of_bounds() {
        let mut page = create_test_page();

        let widget1 = WidgetId(1);
        page.add_widget(widget1, GridRect { col: 0, row: 0, w: 1, h: 1 });

        // Try to drag beyond page bounds
        let target = GridRect { col: 5, row: 0, w: 1, h: 1 };
        let mutation = AndroidLayoutSolver::calculate_drag_preview(&[page], widget1, Some(0), target);

        // Should return empty mutation
        assert_eq!(mutation.moves.len(), 0);
    }

    #[test]
    fn test_resize_out_of_bounds() {
        let mut page = create_test_page();

        let widget1 = WidgetId(1);
        page.add_widget(widget1, GridRect { col: 0, row: 0, w: 1, h: 1 });

        // Try to resize beyond page bounds
        let new_size = GridRect { col: 0, row: 0, w: 5, h: 1 };
        let mutation = AndroidLayoutSolver::calculate_resize_preview(&[page], widget1, Some(0), new_size);

        // Should return empty mutation
        assert_eq!(mutation.moves.len(), 0);
    }

    #[test]
    fn test_drag_with_different_sized_widgets() {
        let mut page = create_test_page();

        // Large widget at top
        let widget1 = WidgetId(1);
        page.add_widget(widget1, GridRect { col: 0, row: 0, w: 2, h: 2 });

        // Small widget below
        let widget2 = WidgetId(2);
        page.add_widget(widget2, GridRect { col: 2, row: 2, w: 1, h: 1 });

        // Drag small widget into large widget's space
        let target = GridRect { col: 0, row: 1, w: 1, h: 1 };
        let mutation = AndroidLayoutSolver::calculate_drag_preview(&[page], widget2, Some(0), target);

        // widget2 should be at target position
        assert!(mutation.moves.iter().any(|(id, rect)| *id == widget2 && *rect == target));

        // widget1 should be pushed down
        assert!(mutation.moves.iter().any(|(id, rect)| *id == widget1 && rect.row >= 1));
    }

    #[test]
    fn test_page_change_tracked() {
        let mut page1 = create_test_page();
        let page2 = create_test_page();

        let widget1 = WidgetId(1);
        page1.add_widget(widget1, GridRect { col: 0, row: 0, w: 1, h: 1 });

        let target = GridRect { col: 1, row: 1, w: 1, h: 1 };
        let mutation = AndroidLayoutSolver::calculate_drag_preview(&[page1, page2], widget1, Some(1), target);

        // Should track page change
        assert_eq!(mutation.page_changes.len(), 1);
        assert_eq!(mutation.page_changes[0], (widget1, 1));
    }

    #[test]
    fn test_large_widget_drag_with_compaction() {
        // Create a 4x6 grid to have enough space
        let mut page = LayoutPage::new(GridConfig { cols: 4, rows: 6, ..Default::default() });

        // Large 4x3 widget at row 1 (occupies rows 1-3, full width)
        let large_widget = WidgetId(1);
        page.add_widget(large_widget, GridRect { col: 0, row: 1, w: 4, h: 3 });

        // Small 1x1 widget at row 4
        let small_widget = WidgetId(2);
        page.add_widget(small_widget, GridRect { col: 0, row: 4, w: 1, h: 1 });

        // Move large widget from row 1 to row 2 (will occupy rows 2-4)
        let target = GridRect { col: 0, row: 2, w: 4, h: 3 };
        let drag_mutation = AndroidLayoutSolver::calculate_drag_preview(
            &[page.clone()],
            large_widget,
            Some(0),
            target,
        );

        // Apply the drag mutation to a new page state
        let mut updated_page = page.clone();
        for (widget_id, new_rect) in &drag_mutation.moves {
            if let Some(node) = updated_page.get_node_mut(*widget_id) {
                node.rect = *new_rect;
            }
        }

        // Verify the drag behavior
        // The large widget should be at row 2
        assert!(
            drag_mutation.moves.iter().any(|(id, rect)| *id == large_widget && rect.row == 2),
            "Large widget should move to row 2"
        );

        // The small widget should be pushed down to row 5
        assert!(
            drag_mutation.moves.iter().any(|(id, rect)| *id == small_widget && rect.row == 5),
            "Small widget should be pushed to row 5"
        );

        // Now run compaction to fill gaps
        let compaction_mutation = AndroidLayoutSolver::compaction_pass(&updated_page);

        // After compaction, both widgets should move up to fill gaps
        // In Android-style compaction, widgets are processed top-to-bottom,
        // so the large widget (currently at row 2) compacts first to row 0,
        // and the small widget compacts to row 3 (first available row after large widget)
        assert!(
            compaction_mutation.moves.iter().any(|(id, rect)| *id == large_widget && rect.row == 0),
            "Large widget should compact to row 0, got moves: {:?}",
            compaction_mutation.moves
        );

        assert!(
            compaction_mutation.moves.iter().any(|(id, rect)| *id == small_widget && rect.row == 3),
            "Small widget should compact to row 3 (after large widget), got moves: {:?}",
            compaction_mutation.moves
        );

        // Total layout height reduced from 6 (row 5 + 1) to 4 (row 3 + 1)
        assert_eq!(compaction_mutation.moves.len(), 2, "Both widgets should move during compaction");
    }

    #[test]
    fn test_full_width_widget_finds_space_for_displaced_widgets() {
        // Create a 4x4 grid - no extra rows, widgets must find space within bounds
        let mut page = LayoutPage::new(GridConfig { cols: 4, rows: 4, ..Default::default() });

        // Large 4x2 widget spanning rows 1-2 (the middle)
        let large_widget = WidgetId(1);
        page.add_widget(large_widget, GridRect { col: 0, row: 1, w: 4, h: 2 });

        // Two 1x1 widgets at the bottom row (row 3)
        let small_widget1 = WidgetId(2);
        page.add_widget(small_widget1, GridRect { col: 0, row: 3, w: 1, h: 1 });

        let small_widget2 = WidgetId(3);
        page.add_widget(small_widget2, GridRect { col: 1, row: 3, w: 1, h: 1 });

        // Move large widget down to span rows 2-3
        let target = GridRect { col: 0, row: 2, w: 4, h: 2 };
        let mutation = AndroidLayoutSolver::calculate_drag_preview(&[page.clone()], large_widget, Some(0), target);

        // The large widget should move to row 2
        assert!(
            mutation.moves.iter().any(|(id, rect)| *id == large_widget && rect.row == 2),
            "Large widget should move to row 2, got moves: {:?}",
            mutation.moves
        );

        // Small widgets can't be pushed to row 4 (out of bounds in 4x4 grid)
        // They should be placed in available space at rows 0-1
        assert!(
            mutation.moves.iter().any(|(id, rect)| *id == small_widget1 && rect.row <= 1),
            "Small widget 1 should be moved to available space in top rows, got moves: {:?}",
            mutation.moves
        );

        assert!(
            mutation.moves.iter().any(|(id, rect)| *id == small_widget2 && rect.row <= 1),
            "Small widget 2 should be moved to available space in top rows, got moves: {:?}",
            mutation.moves
        );

        // All three widgets should have moves recorded
        assert_eq!(
            mutation.moves.len(),
            3,
            "All three widgets (1 large, 2 small) should move"
        );

        // Verify no widgets went out of bounds
        for (widget_id, rect) in &mutation.moves {
            assert!(
                page.is_in_bounds(*rect),
                "Widget {:?} moved to out-of-bounds position: {:?}",
                widget_id,
                rect
            );
        }
    }

    #[test]
    fn test_cross_page_drag_with_rearrangement() {
        // Create two pages with 4x4 grid
        let mut page1 = LayoutPage::new(GridConfig { cols: 4, rows: 4, ..Default::default() });
        let mut page2 = LayoutPage::new(GridConfig { cols: 4, rows: 4, ..Default::default() });

        // Page 1: Large widget that we'll drag to page 2
        let large_widget = WidgetId(1);
        page1.add_widget(large_widget, GridRect { col: 0, row: 0, w: 4, h: 2 });

        // Page 2: Several small widgets that occupy space but can be rearranged
        let widget_a = WidgetId(2);
        page2.add_widget(widget_a, GridRect { col: 0, row: 1, w: 2, h: 1 });

        let widget_b = WidgetId(3);
        page2.add_widget(widget_b, GridRect { col: 2, row: 1, w: 1, h: 1 });

        let widget_c = WidgetId(4);
        page2.add_widget(widget_c, GridRect { col: 0, row: 2, w: 2, h: 1 });

        let widget_d = WidgetId(5);
        page2.add_widget(widget_d, GridRect { col: 3, row: 2, w: 1, h: 1 });

        // Drag the large widget from page 1 to page 2 at position (0, 0)
        let target = GridRect { col: 0, row: 0, w: 4, h: 2 };
        let mutation = AndroidLayoutSolver::calculate_drag_preview(
            &[page1.clone(), page2.clone()],
            large_widget,
            Some(1), // Moving to page 2
            target,
        );

        // The large widget should move to (0, 0) on page 2
        assert!(
            mutation.moves.iter().any(|(id, rect)| *id == large_widget && rect.row == 0 && rect.col == 0),
            "Large widget should move to target position, got moves: {:?}",
            mutation.moves
        );

        // Widgets on page 2 that would collide should be rearranged
        // Widget A at (0, 1) would collide with large widget at rows 0-1
        assert!(
            mutation.moves.iter().any(|(id, _)| *id == widget_a),
            "Widget A should be moved to avoid collision, got moves: {:?}",
            mutation.moves
        );

        // Widget B at (2, 1) would collide with large widget
        assert!(
            mutation.moves.iter().any(|(id, _)| *id == widget_b),
            "Widget B should be moved to avoid collision, got moves: {:?}",
            mutation.moves
        );

        // Widget C at (0, 2) should stay (below the large widget)
        // Widget D at (3, 2) should stay (doesn't collide)

        // Verify all widgets stay within bounds
        for (widget_id, rect) in &mutation.moves {
            assert!(
                page2.is_in_bounds(*rect),
                "Widget {:?} moved to out-of-bounds position: {:?}",
                widget_id,
                rect
            );
        }

        // Verify the page change is tracked
        assert_eq!(
            mutation.page_changes.len(),
            1,
            "Should track page change for the dragged widget"
        );
        assert_eq!(
            mutation.page_changes[0],
            (large_widget, 1),
            "Should move large widget to page 1 (index)"
        );

        // Build the final layout to verify no overlaps
        let mut final_positions = std::collections::HashMap::new();

        // Add widgets from page 2 with their updated positions
        for node in page2.nodes.values() {
            let final_rect = mutation.moves.iter()
                .find(|(id, _)| *id == node.widget_id)
                .map(|(_, rect)| *rect)
                .unwrap_or(node.rect);
            final_positions.insert(node.widget_id, final_rect);
        }

        // Add the large widget at its new position
        if let Some((_, rect)) = mutation.moves.iter().find(|(id, _)| *id == large_widget) {
            final_positions.insert(large_widget, *rect);
        }

        // Verify no overlaps in final layout
        let positions: Vec<_> = final_positions.iter().collect();
        for i in 0..positions.len() {
            for j in (i + 1)..positions.len() {
                let (id1, rect1) = positions[i];
                let (id2, rect2) = positions[j];
                assert!(
                    !rect1.intersects(rect2),
                    "Widgets {:?} and {:?} overlap: {:?} and {:?}",
                    id1, id2, rect1, rect2
                );
            }
        }
    }

    // TODO: Re-enable when gpui_macros recursion/stack overflow issue is fixed
    // #[test]
    // fn test_2x2_widget_displaces_four_1x1_widgets() {
    //     // Create a 4x4 grid
    //     let mut page = LayoutPage::new(GridConfig { cols: 4, rows: 4, ..Default::default() });
    //
    //     // Place a 2x2 widget in the top left (occupies (0,0) to (1,1))
    //     let widget_2x2 = WidgetId(1);
    //     page.add_widget(widget_2x2, GridRect { col: 0, row: 0, w: 2, h: 2 });
    //
    //     // Place four 1x1 widgets in the top right 2x2 area
    //     let widget_1 = WidgetId(2);
    //     page.add_widget(widget_1, GridRect { col: 2, row: 0, w: 1, h: 1 });
    //
    //     let widget_2 = WidgetId(3);
    //     page.add_widget(widget_2, GridRect { col: 3, row: 0, w: 1, h: 1 });
    //
    //     let widget_3 = WidgetId(4);
    //     page.add_widget(widget_3, GridRect { col: 2, row: 1, w: 1, h: 1 });
    //
    //     let widget_4 = WidgetId(5);
    //     page.add_widget(widget_4, GridRect { col: 3, row: 1, w: 1, h: 1 });
    //
    //     // Initial layout:
    //     // [2x2][2x2][1x1][1x1]  <- row 0
    //     // [2x2][2x2][1x1][1x1]  <- row 1
    //     // [   ][   ][   ][   ]  <- row 2
    //     // [   ][   ][   ][   ]  <- row 3
    //
    //     // Move the 2x2 widget onto the four 1x1 widgets (to position (2,0))
    //     let target = GridRect { col: 2, row: 0, w: 2, h: 2 };
    //     let mutation = AndroidLayoutSolver::calculate_drag_preview(&[page.clone()], widget_2x2, Some(0), target);
    //
    //     // Expected layout after move:
    //     // [   ][   ][2x2][2x2]  <- row 0
    //     // [   ][   ][2x2][2x2]  <- row 1
    //     // [1x1][1x1][1x1][1x1]  <- row 2 (all four 1x1 widgets displaced here or below)
    //     // [   ][   ][   ][   ]  <- row 3
    //
    //     // The 2x2 widget should move to target position (2,0)
    //     assert!(
    //         mutation.moves.iter().any(|(id, rect)| *id == widget_2x2 && rect.col == 2 && rect.row == 0),
    //         "2x2 widget should move to position (2,0), got moves: {:?}",
    //         mutation.moves
    //     );
    //
    //     // All four 1x1 widgets should be displaced to new positions
    //     let small_widgets = vec![widget_1, widget_2, widget_3, widget_4];
    //     for &small_widget in &small_widgets {
    //         let moved = mutation.moves.iter().find(|(id, _)| *id == small_widget);
    //         assert!(
    //             moved.is_some(),
    //             "Widget {:?} should be displaced to a new location, got moves: {:?}",
    //             small_widget,
    //             mutation.moves
    //         );
    //
    //         // Verify the widget moved to a different position than its original
    //         if let Some((_, new_rect)) = moved {
    //             let original_rect = page.get_node(small_widget).unwrap().rect;
    //             assert_ne!(
    //                 *new_rect, original_rect,
    //                 "Widget {:?} should move to a different position. Original: {:?}, New: {:?}",
    //                 small_widget, original_rect, new_rect
    //             );
    //         }
    //     }
    //
    //     // Verify all 5 widgets have valid moves (1 2x2 + 4 1x1s)
    //     assert_eq!(
    //         mutation.moves.len(),
    //         5,
    //         "All 5 widgets should have moves recorded (1 2x2 + 4 1x1s), got {} moves",
    //         mutation.moves.len()
    //     );
    //
    //     // Verify all widgets stay within bounds
    //     for (widget_id, rect) in &mutation.moves {
    //         assert!(
    //             page.is_in_bounds(*rect),
    //             "Widget {:?} moved to out-of-bounds position: {:?}",
    //             widget_id,
    //             rect
    //         );
    //     }
    //
    //     // Build final layout and verify no overlaps
    //     let mut final_positions = std::collections::HashMap::new();
    //     for node in page.nodes.values() {
    //         let final_rect = mutation.moves.iter()
    //             .find(|(id, _)| *id == node.widget_id)
    //             .map(|(_, rect)| *rect)
    //             .unwrap_or(node.rect);
    //         final_positions.insert(node.widget_id, final_rect);
    //     }
    //
    //     // Verify no overlaps in final layout
    //     let positions: Vec<_> = final_positions.iter().collect();
    //     for i in 0..positions.len() {
    //         for j in (i + 1)..positions.len() {
    //             let (id1, rect1) = positions[i];
    //             let (id2, rect2) = positions[j];
    //             assert!(
    //                 !rect1.intersects(rect2),
    //                 "Widgets {:?} and {:?} overlap after move: {:?} and {:?}",
    //                 id1, id2, rect1, rect2
    //             );
    //         }
    //     }
    // }
}
