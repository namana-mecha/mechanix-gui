use crate::config::HomescreenConfig;
use crate::models::HomescreenState;
use commons::widgets::wing;
use gpui::*;

/// A positioned widget container that wraps a widget with a wing border
/// and positions it absolutely at the specified bounds.
#[derive(IntoElement)]
struct PositionedWidget {
    bounds: Bounds<Pixels>,
    widget: AnyElement,
}

impl PositionedWidget {
    fn new(bounds: Bounds<Pixels>, widget: AnyElement) -> Self {
        Self { bounds, widget }
    }
}

impl RenderOnce for PositionedWidget {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut w = wing();
        w.border_radius(px(12.0));
        w.absolute()
            .left(self.bounds.origin.x)
            .top(self.bounds.origin.y)
            .w(self.bounds.size.width)
            .h(self.bounds.size.height)
            .child(self.widget)
    }
}

#[derive(Clone, Copy, Debug)]
enum DragState {
    Idle,
    /// Holding down on a potential widget, waiting for hold duration
    Holding {
        widget_id: crate::layout_manager::WidgetId,
        start_pos: Point<Pixels>,
        start_time: std::time::Instant,
    },
    /// Dragging a widget
    DraggingWidget {
        widget_id: crate::layout_manager::WidgetId,
        start_pos: Point<Pixels>,
        current_pos: Point<Pixels>,
        original_bounds: Bounds<Pixels>,
    },
    /// Dragging to switch pages
    DraggingPage { start_x: f32, current_offset: f32 },
}

pub struct Homescreen {
    state: HomescreenState,
    config: HomescreenConfig,
    drag_state: DragState,
    drag_offset: f32,         // Current offset during drag
    target_offset: f32,       // Target offset for animation (always 0.0 when at rest)
    animation_trigger: usize, // Increment to trigger new animation

    // Widget return animation state (supports multiple widgets animating simultaneously)
    widget_animations: Vec<WidgetAnimationState>,
}

#[derive(Clone, Copy, Debug)]
struct WidgetAnimationState {
    widget_id: crate::layout_manager::WidgetId,
    start_bounds: Bounds<Pixels>,
    target_bounds: Bounds<Pixels>,
    trigger: usize,
    start_time: std::time::Instant,
}

impl Homescreen {
    pub fn new(_cx: &mut Context<Self>, config: HomescreenConfig) -> Self {
        let state = HomescreenState::new(&config);
        Homescreen {
            state,
            config,
            drag_state: DragState::Idle,
            drag_offset: 0.0,
            target_offset: 0.0,
            animation_trigger: 0,
            widget_animations: Vec::new(),
        }
    }

    fn handle_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let click_pos = event.position;

        // Check if we clicked on a widget
        let current_page = self.state.layout_manager().get_active_page_index();
        let page = &self.state.layout_manager().get_pages()[current_page];

        // Find if we clicked on any widget
        let mut clicked_widget = None;
        for (widget_id, node) in &page.nodes {
            let bounds = self.state.layout_manager().grid_to_pixel_bounds(page, node.rect);
            if bounds.contains(&click_pos) {
                clicked_widget = Some(*widget_id);
                break;
            }
        }

        if let Some(widget_id) = clicked_widget {
            // Start holding on this widget
            self.drag_state = DragState::Holding {
                widget_id,
                start_pos: click_pos,
                start_time: std::time::Instant::now(),
            };
            cx.notify();
        } else {
            // Start page drag
            self.drag_state = DragState::DraggingPage {
                start_x: event.position.x.into(),
                current_offset: 0.0,
            };
        }
    }

    fn handle_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.drag_state {
            DragState::Holding {
                widget_id,
                start_pos,
                start_time,
            } => {
                let current_pos = event.position;
                let delta_x: f32 = (current_pos.x - start_pos.x).abs().into();
                let delta_y: f32 = (current_pos.y - start_pos.y).abs().into();
                let movement = (delta_x.powi(2) + delta_y.powi(2)).sqrt();

                // Check if we've held long enough
                let hold_elapsed = start_time.elapsed().as_secs_f32();
                if hold_elapsed >= self.config.interaction.hold_duration {
                    // Get the widget bounds
                    let current_page = self.state.layout_manager().get_active_page_index();
                    let page = &self.state.layout_manager().get_pages()[current_page];
                    if let Some(node) = page.nodes.get(&widget_id) {
                        let original_bounds =
                            self.state.layout_manager().grid_to_pixel_bounds(page, node.rect);

                        // Transition to dragging widget
                        self.drag_state = DragState::DraggingWidget {
                            widget_id,
                            start_pos,
                            current_pos,
                            original_bounds,
                        };
                        cx.notify();
                    }
                }
                // Check if we've moved too much during hold
                else if movement > self.config.interaction.hold_movement_threshold {
                    // Abort widget hold, switch to page drag
                    self.drag_state = DragState::DraggingPage {
                        start_x: start_pos.x.into(),
                        current_offset: 0.0,
                    };
                    cx.notify();
                }
            }
            DragState::DraggingWidget {
                widget_id,
                start_pos,
                original_bounds,
                ..
            } => {
                // Update the current position
                self.drag_state = DragState::DraggingWidget {
                    widget_id,
                    start_pos,
                    current_pos: event.position,
                    original_bounds,
                };
                cx.notify();
            }
            DragState::DraggingPage { start_x, .. } => {
                let current_x: f32 = event.position.x.into();
                let delta = current_x - start_x;

                // Check if we've moved beyond the threshold
                if delta.abs() >= self.config.interaction.drag_threshold {
                    self.drag_state = DragState::DraggingPage {
                        start_x,
                        current_offset: delta,
                    };
                    self.drag_offset = delta;
                    cx.notify();
                }
            }
            DragState::Idle => {}
        }
    }

    fn handle_mouse_up(
        &mut self,
        _event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.drag_state {
            DragState::Holding { .. } => {
                // Release without dragging, just reset
                self.drag_state = DragState::Idle;
                cx.notify();
            }
            DragState::DraggingWidget {
                widget_id,
                start_pos,
                current_pos,
                original_bounds,
            } => {
                // Calculate current dragged position
                let drag_offset_x = current_pos.x - start_pos.x;
                let drag_offset_y = current_pos.y - start_pos.y;

                let current_bounds = Bounds {
                    origin: point(
                        original_bounds.origin.x + drag_offset_x,
                        original_bounds.origin.y + drag_offset_y,
                    ),
                    size: original_bounds.size,
                };

                // Set up animation to return to original position
                // Remove any existing animation for this widget
                self.widget_animations.retain(|w| w.widget_id != widget_id);

                let trigger = self.widget_animations.len();
                self.widget_animations.push(WidgetAnimationState {
                    widget_id,
                    start_bounds: current_bounds,
                    target_bounds: original_bounds,
                    trigger,
                    start_time: std::time::Instant::now(),
                });

                self.drag_state = DragState::Idle;
                cx.notify();
            }
            DragState::DraggingPage { current_offset, .. } => {
                let window_width = self.config.window.width;
                let threshold = window_width * self.config.interaction.page_swipe_threshold;
                let current_page = self.state.layout_manager().get_active_page_index();
                let num_pages = self.state.layout_manager().get_pages().len();

                let should_switch_left = current_offset > threshold && current_page > 0;
                let should_switch_right =
                    current_offset < -threshold && current_page < num_pages - 1;

                self.target_offset = current_offset;

                if should_switch_left {
                    self.target_offset = -current_offset;
                    self.state
                        .layout_manager_mut()
                        .set_active_page(current_page - 1);
                } else if should_switch_right {
                    self.target_offset = -current_offset;
                    self.state
                        .layout_manager_mut()
                        .set_active_page(current_page + 1);
                }

                self.animation_trigger += 1;

                self.drag_state = DragState::Idle;
                self.drag_offset = 0.0;

                cx.notify();
            }
            DragState::Idle => {}
        }
    }
}

impl Render for Homescreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let window_width = self.config.window.width;
        let page_gap = self.config.visual.page_gap;
        let current_page_idx = self.state.layout_manager().get_active_page_index();
        let num_pages = self.state.layout_manager().get_pages().len();

        // Extract dragged widget info if dragging
        let dragged_widget_info = if let DragState::DraggingWidget {
            widget_id,
            start_pos,
            current_pos,
            original_bounds,
        } = self.drag_state
        {
            Some((widget_id, start_pos, current_pos, original_bounds))
        } else {
            None
        };

        // Remove completed animations before rendering
        let animation_duration = self.config.animation.widget_return;
        self.widget_animations.retain(|anim| {
            anim.start_time.elapsed() < animation_duration
        });

        // Clone animating widget info for rendering
        let animating_widgets: Vec<_> = self.widget_animations.clone();

        let is_dragging_page = matches!(self.drag_state, DragState::DraggingPage { .. });
        let drag_offset = self.drag_offset;
        let target_offset = self.target_offset;
        let animation_trigger = self.animation_trigger as u64;
        let animation_duration = self.config.animation.page_transition;

        let all_pages_data: Vec<_> = self
            .state
            .layout_manager()
            .get_pages()
            .iter()
            .enumerate()
            .map(|(page_idx, page)| {
                let widget_data: Vec<_> = page
                    .nodes
                    .iter()
                    .map(|(widget_id, node)| {
                        let bounds = self
                            .state
                            .layout_manager()
                            .grid_to_pixel_bounds(page, node.rect);
                        (*widget_id, bounds)
                    })
                    .collect();

                (page_idx, widget_data)
            })
            .collect();

        let mut pages_wrapper = div().size_full().relative();

        for (page_idx, widget_data) in &all_pages_data {
            let base_offset =
                (*page_idx as f32 - current_page_idx as f32) * (window_width + page_gap);

            let mut page_div = div()
                .absolute()
                .size_full()
                .left(px(base_offset))
                .top(px(0.0));

            for (widget_id, bounds) in widget_data {
                // Skip the dragged widget, we'll render it separately
                if let Some((dragged_id, _, _, _)) = dragged_widget_info {
                    if *widget_id == dragged_id {
                        continue;
                    }
                }

                // Skip animating widgets, we'll render them separately
                if animating_widgets.iter().any(|anim| anim.widget_id == *widget_id) {
                    continue;
                }

                if let Some(widget) = self.state.get_widget_mut(*widget_id) {
                    widget.set_bounds(*bounds);
                    let widget_element = widget.render(window, cx);
                    page_div = page_div.child(PositionedWidget::new(*bounds, widget_element));
                }
            }

            pages_wrapper = pages_wrapper.child(page_div);
        }

        let pages_container = div().size_full().relative().overflow_hidden().child({
            if is_dragging_page {
                div()
                    .size_full()
                    .relative()
                    .left(px(drag_offset))
                    .child(pages_wrapper)
                    .into_any_element()
            } else {
                div()
                    .size_full()
                    .relative()
                    .child(pages_wrapper)
                    .with_animation(
                        ElementId::Integer(animation_trigger),
                        Animation::new(animation_duration).with_easing(ease_out_quint()),
                        move |element, delta| {
                            let offset = target_offset * (1.0 - delta);
                            element.left(px(offset))
                        },
                    )
                    .into_any_element()
            }
        });

        let mut pagination = div()
            .absolute()
            .bottom(px(20.0))
            .left(px(0.0))
            .right(px(0.0))
            .flex()
            .justify_center()
            .items_center()
            .gap(px(self.config.visual.indicator_dot_gap));

        for page_idx in 0..num_pages {
            let is_active = page_idx == current_page_idx;
            let color = if is_active {
                rgb(self.config.visual.indicator_active_color)
            } else {
                rgba(self.config.visual.indicator_inactive_color)
            };

            let dot = div()
                .w(px(self.config.visual.indicator_dot_size))
                .h(px(self.config.visual.indicator_dot_size))
                .rounded(px(self.config.visual.indicator_dot_size / 2.0))
                .bg(color);

            pagination = pagination.child(dot);
        }

        let mut main_container = div()
            .size_full()
            .bg(rgb(self.config.visual.background_color))
            .relative()
            .on_mouse_down(MouseButton::Left, cx.listener(Self::handle_mouse_down))
            .on_mouse_move(cx.listener(Self::handle_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_mouse_up))
            .child(pages_container)
            .child(pagination);

        // Render dragged widget if we're dragging one
        if let Some((widget_id, start_pos, current_pos, original_bounds)) = dragged_widget_info {
            if let Some(widget) = self.state.get_widget_mut(widget_id) {
                // Calculate the dragged position
                let drag_offset_x = current_pos.x - start_pos.x;
                let drag_offset_y = current_pos.y - start_pos.y;

                let dragged_bounds = Bounds {
                    origin: point(
                        original_bounds.origin.x + drag_offset_x,
                        original_bounds.origin.y + drag_offset_y,
                    ),
                    size: original_bounds.size,
                };

                widget.set_bounds(dragged_bounds);
                let widget_element = widget.render(window, cx);
                main_container =
                    main_container.child(PositionedWidget::new(dragged_bounds, widget_element));
            }
        }

        // Render animating widgets
        for anim_state in animating_widgets {
            if let Some(widget) = self.state.get_widget_mut(anim_state.widget_id) {
                let start_bounds = anim_state.start_bounds;
                let target_bounds = anim_state.target_bounds;
                let trigger = anim_state.trigger;
                let animation_duration = self.config.animation.widget_return;

                widget.set_bounds(target_bounds);
                let widget_element = widget.render(window, cx);

                let positioned_widget = PositionedWidget::new(target_bounds, widget_element);

                main_container = main_container.child(
                    div()
                        .absolute()
                        .size_full()
                        .child(positioned_widget)
                        .with_animation(
                            ElementId::Integer(trigger as u64),
                            Animation::new(animation_duration).with_easing(ease_out_quint()),
                            move |element, delta| {
                                // Interpolate from start to target
                                // offset_x and offset_y are Pixels, convert to f32 for calculation
                                let offset_x: f32 = (start_bounds.origin.x - target_bounds.origin.x).into();
                                let offset_y: f32 = (start_bounds.origin.y - target_bounds.origin.y).into();
                                element.left(px(offset_x * (1.0 - delta))).top(px(offset_y * (1.0 - delta)))
                            },
                        ),
                );
            }
        }

        main_container
    }
}
