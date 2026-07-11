use std::collections::HashSet;

use nnbfl::bflan::{anim_info::AnimInfo, curves::Curve};

use crate::{
    anim_state::{AnimInstance, AnimPlayer},
    ui::DrawUi,
};

pub const TIMELINE_RULER_HEIGHT: f32 = 22.0;
pub const TIMELINE_MARKER_RADIUS: f32 = 4.0;
pub const TIMELINE_MIN_VISIBLE_FRAMES: f32 = 1.0;

const TRACK_PALETTE: [egui::Color32; 8] = [
    egui::Color32::from_rgb(224, 96, 96),
    egui::Color32::from_rgb(96, 170, 224),
    egui::Color32::from_rgb(120, 200, 120),
    egui::Color32::from_rgb(230, 190, 90),
    egui::Color32::from_rgb(190, 120, 224),
    egui::Color32::from_rgb(230, 140, 90),
    egui::Color32::from_rgb(90, 210, 200),
    egui::Color32::from_rgb(224, 120, 170),
];

pub fn track_color(index_in_pane: usize) -> egui::Color32 {
    TRACK_PALETTE[index_in_pane % TRACK_PALETTE.len()]
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TimelineTrack {
    pub label: String,
    pub content_idx: usize,
    pub info_idx: usize,
    pub target_idx: usize,
}

#[derive(Clone, Debug)]
pub enum TimelineRow {
    PaneHeader { content_idx: usize, label: String },
    Track(TimelineTrack),
}

impl TimelineRow {
    pub const HEIGHT: f32 = 26.0;
    pub const GAP: f32 = 8.0;

    pub fn build(anim: &AnimInstance, expanded: &HashSet<usize>) -> Vec<Self> {
        let mut out = Vec::new();

        for (content_idx, content) in anim.bflan.anim_info.contents.iter().enumerate() {
            let target_count: usize = content
                .infos
                .iter()
                .map(|info| match info {
                    AnimInfo::Standard { targets, .. } => targets.len(),
                    _ => 0,
                })
                .sum();

            if target_count == 0 {
                continue;
            }

            out.push(Self::PaneHeader {
                content_idx,
                label: content.name.trim_end_matches('\0').to_string(),
            });

            if !expanded.contains(&content_idx) {
                continue;
            }

            for (info_idx, info) in content.infos.iter().enumerate() {
                if let AnimInfo::Standard { targets, .. } = info {
                    for (target_idx, target) in targets.iter().enumerate() {
                        out.push(Self::Track(TimelineTrack {
                            label: format!("{:?}", target.target),
                            content_idx,
                            info_idx,
                            target_idx,
                        }));
                    }
                }
            }
        }

        out
    }

    pub const fn total_height() -> f32 {
        Self::HEIGHT + Self::GAP
    }
}

fn curve_value_range(curve: &Curve) -> (f32, f32) {
    let values: Vec<f32> = match curve {
        Curve::Constant(keys) => keys.clone(),
        Curve::Step(keys) => keys.iter().map(|k| k.value as f32).collect(),
        Curve::Hermite(keys) => keys.iter().map(|k| k.value).collect(),
    };

    let Some(&first) = values.first() else {
        return (0.0, 1.0);
    };

    let (min_v, max_v) = values
        .iter()
        .fold((first, first), |(lo, hi), &v| (lo.min(v), hi.max(v)));

    if (max_v - min_v).abs() < f32::EPSILON {
        (min_v - 1.0, max_v + 1.0)
    } else {
        let pad = (max_v - min_v) * 0.1;
        (min_v - pad, max_v + pad)
    }
}

#[derive(Clone, Debug)]
pub struct PendingKeyEdit {
    pub track: TimelineTrack,
    pub key_idx: usize,
    pub frame: f32,
    pub value: f32,
}

#[derive(Clone, Debug)]
pub struct PendingSlopeEdit {
    pub track: TimelineTrack,
    pub key_idx: usize,
    pub slope: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectedKey {
    pub track: TimelineTrack,
    pub key_idx: usize,
}

pub struct TimelineState {
    pub anim_player: AnimPlayer,
    pub pending_key_edit: Option<PendingKeyEdit>,
    pub pending_slope_edit: Option<PendingSlopeEdit>,
    pub selected_key: Option<SelectedKey>,
    pub expanded_anim_panes: HashSet<usize>,
    pub hidden_tracks: HashSet<TimelineTrack>,
    pub zoom: f32,

    /// First visible frame from the left edge of the graph.
    pub pan_frame: f32,
    pub panning: bool,
    pub frame_rate: f32,
}

impl Default for TimelineState {
    fn default() -> Self {
        Self {
            anim_player: AnimPlayer::new(),
            pending_key_edit: None,
            pending_slope_edit: None,
            selected_key: None,
            hidden_tracks: HashSet::new(),
            expanded_anim_panes: HashSet::new(),
            zoom: 0.0,
            pan_frame: 0.0,
            panning: false,
            frame_rate: 30.0,
        }
    }
}

fn clamp_to_rect(rect: egui::Rect, p: egui::Pos2) -> egui::Pos2 {
    let (x_lo, x_hi) = (rect.min.x.min(rect.max.x), rect.min.x.max(rect.max.x));
    let (y_lo, y_hi) = (rect.min.y.min(rect.max.y), rect.min.y.max(rect.max.y));

    egui::pos2(p.x.clamp(x_lo, x_hi), p.y.clamp(y_lo, y_hi))
}

impl DrawUi<()> for TimelineState {
    fn draw(&mut self, ui: &mut egui::Ui) {
        let Some(active_idx) = self.anim_player.active else {
            return;
        };

        let (frame_count, anim_name, anim_frame, rows, anim_groups) = {
            let Some(anim) = self.anim_player.anims.get(active_idx) else {
                return;
            };

            let frame_count = anim.frame_count().max(1.0);
            let rows = TimelineRow::build(anim, &self.expanded_anim_panes);

            (
                frame_count,
                anim.name.clone(),
                anim.frame,
                rows,
                anim.bflan.anim_tag.groups.clone(),
            )
        };

        egui::Panel::bottom("timeline_panel")
            .resizable(true)
            .default_size(320.0)
            .min_size(120.0)
            .show(ui, |ui| {
                let zoom = self.zoom.max(1.0);
                let visible_span =
                    (frame_count / zoom).clamp(TIMELINE_MIN_VISIBLE_FRAMES, frame_count);
                let max_pan = (frame_count - visible_span).max(0.0);
                self.pan_frame = self.pan_frame.clamp(0.0, max_pan);

                ui.horizontal(|ui| {
                    ui.heading("Timeline");
                    ui.separator();

                    ui.label(format!(
                        "{} - frame {:.1} / {frame_count:.0}",
                        anim_name, anim_frame
                    ));

                    ui.separator();

                    ui.label("Groups:");
                    for group in anim_groups {
                        ui.label(group);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(egui::DragValue::new(&mut self.frame_rate).speed(1));
                        ui.label("FPS:");

                        ui.separator();

                        if ui.small_button("Fit").clicked() {
                            self.zoom = 1.0;
                            self.pan_frame = 0.0;
                        }

                        ui.label(format!("Zoom {zoom:.1}x"));
                    });
                });

                ui.separator();

                if rows.is_empty() {
                    ui.weak("No animated panes in this animation.");
                    return;
                }

                const LABEL_COL_WIDTH: f32 = 210.0;
                const INSPECTOR_WIDTH: f32 = 200.0;

                let content_rect = ui.available_rect_before_wrap();
                let has_inspector = self.selected_key.is_some();
                let inspector_w = if has_inspector { INSPECTOR_WIDTH } else { 0.0 };

                let main_rect = egui::Rect::from_min_max(
                    content_rect.min,
                    egui::pos2(content_rect.max.x - inspector_w, content_rect.max.y),
                );

                let graph_x0 = main_rect.min.x + LABEL_COL_WIDTH;
                let ruler_rect = egui::Rect::from_min_max(
                    egui::pos2(graph_x0, main_rect.min.y),
                    egui::pos2(main_rect.max.x, main_rect.min.y + TIMELINE_RULER_HEIGHT),
                );

                draw_ruler(ui, ruler_rect, self.pan_frame, visible_span);

                let scroll_rect = egui::Rect::from_min_max(
                    egui::pos2(main_rect.min.x, ruler_rect.max.y),
                    main_rect.max,
                );

                let row_h = TimelineRow::total_height();
                let content_height = rows.len() as f32 * row_h + 8.0;

                let builder = egui::UiBuilder::new().max_rect(scroll_rect);

                ui.scope_builder(builder, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("timeline_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let avail = ui.available_rect_before_wrap();
                            let graph_rect = egui::Rect::from_min_max(
                                egui::pos2(avail.min.x + LABEL_COL_WIDTH, avail.min.y),
                                egui::pos2(avail.max.x, avail.min.y + content_height),
                            );

                            let canvas_response = ui.interact(
                                graph_rect,
                                ui.id().with("timeline_graph"),
                                egui::Sense::click_and_drag(),
                            );

                            let mut any_point_active = false;

                            let mut i = 0;
                            while i < rows.len() {
                                let row_top = avail.min.y + i as f32 * row_h;

                                let row_rect = egui::Rect::from_min_size(
                                    egui::pos2(avail.min.x, row_top),
                                    egui::vec2(LABEL_COL_WIDTH, row_h),
                                );

                                let TimelineRow::PaneHeader { content_idx, label } = &rows[i]
                                else {
                                    i += 1;
                                    continue;
                                };

                                let expanded = self.expanded_anim_panes.contains(content_idx);
                                let header_response = ui.interact(
                                    row_rect,
                                    ui.id().with(("timeline_header", *content_idx)),
                                    egui::Sense::click(),
                                );

                                if header_response.clicked() {
                                    if expanded {
                                        self.expanded_anim_panes.remove(content_idx);
                                    } else {
                                        self.expanded_anim_panes.insert(*content_idx);
                                    }
                                }

                                if header_response.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }

                                let status = if expanded { "Close" } else { "Open" };
                                ui.painter().text(
                                    egui::pos2(avail.min.x + 4.0, row_top + row_h * 0.5 - 2.0),
                                    egui::Align2::LEFT_CENTER,
                                    format!("{status}, {label}"),
                                    egui::FontId::proportional(12.0),
                                    egui::Color32::WHITE,
                                );

                                if !expanded {
                                    i += 1;
                                    continue;
                                }

                                let mut track_count = 0;
                                while let Some(TimelineRow::Track(_)) =
                                    rows.get(i + 1 + track_count)
                                {
                                    track_count += 1;
                                }

                                let tracks: Vec<&TimelineTrack> = rows[i + 1..i + 1 + track_count]
                                    .iter()
                                    .map(|r| match r {
                                        TimelineRow::Track(t) => t,
                                        TimelineRow::PaneHeader { .. } => unreachable!(),
                                    })
                                    .collect();

                                for (t_idx, track) in tracks.iter().enumerate() {
                                    let legend_top = row_top + (t_idx + 1) as f32 * row_h;

                                    let color = track_color(t_idx);
                                    let mut visible = !self.hidden_tracks.contains(*track);

                                    let checkbox_rect = egui::Rect::from_min_size(
                                        egui::pos2(
                                            avail.min.x + 16.0,
                                            legend_top + row_h * 0.5 - 8.0,
                                        ),
                                        egui::vec2(16.0, 16.0),
                                    );

                                    let mut checkbox_ui = ui.new_child(
                                        egui::UiBuilder::new().max_rect(checkbox_rect).layout(
                                            egui::Layout::left_to_right(egui::Align::Center),
                                        ),
                                    );

                                    if checkbox_ui.checkbox(&mut visible, "").changed() {
                                        if visible {
                                            self.hidden_tracks.remove(*track);
                                        } else {
                                            self.hidden_tracks.insert((*track).clone());
                                        }
                                    }

                                    let swatch_center = egui::pos2(
                                        avail.min.x + 40.0,
                                        legend_top + row_h * 0.5 - 2.0,
                                    );

                                    ui.painter().rect_filled(
                                        egui::Rect::from_center_size(
                                            swatch_center,
                                            egui::vec2(8.0, 8.0),
                                        ),
                                        1.0,
                                        color,
                                    );

                                    ui.painter().text(
                                        egui::pos2(
                                            avail.min.x + 52.0,
                                            legend_top + row_h * 0.5 - 2.0,
                                        ),
                                        egui::Align2::LEFT_CENTER,
                                        &track.label,
                                        egui::FontId::proportional(11.0),
                                        if visible {
                                            egui::Color32::from_gray(200)
                                        } else {
                                            egui::Color32::from_gray(110)
                                        },
                                    );

                                    if t_idx + 1 < tracks.len() {
                                        let sep_y = legend_top + row_h - TimelineRow::GAP * 0.5;
                                        ui.painter().line_segment(
                                            [
                                                egui::pos2(avail.min.x + 8.0, sep_y),
                                                egui::pos2(
                                                    avail.min.x + LABEL_COL_WIDTH - 8.0,
                                                    sep_y,
                                                ),
                                            ],
                                            egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
                                        );
                                    }
                                }

                                let canvas_bottom =
                                    row_top + (1 + track_count) as f32 * row_h - TimelineRow::GAP;
                                let pane_canvas = egui::Rect::from_min_max(
                                    egui::pos2(graph_rect.min.x, row_top),
                                    egui::pos2(graph_rect.max.x, canvas_bottom),
                                );

                                self.draw_pane_canvas(
                                    ui,
                                    active_idx,
                                    &tracks,
                                    pane_canvas,
                                    &mut any_point_active,
                                );

                                i += 1 + track_count;
                            }

                            if !any_point_active {
                                if canvas_response.drag_started() {
                                    self.panning = true;
                                }

                                if canvas_response.dragged() && self.panning {
                                    let px_per_frame = graph_rect.width() / visible_span;
                                    let delta_frames =
                                        -canvas_response.drag_delta().x / px_per_frame.max(0.001);
                                    self.pan_frame =
                                        (self.pan_frame + delta_frames).clamp(0.0, max_pan);
                                }
                            }

                            if canvas_response.drag_stopped() {
                                self.panning = false;
                            }

                            if canvas_response.hovered() {
                                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                                if scroll.abs() > f32::EPSILON
                                    && let Some(pos) = canvas_response.hover_pos()
                                {
                                    let px_per_frame = graph_rect.width() / visible_span;
                                    let pointer_frame =
                                        self.pan_frame + (pos.x - graph_rect.min.x) / px_per_frame;

                                    let max_zoom = frame_count / TIMELINE_MIN_VISIBLE_FRAMES;
                                    let new_zoom = (zoom * (1.0 + scroll * 0.002))
                                        .clamp(1.0, max_zoom.max(1.0));
                                    let new_span = (frame_count / new_zoom)
                                        .clamp(TIMELINE_MIN_VISIBLE_FRAMES, frame_count);

                                    let t = (pointer_frame - self.pan_frame) / visible_span;
                                    let new_max_pan = (frame_count - new_span).max(0.0);
                                    self.pan_frame =
                                        (pointer_frame - t * new_span).clamp(0.0, new_max_pan);
                                    self.zoom = new_zoom;
                                }

                                let cursor = if any_point_active {
                                    egui::CursorIcon::Grabbing
                                } else if self.panning {
                                    egui::CursorIcon::Grab
                                } else {
                                    egui::CursorIcon::Default
                                };

                                ui.ctx().set_cursor_icon(cursor);
                            }

                            ui.allocate_rect(
                                egui::Rect::from_min_size(
                                    avail.min,
                                    egui::vec2(avail.width(), content_height),
                                ),
                                egui::Sense::hover(),
                            );
                        });
                });

                if has_inspector {
                    let inspector_rect = egui::Rect::from_min_max(
                        egui::pos2(content_rect.max.x - INSPECTOR_WIDTH, content_rect.min.y),
                        content_rect.max,
                    );

                    self.draw_key_inspector(ui, active_idx, inspector_rect);
                }
            });
    }
}

fn draw_ruler(ui: &mut egui::Ui, ruler_rect: egui::Rect, visible_start: f32, visible_span: f32) {
    let px_per_frame = ruler_rect.width() / visible_span.max(0.001);
    let tick_step = if px_per_frame >= 8.0 {
        1
    } else {
        (8.0 / px_per_frame.max(0.001)).ceil() as i32
    };

    let major_every = (tick_step * 5).max(1);

    let painter = ui.painter();
    let first_frame = (visible_start / tick_step as f32).floor() as i32 * tick_step;
    let last_frame = (visible_start + visible_span).ceil() as i32;

    let mut frame = first_frame.max(0);
    while frame <= last_frame {
        let x =
            ruler_rect.min.x + (frame as f32 - visible_start) / visible_span * ruler_rect.width();
        if x >= ruler_rect.min.x && x <= ruler_rect.max.x {
            let is_major = frame % major_every == 0;
            let y0 = ruler_rect.max.y - if is_major { 12.0 } else { 6.0 };

            painter.line_segment(
                [egui::pos2(x, y0), egui::pos2(x, ruler_rect.max.y)],
                egui::Stroke::new(1.0, egui::Color32::from_gray(140)),
            );

            if is_major {
                painter.text(
                    egui::pos2(x + 2.0, ruler_rect.min.y + 2.0),
                    egui::Align2::LEFT_TOP,
                    frame.to_string(),
                    egui::FontId::proportional(9.0),
                    egui::Color32::from_gray(160),
                );
            }
        }

        frame += tick_step;
    }

    painter.line_segment(
        [
            egui::pos2(ruler_rect.min.x, ruler_rect.max.y),
            egui::pos2(ruler_rect.max.x, ruler_rect.max.y),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_gray(140)),
    );
}

impl TimelineState {
    fn draw_pane_canvas(
        &mut self,
        ui: &mut egui::Ui,
        active_idx: usize,
        tracks: &[&TimelineTrack],
        canvas_rect: egui::Rect,
        any_point_active: &mut bool,
    ) {
        let Some(anim) = self.anim_player.anims.get(active_idx) else {
            return;
        };

        ui.painter()
            .rect_filled(canvas_rect, 2.0, egui::Color32::from_black_alpha(40));

        let frame_count = anim.frame_count().max(1.0);
        let zoom = self.zoom.max(1.0);
        let visible_span = (frame_count / zoom).clamp(TIMELINE_MIN_VISIBLE_FRAMES, frame_count);
        let visible_start = self.pan_frame;

        for (color_idx, track) in tracks.iter().enumerate() {
            let Some(curve) = anim.curve(track) else {
                continue;
            };

            let (min_v, max_v) = curve_value_range(curve);
            let data_rect = egui::Rect::from_min_max(
                egui::pos2(visible_start, max_v),
                egui::pos2(visible_start + visible_span, min_v),
            );

            let to_screen = egui::emath::RectTransform::from_to(data_rect, canvas_rect);

            let hidden = self.hidden_tracks.contains(*track);
            if hidden {
                continue;
            }

            let color = track_color(color_idx);
            draw_curve(ui, &to_screen, curve, color);

            let key_count = match curve {
                Curve::Constant(keys) => keys.len(),
                Curve::Step(keys) => keys.len(),
                Curve::Hermite(keys) => keys.len(),
            };

            for key_idx in 0..key_count {
                let (frame, value) = key_point(curve, key_idx);
                let screen_pos = to_screen.transform_pos(egui::pos2(frame, value));
                if !canvas_rect
                    .expand(TIMELINE_MARKER_RADIUS)
                    .contains(screen_pos)
                {
                    continue;
                }

                let point_rect = egui::Rect::from_center_size(
                    screen_pos,
                    egui::Vec2::splat(TIMELINE_MARKER_RADIUS * 2.5),
                );

                let point_id = ui.id().with((
                    "timeline_key",
                    track.content_idx,
                    track.info_idx,
                    track.target_idx,
                    key_idx,
                ));

                let point_response =
                    ui.interact(point_rect, point_id, egui::Sense::click_and_drag());

                if point_response.drag_started() || point_response.clicked() {
                    self.selected_key = Some(SelectedKey {
                        track: (*track).clone(),
                        key_idx,
                    });
                }

                if point_response.dragged() {
                    *any_point_active = true;
                    if let Some(pos) = point_response.interact_pointer_pos() {
                        let data_pos = to_screen.inverse().transform_pos(pos);
                        self.pending_key_edit = Some(PendingKeyEdit {
                            track: (*track).clone(),
                            key_idx,
                            frame: data_pos.x,
                            value: data_pos.y,
                        });
                    }
                }

                if point_response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }

                let is_selected = self
                    .selected_key
                    .as_ref()
                    .is_some_and(|s| s.track == **track && s.key_idx == key_idx);

                ui.painter()
                    .circle_filled(screen_pos, TIMELINE_MARKER_RADIUS, color);
                ui.painter().circle_stroke(
                    screen_pos,
                    TIMELINE_MARKER_RADIUS,
                    egui::Stroke::new(
                        if is_selected { 2.0 } else { 1.0 },
                        if is_selected {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::BLACK
                        },
                    ),
                );
            }

            if let Curve::Hermite(keys) = curve
                && let Some(selected) = &self.selected_key
                && selected.track == **track
                && let Some(key) = keys.get(selected.key_idx)
            {
                let handle_dx = visible_span * 0.06;
                let handle_frame = key.frame + handle_dx;
                let handle_value = key.value + key.slope * handle_dx;

                let key_screen = to_screen.transform_pos(egui::pos2(key.frame, key.value));
                let handle_screen = clamp_to_rect(
                    canvas_rect,
                    to_screen.transform_pos(egui::pos2(handle_frame, handle_value)),
                );

                ui.painter().line_segment(
                    [key_screen, handle_screen],
                    egui::Stroke::new(1.5, egui::Color32::WHITE),
                );

                let handle_rect = egui::Rect::from_center_size(
                    handle_screen,
                    egui::Vec2::splat(TIMELINE_MARKER_RADIUS * 2.0),
                );

                let handle_id = ui.id().with((
                    "timeline_handle",
                    track.content_idx,
                    track.info_idx,
                    track.target_idx,
                    selected.key_idx,
                ));

                let handle_response =
                    ui.interact(handle_rect, handle_id, egui::Sense::click_and_drag());

                if handle_response.dragged() {
                    *any_point_active = true;
                    if let Some(pos) = handle_response.interact_pointer_pos() {
                        let data_pos =
                            clamp_to_rect(data_rect, to_screen.inverse().transform_pos(pos));
                        let df = data_pos.x - key.frame;
                        if df.abs() > 0.01 {
                            self.pending_slope_edit = Some(PendingSlopeEdit {
                                track: (*track).clone(),
                                key_idx: selected.key_idx,
                                slope: (data_pos.y - key.value) / df,
                            });
                        }
                    }
                }

                if handle_response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }

                ui.painter().add(egui::Shape::convex_polygon(
                    diamond_points(handle_screen, TIMELINE_MARKER_RADIUS * 0.9),
                    egui::Color32::WHITE,
                    egui::Stroke::new(1.0, egui::Color32::BLACK),
                ));
            }
        }

        let playhead_x =
            canvas_rect.min.x + (anim.frame - visible_start) / visible_span * canvas_rect.width();

        if playhead_x >= canvas_rect.min.x && playhead_x <= canvas_rect.max.x {
            ui.painter().line_segment(
                [
                    egui::pos2(playhead_x, canvas_rect.min.y),
                    egui::pos2(playhead_x, canvas_rect.max.y),
                ],
                egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 80, 90)),
            );
        }
    }

    fn draw_key_inspector(&mut self, ui: &mut egui::Ui, active_idx: usize, rect: egui::Rect) {
        let Some(anim) = self.anim_player.anims.get(active_idx) else {
            return;
        };

        let Some(selected) = self.selected_key.clone() else {
            return;
        };

        let Some(curve) = anim.curve(&selected.track) else {
            self.selected_key = None;
            return;
        };

        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        let ui = &mut child;

        ui.separator();
        ui.heading("Keyframe");
        ui.label(egui::RichText::new(&selected.track.label).weak());
        ui.add_space(6.0);

        match curve {
            Curve::Hermite(keys) => {
                let Some(key) = keys.get(selected.key_idx) else {
                    return;
                };
                let (mut frame, mut value, mut slope) = (key.frame, key.value, key.slope);

                egui::Grid::new("timeline_inspector_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Frame");
                        if ui
                            .add(egui::DragValue::new(&mut frame).speed(0.1))
                            .changed()
                        {
                            self.pending_key_edit = Some(PendingKeyEdit {
                                track: selected.track.clone(),
                                key_idx: selected.key_idx,
                                frame,
                                value,
                            });
                        }
                        ui.end_row();

                        ui.label("Value");
                        if ui
                            .add(egui::DragValue::new(&mut value).speed(0.1))
                            .changed()
                        {
                            self.pending_key_edit = Some(PendingKeyEdit {
                                track: selected.track.clone(),
                                key_idx: selected.key_idx,
                                frame,
                                value,
                            });
                        }
                        ui.end_row();

                        ui.label("Slope");
                        if ui
                            .add(egui::DragValue::new(&mut slope).speed(0.01))
                            .changed()
                        {
                            self.pending_slope_edit = Some(PendingSlopeEdit {
                                track: selected.track.clone(),
                                key_idx: selected.key_idx,
                                slope,
                            });
                        }
                        ui.end_row();
                    });
            }

            Curve::Step(keys) => {
                let Some(key) = keys.get(selected.key_idx) else {
                    return;
                };
                let (mut frame, mut value) = (key.frame, key.value as f32);

                egui::Grid::new("timeline_inspector_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Frame");
                        if ui
                            .add(egui::DragValue::new(&mut frame).speed(0.1))
                            .changed()
                        {
                            self.pending_key_edit = Some(PendingKeyEdit {
                                track: selected.track.clone(),
                                key_idx: selected.key_idx,
                                frame,
                                value,
                            });
                        }
                        ui.end_row();

                        ui.label("Value");
                        if ui
                            .add(
                                egui::DragValue::new(&mut value)
                                    .speed(1.0)
                                    .range(0.0..=65535.0),
                            )
                            .changed()
                        {
                            self.pending_key_edit = Some(PendingKeyEdit {
                                track: selected.track.clone(),
                                key_idx: selected.key_idx,
                                frame,
                                value,
                            });
                        }
                        ui.end_row();
                    });
            }

            Curve::Constant(values) => {
                let Some(&v) = values.get(selected.key_idx) else {
                    return;
                };
                let mut value = v;

                ui.horizontal(|ui| {
                    ui.label("Value");
                    if ui
                        .add(egui::DragValue::new(&mut value).speed(1.0))
                        .changed()
                    {
                        self.pending_key_edit = Some(PendingKeyEdit {
                            track: selected.track.clone(),
                            key_idx: selected.key_idx,
                            frame: selected.key_idx as f32,
                            value,
                        });
                    }
                });
            }
        }

        ui.add_space(10.0);

        if ui.button("Close").clicked() {
            self.selected_key = None;
        }
    }
}

fn diamond_points(center: egui::Pos2, r: f32) -> Vec<egui::Pos2> {
    vec![
        egui::pos2(center.x, center.y - r),
        egui::pos2(center.x + r, center.y),
        egui::pos2(center.x, center.y + r),
        egui::pos2(center.x - r, center.y),
    ]
}

fn key_point(curve: &Curve, idx: usize) -> (f32, f32) {
    match curve {
        Curve::Constant(keys) => (idx as f32, keys[idx]),
        Curve::Step(keys) => (keys[idx].frame, keys[idx].value as f32),
        Curve::Hermite(keys) => (keys[idx].frame, keys[idx].value),
    }
}

fn draw_curve(
    ui: &egui::Ui,
    to_screen: &egui::emath::RectTransform,
    curve: &Curve,
    color: egui::Color32,
) {
    let stroke = egui::Stroke::new(1.5, color);

    match curve {
        Curve::Hermite(keys) => {
            for pair in keys.windows(2) {
                let [k0, k1] = pair else { continue };
                let dt = k1.frame - k0.frame;
                if dt.abs() < f32::EPSILON {
                    continue;
                }

                let p0 = egui::pos2(k0.frame, k0.value);
                let p1 = egui::pos2(k0.frame + dt / 3.0, k0.value + dt * k0.slope / 3.0);
                let p2 = egui::pos2(k1.frame - dt / 3.0, k1.value - dt * k1.slope / 3.0);
                let p3 = egui::pos2(k1.frame, k1.value);

                let screen_points = [
                    to_screen.transform_pos(p0),
                    to_screen.transform_pos(p1),
                    to_screen.transform_pos(p2),
                    to_screen.transform_pos(p3),
                ];

                let shape = egui::epaint::CubicBezierShape::from_points_stroke(
                    screen_points,
                    false,
                    egui::Color32::TRANSPARENT,
                    stroke,
                );
                ui.painter().add(shape);
            }
        }
        Curve::Step(keys) => {
            let mut points = Vec::with_capacity(keys.len() * 2);
            for pair in keys.windows(2) {
                let [k0, k1] = pair else { continue };
                points.push(to_screen.transform_pos(egui::pos2(k0.frame, k0.value as f32)));
                points.push(to_screen.transform_pos(egui::pos2(k1.frame, k0.value as f32)));
            }

            if let Some(last) = keys.last() {
                points.push(to_screen.transform_pos(egui::pos2(last.frame, last.value as f32)));
                points.push(
                    to_screen
                        .transform_pos(egui::pos2(to_screen.from().right(), last.value as f32)),
                );
            }

            if points.len() >= 2 {
                ui.painter()
                    .add(egui::epaint::PathShape::line(points, stroke));
            }
        }

        Curve::Constant(values) => {
            let mut points = Vec::with_capacity(values.len() * 2);
            for (i, &v) in values.iter().enumerate() {
                points.push(to_screen.transform_pos(egui::pos2(i as f32, v)));
                points.push(to_screen.transform_pos(egui::pos2((i + 1) as f32, v)));
            }

            if points.len() >= 2 {
                ui.painter()
                    .add(egui::epaint::PathShape::line(points, stroke));
            }
        }
    }
}
