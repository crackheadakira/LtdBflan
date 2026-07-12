mod anim_state;
mod archive_browser;
mod bflyt_view;
mod camera;
mod chinese_font;
mod edit_history;
mod keybinds;
mod pane_tree;
mod renderer;
mod traits;
mod ui;

use std::{path::PathBuf, sync::Arc, time::Instant};

use camera::Camera;
use chinese_font::{FontError, setup_chinese_fonts};
use egui_wgpu::{RendererOptions, ScreenDescriptor};
use nnbfl::{
    bflan::file::Bflan,
    bflyt::{file::Bflyt, list::Layout},
    core::FileReadWriteable,
    sarc::file::{MagicFiles, Sarc, SarcFile},
    ui2d::types::{Vector2f, Vector3f},
};
use pollster::FutureExt;
use renderer::quad::GridRenderer;
use renderer::texture::TextureCache;
use renderer::textured_quad::PaneRenderer;
use wgpu::CurrentSurfaceTexture;
use winit::{
    application::ApplicationHandler,
    event::{MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

use bflyt_view::{BflytView, build_view};

use crate::{
    anim_state::AnimPlayer,
    archive_browser::ArchiveScan,
    renderer::selection::{Handle, SelectionRenderer, point_in_quad},
    traits::Displaying,
    ui::general::{SUPPORTED_SARC_EXTENSIONS, UiAction, UiState, draw_ui},
};

struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    grid_renderer: GridRenderer,
    pane_renderer: PaneRenderer,
    selection_renderer: SelectionRenderer,
    egui_renderer: egui_wgpu::Renderer,

    texture_cache: TextureCache,
}

impl GpuState {
    fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions::default(),
            display: None,
        });

        let surface = instance.create_surface(window).expect("create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .block_on()
            .expect("find adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
            })
            .block_on()
            .expect("create device");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let grid_renderer = GridRenderer::new(&device, surface_format);

        let selection_renderer = SelectionRenderer::new(&device, surface_format);

        let texture_cache = TextureCache::new();
        let pane_renderer = PaneRenderer::new(&device, &queue, surface_format);

        let egui_renderer =
            egui_wgpu::Renderer::new(&device, surface_format, RendererOptions::default());

        Self {
            surface,
            device,
            queue,
            config,
            grid_renderer,
            pane_renderer,
            texture_cache,
            egui_renderer,
            selection_renderer,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
    }

    fn render(
        &mut self,
        window: &Window,
        egui_ctx: &egui::Context,
        egui_state: &mut egui_winit::State,
        mut ctx: RenderContext<'_>,
    ) {
        if ctx.ui_state.material_editor.pending_upload
            && let Some(ref mut bflyt_view) = ctx.bflyt_view
        {
            ctx.ui_state.material_editor.pending_upload = false;
            bflyt_view.tree.recompute_dirty_materials();

            let render_quads = bflyt_view.tree.collect_render_quads();
            self.pane_renderer.upload_quads(
                &self.device,
                &render_quads,
                &self.texture_cache,
                bflyt_view.tree.layout_size.x,
                bflyt_view.tree.layout_size.y,
            );
        }

        self.grid_renderer
            .update_projection(&self.queue, ctx.camera, &self.config);

        let matrix = ctx
            .camera
            .build_matrix(self.config.width as f32, self.config.height as f32);
        self.pane_renderer.update_projection(&self.queue, matrix);
        self.selection_renderer
            .update_projection(&self.queue, matrix);

        let mut scissor_rect = None;
        if let Some(ref view) = ctx.bflyt_view
            && ctx.ui_state.visiblity_flags.clip_to_root
        {
            let screen_w = self.config.width as f32;
            let screen_h = self.config.height as f32;

            let scale_x = matrix[0][0];
            let scale_y = matrix[1][1];
            let trans_x = matrix[3][0];
            let trans_y = matrix[3][1];

            let ndc_x0 = trans_x;
            let ndc_y0 = trans_y;
            let ndc_x1 = view.tree.layout_size.x * scale_x + trans_x;
            let ndc_y1 = view.tree.layout_size.y * scale_y + trans_y;

            let x0 = ((ndc_x0 + 1.0) * 0.5 * screen_w).clamp(0.0, screen_w);
            let y0 = ((1.0 - ndc_y0) * 0.5 * screen_h).clamp(0.0, screen_h);
            let x1 = ((ndc_x1 + 1.0) * 0.5 * screen_w).clamp(0.0, screen_w);
            let y1 = ((1.0 - ndc_y1) * 0.5 * screen_h).clamp(0.0, screen_h);

            let sx = x0.min(x1) as u32;
            let sy = y0.min(y1) as u32;
            let sw = (x0 - x1).abs() as u32;
            let sh = (y0 - y1).abs() as u32;

            if sw > 0 && sh > 0 {
                scissor_rect = Some((sx, sy, sw, sh));
            }
        }

        let output = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(o) => o,
            CurrentSurfaceTexture::Lost | CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            _ => {
                log::error!("Unknown surface error");
                return;
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let raw_input = egui_state.take_egui_input(window);
        let full_output = egui_ctx.run_ui(raw_input, |ui| {
            draw_ui(
                ui,
                &mut ctx,
                self.config.width as f32,
                self.config.height as f32,
            );
        });

        egui_state.handle_platform_output(window, full_output.platform_output.clone());

        if let Some(ref mut bflyt_view) = ctx.bflyt_view {
            match ctx
                .ui_state
                .selected_pane
                .and_then(|idx| bflyt_view.tree.find_by_idx(idx))
            {
                Some(node) => self
                    .selection_renderer
                    .update(&self.device, &node.world_corners),
                None => self.selection_renderer.clear(),
            }

            let mut render_quads = bflyt_view.tree.collect_render_quads();

            self.pane_renderer.update_anim(
                &self.queue,
                &render_quads,
                &ctx.ui_state.hidden_panes,
                ctx.ui_state.visiblity_flags,
            );

            self.pane_renderer.update_texture_pattern(
                &self.device,
                &render_quads,
                &self.texture_cache,
            );

            self.pane_renderer.recompute_proj_mtx(
                &mut render_quads,
                &self.texture_cache,
                bflyt_view.tree.layout_size.x,
                bflyt_view.tree.layout_size.y,
            );

            self.pane_renderer.update_selection(
                &self.queue,
                &mut render_quads,
                ctx.ui_state.selected_pane,
                &ctx.ui_state.hidden_panes,
                ctx.ui_state.visiblity_flags,
                ctx.ui_state.active_debug_stage,
            );

            self.pane_renderer.flush_mat_buffers(
                &self.queue,
                &render_quads,
                &ctx.ui_state.hidden_panes,
            );
        }

        let paint_jobs = egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: full_output.pixels_per_point,
        };

        if let Some(pending_key_edit) = ctx.ui_state.timeline.pending_key_edit.take() {
            ctx.ui_state
                .timeline
                .anim_player
                .apply_key_edit(&pending_key_edit);
        }

        if let Some(pending_slope_edit) = ctx.ui_state.timeline.pending_slope_edit.take() {
            ctx.ui_state
                .timeline
                .anim_player
                .apply_slope_edit(&pending_slope_edit);
        }

        for (id, delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, delta);
        }
        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        let mut render_encoder = self.device.create_command_encoder(&Default::default());
        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut render_encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        {
            let mut rpass = render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.10,
                            g: 0.10,
                            b: 0.12,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            self.grid_renderer.render_grid(&mut rpass);

            if let Some((sx, sy, sw, sh)) = scissor_rect {
                rpass.set_scissor_rect(sx, sy, sw, sh);
            }

            self.pane_renderer.render(&mut rpass);

            self.selection_renderer.render(&mut rpass);

            if scissor_rect.is_some() {
                rpass.set_scissor_rect(0, 0, self.config.width, self.config.height);
            }

            let mut rpass = rpass.forget_lifetime();
            self.egui_renderer
                .render(&mut rpass, &paint_jobs, &screen_descriptor);
        }

        self.queue.submit(std::iter::once(render_encoder.finish()));
        output.present();
    }
}

pub struct RenderContext<'a> {
    pub camera: &'a Camera,
    pub bflyt_view: Option<&'a mut BflytView>,
    pub ui_state: &'a mut UiState,
}

struct DragState {
    pane_idx: usize,
    handle: Handle,
    start_world: [f32; 2],
    start_translation: Vector3f,
    start_size: Vector2f,
    rotate_z: f32,
}

struct App {
    bflyt_path: Option<PathBuf>,
    bflyt_view: Option<BflytView>,
    ui_state: UiState,
    camera: Camera,
    egui_ctx: egui::Context,
    egui_state: Option<egui_winit::State>,
    gpu: Option<GpuState>,
    window: Option<Arc<Window>>,
    last_tick: Instant,
    drag_state: Option<DragState>,
}

impl App {
    fn new() -> Self {
        Self {
            bflyt_path: None,
            bflyt_view: None,
            ui_state: UiState::default(),
            camera: Camera::new(),
            egui_ctx: egui::Context::default(),
            egui_state: None,
            gpu: None,
            window: None,
            last_tick: Instant::now(),
            drag_state: None,
        }
    }

    pub fn bake_bflyt(&self) -> Option<Bflyt> {
        let view = self.bflyt_view.as_ref()?;

        let mut nodes = Vec::new();
        for root_node in &view.tree.roots {
            root_node.flatten_to_bflyt_nodes(&mut nodes);
        }

        let layout_header = Layout {
            is_centered: view.is_centered,
            width: view.tree.layout_size.x,
            height: view.tree.layout_size.y,
            parts_width: view.parts_size.x,
            parts_height: view.parts_size.y,
            name: view.file_name.clone(),
        };

        Some(Bflyt {
            endianness: 0xFEFF,
            version: view.version,
            layout: layout_header,
            user_data: view.tree.user_data.clone(),
            texture_list: view.tree.texture_list.clone(),
            font_list: view.tree.font_list.clone(),
            material_list: view.tree.material_list.clone(),
            capture_texture_list: view.tree.capture_texture_list.clone(),
            nodes,
            root_group: view.tree.group.clone(),
            control_source: view.tree.control_source.clone(),
        })
    }

    fn try_start_drag(&mut self, screen_pos: [f32; 2]) -> bool {
        let Some(gpu) = &self.gpu else { return false };

        let Some(idx) = self.ui_state.selected_pane else {
            return false;
        };

        let Some(view) = &self.bflyt_view else {
            return false;
        };

        let Some(node) = view.tree.find_by_idx(idx) else {
            return false;
        };

        if node.plain_quad.is_parts_root || node.parts_source.is_some() {
            return false;
        };

        let world_pos = self.camera.screen_to_world(screen_pos);

        let radius = 8.0 / self.camera.zoom.max(0.01);

        let handle = match gpu.selection_renderer.hit_test(world_pos, radius) {
            Some(h) => h,
            None if point_in_quad(world_pos, &node.world_corners) => Handle::Body,
            None => return false,
        };

        let base = node.section.get_base_pane();
        let translation = base.map(|b| b.translation).unwrap_or_default();

        let size = base.map(|b| b.size).unwrap_or(node.world_size);
        let rotate_z = base.map(|b| b.rotation.z).unwrap_or(0.0);

        self.drag_state = Some(DragState {
            pane_idx: idx,
            handle,
            start_world: world_pos,
            start_translation: translation,
            start_size: size,
            rotate_z,
        });
        true
    }

    fn update_drag(&mut self, screen_pos: [f32; 2]) {
        let Some(drag) = &self.drag_state else { return };
        let Some(view) = &mut self.bflyt_view else {
            return;
        };

        let world_pos = self.camera.screen_to_world(screen_pos);

        let dx = world_pos[0] - drag.start_world[0];
        let dy = world_pos[1] - drag.start_world[1];

        let Some(node) = view.tree.find_by_idx_mut(drag.pane_idx) else {
            return;
        };

        if node.plain_quad.is_parts_root {
            return;
        }

        let Some(base) = node.section.get_base_pane_mut() else {
            return;
        };

        match drag.handle {
            Handle::Body => {
                base.translation.x = drag.start_translation.x + dx;
                base.translation.y = drag.start_translation.y - dy;
            }
            Handle::Rotation => {
                let tl = [node.world_corners.top_left.x, node.world_corners.top_left.y];
                let br = [
                    node.world_corners.bottom_right.x,
                    node.world_corners.bottom_right.y,
                ];
                let geom_center_x = (tl[0] + br[0]) * 0.5;
                let geom_center_y = (tl[1] + br[1]) * 0.5;

                let start_v_x = drag.start_world[0] - geom_center_x;
                let start_v_y = drag.start_world[1] - geom_center_y;
                let start_angle = start_v_y.atan2(start_v_x).to_degrees();

                let current_v_x = world_pos[0] - geom_center_x;
                let current_v_y = world_pos[1] - geom_center_y;
                let current_angle = current_v_y.atan2(current_v_x).to_degrees();

                let mut angle_delta = current_angle - start_angle;

                if angle_delta > 180.0 {
                    angle_delta -= 360.0;
                } else if angle_delta < -180.0 {
                    angle_delta += 360.0;
                }

                base.rotation.z = drag.rotate_z - angle_delta;
            }
            _ => {
                let rad = -drag.rotate_z.to_radians();
                let (sin_r, cos_r) = rad.sin_cos();
                let local_dx = dx * cos_r + dy * sin_r;
                let local_dy = -dx * sin_r + dy * cos_r;

                match drag.handle {
                    Handle::TopLeft
                    | Handle::TopRight
                    | Handle::BottomLeft
                    | Handle::BottomRight => {
                        let sx = if matches!(drag.handle, Handle::TopLeft | Handle::BottomLeft) {
                            -1.0
                        } else {
                            1.0
                        };

                        let sy = if matches!(drag.handle, Handle::TopLeft | Handle::TopRight) {
                            -1.0
                        } else {
                            1.0
                        };

                        base.size.x = (drag.start_size.x + local_dx * sx * 2.0).max(1.0);
                        base.size.y = (drag.start_size.y + local_dy * sy * 2.0).max(1.0);
                    }

                    Handle::Left | Handle::Right => {
                        let sx = if drag.handle == Handle::Left {
                            -1.0
                        } else {
                            1.0
                        };
                        base.size.x = (drag.start_size.x + local_dx * sx * 2.0).max(1.0);
                    }

                    Handle::Top | Handle::Bottom => {
                        let sy = if drag.handle == Handle::Top {
                            -1.0
                        } else {
                            1.0
                        };
                        base.size.y = (drag.start_size.y + local_dy * sy * 2.0).max(1.0);
                    }
                    _ => {}
                }
            }
        }

        node.mark_transform_dirty();
        view.tree.recompute_dirty();
    }

    fn end_drag(&mut self) {
        let Some(drag) = self.drag_state.take() else {
            return;
        };

        let Some(view) = &mut self.bflyt_view else {
            return;
        };

        let Some(node) = view.tree.find_by_idx_mut(drag.pane_idx) else {
            return;
        };

        let Some(base) = node.section.get_base_pane() else {
            return;
        };

        let before = edit_history::PaneTransform {
            translation: drag.start_translation,
            size: drag.start_size,
            rotation_z: drag.rotate_z,
        };

        let after = edit_history::PaneTransform {
            translation: base.translation,
            size: base.size,
            rotation_z: base.rotation.z,
        };

        view.history.record_transform(drag.pane_idx, before, after);
    }

    fn try_select_at(&mut self, screen_pos: [f32; 2]) {
        let Some(view) = &self.bflyt_view else { return };
        let world_pos = self.camera.screen_to_world(screen_pos);

        let mut best = None;

        for node in view.tree.iter() {
            if !node.visible
                || self.ui_state.hidden_panes.contains(&node.pane_idx)
                    | node.plain_quad.is_parts_root
                    | node.parts_source.is_some()
            {
                continue;
            }

            if !point_in_quad(world_pos, &node.world_corners) {
                continue;
            }

            best = Some(node.pane_idx);
        }

        self.ui_state.selected_pane = best;
    }

    fn load_file(&mut self) {
        let Some(bflyt_path) = &self.bflyt_path else {
            return;
        };

        let bytes = std::fs::read(bflyt_path).expect("read bflyt file");
        let mut detected_files = Vec::new();
        extract_all_files_recursive(bytes, &mut detected_files);

        self.load_file_from_buffer(detected_files);
    }

    fn load_file_from_buffer(&mut self, all_files: Vec<MagicFiles>) {
        let bflyt_result = all_files.iter().find_map(|file| {
            if let MagicFiles::Bflyt(bytes) = file {
                Some(Bflyt::parse_file(bytes))
            } else {
                None
            }
        });

        let bflyt = match bflyt_result {
            Some(Ok(parsed_bflyt)) => parsed_bflyt,
            Some(Err(err)) => {
                self.ui_state.error_message =
                    Some(format!("Failed to parse BFLYT layout: {err:?}"));
                log::error!("BFLYT parsing error: {err:?}");
                return;
            }
            None => {
                self.ui_state.error_message =
                    Some("No BFLYT file found in data payload.".to_string());
                log::error!("No BFLYT file found in container.");
                return;
            }
        };

        self.ui_state.error_message = None;

        let has_textures = all_files.iter().any(|f| matches!(f, MagicFiles::Bntx(_)));

        let layout_name = bflyt.layout.name.clone();
        let mut view = build_view(
            bflyt,
            self.ui_state.archive_browser.layout_dir.as_deref(),
            layout_name.clone(),
            has_textures,
            self.ui_state
                .archive_browser
                .archive_scan
                .as_ref()
                .map(|s| s.entries.as_slice()),
        );

        self.ui_state.timeline.anim_player = AnimPlayer::new();

        for magic_file in all_files {
            match magic_file {
                MagicFiles::Bntx(bytes) => {
                    view.tree.discovered_bntx_buffers.push(bytes);
                }
                MagicFiles::Bflan(bytes) => {
                    if let Ok(bflan) = Bflan::parse_file(&bytes) {
                        self.ui_state.timeline.anim_player.load(bflan);
                    }
                }
                _ => {}
            }
        }

        self.ui_state.anim_names = self
            .ui_state
            .timeline
            .anim_player
            .anims
            .iter_mut()
            .enumerate()
            .map(|(idx, a)| {
                if a.name.is_empty() {
                    a.name = format!("Animation {}", idx + 1)
                };

                a.name.clone()
            })
            .collect();

        self.camera.zoom = 1.0;
        self.camera.offset = [0.0, 0.0];
        self.bflyt_view = Some(view);

        if let Some(gpu) = &mut self.gpu {
            let view = self.bflyt_view.as_mut().unwrap();

            let render_quads = view.tree.collect_render_quads();

            for bntx_bytes in &view.tree.discovered_bntx_buffers {
                gpu.texture_cache
                    .load_from_bntx_bytes(&gpu.device, &gpu.queue, bntx_bytes);
            }

            gpu.pane_renderer.upload_quads(
                &gpu.device,
                &render_quads,
                &gpu.texture_cache,
                view.tree.layout_size.x,
                view.tree.layout_size.y,
            );

            if let Some(window) = &self.window {
                let size = window.inner_size();
                self.camera.fit(
                    view.tree.layout_size.x,
                    view.tree.layout_size.y,
                    size.width as f32,
                    size.height as f32,
                );

                window.set_title(&format!("nnbfl-preview - {}", &view.file_name));
            }
        }

        log::info!(
            "Loaded {} panes from {layout_name}",
            self.bflyt_view.as_ref().unwrap().tree.iter().count(),
        );
    }

    fn extract_blarc_from_sarc_bytes(&self, path: &PathBuf) -> Option<Vec<MagicFiles>> {
        let mut file_bytes = std::fs::read(path).ok()?;
        let filename = path.file_name()?.to_string_lossy();

        file_bytes = decompress_if_needed(file_bytes, &filename);

        let mut all_files = Vec::new();
        extract_all_files_recursive(file_bytes, &mut all_files);

        let has_bflyt = all_files.iter().any(|f| matches!(f, MagicFiles::Bflyt(_)));
        if !has_bflyt {
            return None;
        }

        Some(all_files)
    }

    fn perform_pane_edit(&mut self, edit: edit_history::PaneEdit) {
        let Some(view) = &mut self.bflyt_view else {
            return;
        };

        let target_idx = match &edit {
            edit_history::PaneEdit::Delete { target_idx } => Some(*target_idx),
            edit_history::PaneEdit::Duplicate { source_idx } => Some(*source_idx),
            edit_history::PaneEdit::Insert { .. } => None,
        };
        if let Some(idx) = target_idx
            && view
                .tree
                .find_by_idx(idx)
                .is_some_and(|n| n.parts_source.is_some())
        {
            return;
        }

        let resulting_idx = view.history.perform(&mut view.tree, edit);
        if resulting_idx.is_some() {
            self.ui_state.selected_pane = resulting_idx;
        } else {
            self.ui_state.selected_pane = None;
        }

        self.after_structural_edit();
    }

    fn after_structural_edit(&mut self) {
        let Some(view) = &mut self.bflyt_view else {
            return;
        };

        view.tree.recompute_dirty();

        let Some(gpu) = &mut self.gpu else { return };
        let render_quads = view.tree.collect_render_quads();

        gpu.pane_renderer.upload_quads(
            &gpu.device,
            &render_quads,
            &gpu.texture_cache,
            view.tree.layout_size.x,
            view.tree.layout_size.y,
        );

        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}

pub fn extract_all_files_recursive(data: Vec<u8>, out_files: &mut Vec<MagicFiles>) {
    let current_file = SarcFile {
        name: None,
        hash: 0,
        data,
    };

    match current_file.match_by_magic() {
        MagicFiles::Zstd(compressed_data) => {
            let mut decompressed = Vec::new();

            if tomolib::formats::zs::decompress(&compressed_data[..], &mut decompressed).is_ok() {
                extract_all_files_recursive(decompressed, out_files);
            } else {
                log::error!("Failed to decompress Zstd data.");
                out_files.push(MagicFiles::Unknown(compressed_data));
            }
        }

        MagicFiles::Yaz0(compressed_data) => match szs::decode(&compressed_data) {
            Ok(decompressed) => {
                extract_all_files_recursive(decompressed, out_files);
            }
            Err(err) => {
                log::error!("Failed to decompress Yaz0 data: {err}");
                out_files.push(MagicFiles::Unknown(compressed_data));
            }
        },

        MagicFiles::Sarc(sarc_bytes) => {
            if let Ok(sarc) = Sarc::parse_file(&sarc_bytes) {
                for file in sarc.files {
                    extract_all_files_recursive(file.data, out_files);
                }
            } else {
                out_files.push(MagicFiles::Sarc(sarc_bytes));
            }
        }

        MagicFiles::Bflyt(bytes) => out_files.push(MagicFiles::Bflyt(bytes)),
        MagicFiles::Bflan(bytes) => out_files.push(MagicFiles::Bflan(bytes)),
        MagicFiles::Bntx(bytes) => out_files.push(MagicFiles::Bntx(bytes)),
        MagicFiles::Msbt(bytes) => out_files.push(MagicFiles::Msbt(bytes)),
        MagicFiles::Msbp(bytes) => out_files.push(MagicFiles::Msbp(bytes)),

        MagicFiles::Unknown(bytes) => {
            out_files.push(MagicFiles::Unknown(bytes));
        }
    }
}

fn decompress_if_needed(data: Vec<u8>, filename: &str) -> Vec<u8> {
    if data.len() >= 4 && data[0..4] == [0x28, 0xB5, 0x2F, 0xFD] {
        let mut decompressed = Vec::new();

        if tomolib::formats::zs::decompress(&data[..], &mut decompressed).is_ok() {
            return decompressed;
        } else {
            log::error!("Failed to decompress Zstd file: {filename}");
        }
    }

    data
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let title_path = if let Some(bflyt_path) = &self.bflyt_path {
            bflyt_path.file_name().unwrap().to_string_lossy()
        } else {
            "No file loaded".into()
        };

        let icon_bytes = include_bytes!("../assets/icon.rgba");
        let (width, height) = (64, 64);

        let icon = winit::window::Icon::from_rgba(icon_bytes.to_vec(), width, height).ok();

        let mut window_attributes = winit::window::WindowAttributes::default()
            .with_title(format!("nnbfl-preview - {title_path}"))
            .with_inner_size(winit::dpi::LogicalSize::new(1280u32, 720u32))
            .with_window_icon(icon);

        #[cfg(any(
            target_os = "linux",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            use winit::platform::wayland::WindowAttributesExtWayland;
            window_attributes = window_attributes.with_name("nnbfl-preview", "nnbfl-preview");
        }

        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("create window"),
        );

        let egui_state = egui_winit::State::new(
            self.egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            None,
            None,
            None,
        );

        if let Err(err) = setup_chinese_fonts(&self.egui_ctx) {
            match err {
                FontError::NotFound(e) => log::warn!("CJK font not found: {e}"),
                FontError::ReadError(e) => log::warn!("CJK font read error: {e}"),
                FontError::UnsupportedPlatform => log::warn!("CJK font platform unsupported"),
            }
        };

        self.load_file();

        let size = window.inner_size();
        self.camera
            .fit(1280.0, 720.0, size.width as f32, size.height as f32);

        let gpu = GpuState::new(window.clone());

        self.egui_state = Some(egui_state);
        self.gpu = Some(gpu);
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let (Some(state), Some(window)) = (&mut self.egui_state, &self.window) {
            let _ = state.on_window_event(window, &event);
        }

        match &event {
            WindowEvent::CursorMoved { .. }
            | WindowEvent::MouseInput { .. }
            | WindowEvent::MouseWheel { .. }
            | WindowEvent::KeyboardInput { .. } => {
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }

        let egui_wants_pointer = self.egui_ctx.egui_wants_pointer_input();
        let egui_wants_scroll = self.egui_ctx.egui_wants_pointer_input();

        if let Some(action) = self.ui_state.pending_action.take() {
            match action {
                UiAction::SetBlarcDir => {
                    let Some(dir) = rfd::FileDialog::new().pick_folder() else {
                        return;
                    };

                    self.ui_state.archive_browser.layout_dir = Some(dir);
                    if let Some(path) = self.bflyt_path.clone() {
                        let bytes = std::fs::read(&path).ok();

                        if let Some(bytes) = bytes {
                            self.load_file_from_buffer(vec![MagicFiles::Bflyt(bytes)]);
                        }
                    }
                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
                }

                UiAction::LoadFile => {
                    let Some(path) = rfd::FileDialog::new()
                        .add_filter(
                            "Supported files",
                            &[SUPPORTED_SARC_EXTENSIONS, &["bflyt"]].concat(),
                        )
                        .pick_file()
                    else {
                        return;
                    };

                    let path_str = path.to_string_lossy().to_lowercase();

                    let is_sarc = SUPPORTED_SARC_EXTENSIONS
                        .iter()
                        .any(|ext| path_str.ends_with(&format!(".{}", ext.to_lowercase())));

                    self.ui_state.reset();

                    if is_sarc {
                        if let Some(all_files) = self.extract_blarc_from_sarc_bytes(&path) {
                            self.load_file_from_buffer(all_files);
                        }
                    } else {
                        self.bflyt_path = Some(path);
                        self.load_file();
                    }

                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
                }

                UiAction::SaveFile => {
                    log::info!("i want to save file");

                    if self.bflyt_view.is_some() {
                        let mut dialog = rfd::FileDialog::new()
                            .set_title("Save as .bflyt")
                            .add_filter("BFLYT Layout", &["bflyt"]);

                        if let Some(path) = &self.bflyt_path
                            && let Some(name) = path.file_name()
                        {
                            dialog = dialog.set_file_name(name.to_string_lossy());
                        }

                        if let Some(target_path) = dialog.save_file() {
                            if let Some(baked_bflyt) = self.bake_bflyt() {
                                let writer = baked_bflyt.write_file();

                                match std::fs::write(&target_path, &writer.buffer) {
                                    Ok(_) => {
                                        log::info!("File succesfully written to: {target_path:?}",);
                                    }
                                    Err(e) => {
                                        log::error!("Failed writing to disk: {e:?}");
                                        self.ui_state.error_message =
                                            Some(format!("Write error: {e}"));
                                    }
                                }
                            } else {
                                log::error!(
                                    "Failed baking layout fields into file model layout structures."
                                );
                            }
                        }
                    }
                }

                UiAction::StartArchiveScan => {
                    if let Some(dir) = self.ui_state.archive_browser.layout_dir.clone() {
                        self.ui_state.archive_browser.archive_scan = Some(ArchiveScan::start(dir));
                        if let Some(w) = &self.window {
                            w.request_redraw();
                        }
                    }
                }

                UiAction::CancelArchiveScan => {
                    if let Some(scan) = &mut self.ui_state.archive_browser.archive_scan {
                        scan.request_cancel();
                    }
                }

                UiAction::LoadArchiveEntry(entry) => {
                    let resolved = std::fs::read(&entry.path).ok().and_then(|bytes| {
                        archive_browser::resolve_nested_package_bytes(bytes, &entry.nested_path)
                    });

                    self.ui_state.reset();

                    match resolved {
                        Some(package_bytes) => {
                            let mut all_files = Vec::new();
                            extract_all_files_recursive(package_bytes, &mut all_files);

                            let mut final_files = Vec::new();
                            let mut target_layout = None;

                            for (idx, file) in all_files.into_iter().enumerate() {
                                if matches!(file, MagicFiles::Bflyt(_)) && idx == entry.file_idx {
                                    target_layout = Some(file);
                                } else {
                                    final_files.push(file);
                                }
                            }

                            if let Some(target) = target_layout {
                                final_files.insert(0, target);
                                self.load_file_from_buffer(final_files);
                            } else {
                                self.load_file_from_buffer(final_files);
                            }
                        }

                        None => {
                            self.ui_state.error_message =
                                Some(format!("Failed to unpack '{}'.", entry.display_name));
                        }
                    }

                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
                }

                UiAction::DeletePane(target_idx) => {
                    self.perform_pane_edit(edit_history::PaneEdit::Delete { target_idx })
                }

                UiAction::DuplicatePane(source_idx) => {
                    self.perform_pane_edit(edit_history::PaneEdit::Duplicate { source_idx })
                }

                UiAction::Undo => {
                    if let Some(view) = &mut self.bflyt_view {
                        let resulting_idx = view.history.undo(&mut view.tree);
                        self.ui_state.selected_pane = resulting_idx;
                        self.after_structural_edit();
                    }
                }

                UiAction::Redo => {
                    if let Some(view) = &mut self.bflyt_view {
                        let resulting_idx = view.history.redo(&mut view.tree);
                        self.ui_state.selected_pane = resulting_idx;
                        self.after_structural_edit();
                    }
                }
            }
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::CursorMoved { position, .. } => {
                let pos = [position.x as f32, position.y as f32];
                self.camera.cursor_screen = pos;

                if self.camera.is_panning && !egui_wants_pointer {
                    self.camera.pan(pos);
                }

                if self.drag_state.is_some() && !egui_wants_pointer {
                    self.update_drag(pos);
                }

                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }

            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => match state {
                winit::event::ElementState::Pressed => {
                    if !egui_wants_pointer {
                        let pos = self.camera.cursor_screen;

                        if !self.try_start_drag(pos) {
                            self.try_select_at(pos);
                            self.try_start_drag(pos);
                        }
                    }
                }
                winit::event::ElementState::Released => {
                    self.end_drag();
                }
            },

            WindowEvent::MouseInput {
                state,
                button: MouseButton::Middle,
                ..
            } => match state {
                winit::event::ElementState::Pressed => {
                    if !egui_wants_pointer {
                        self.camera.start_pan(self.camera.cursor_screen);
                    }
                }
                winit::event::ElementState::Released => self.camera.end_pan(),
            },

            WindowEvent::MouseInput {
                state,
                button: MouseButton::Right,
                ..
            } if state == winit::event::ElementState::Pressed
                && !egui_wants_pointer
                && let Some(pane_idx) = self.ui_state.selected_pane =>
            {
                self.ui_state
                    .context_menu
                    .open_context_menu(self.camera.cursor_screen, pane_idx);
            }

            WindowEvent::MouseWheel { delta, .. } if !egui_wants_scroll => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 * 0.01,
                };

                self.camera.zoom_around_cursor(lines);
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }

            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size);
                }

                if let Some(view) = &self.bflyt_view {
                    self.camera.fit(
                        view.tree.layout_size.x,
                        view.tree.layout_size.y,
                        size.width as f32,
                        size.height as f32,
                    );
                }
            }

            WindowEvent::DroppedFile(path) => {
                if path.extension().and_then(|s| s.to_str()) == Some("bflyt") {
                    self.bflyt_path = Some(path);
                    self.load_file();
                } else {
                    self.ui_state.error_message =
                        Some("Invalid file type. Please drop a .bflyt file".to_string());
                }
            }

            WindowEvent::RedrawRequested => {
                if let (Some(gpu), Some(window), Some(egui_state)) =
                    (&mut self.gpu, &self.window, &mut self.egui_state)
                {
                    if window.has_focus() {
                        let dt = self.last_tick.elapsed().as_secs_f32();
                        self.last_tick = Instant::now();

                        if let Some(next) = self
                            .ui_state
                            .timeline
                            .anim_player
                            .tick(dt, self.ui_state.timeline.frame_rate)
                        {
                            let anim_idx = self
                                .ui_state
                                .timeline
                                .anim_player
                                .anims
                                .iter()
                                .position(|a| a.name == next);

                            self.ui_state.timeline.anim_player.play(
                                anim_idx,
                                self.bflyt_view.as_ref(),
                                &mut self.ui_state.hidden_panes,
                            );
                        }

                        if let Some(anim_idx) = self.ui_state.pending_play_anim.take() {
                            self.ui_state.timeline.anim_player.play(
                                Some(anim_idx),
                                self.bflyt_view.as_ref(),
                                &mut self.ui_state.hidden_panes,
                            );
                        }

                        if let Some(view) = &mut self.bflyt_view {
                            view.reset_to_base();
                            self.ui_state.timeline.anim_player.apply(view);
                        }
                    }

                    gpu.render(
                        window,
                        &self.egui_ctx,
                        egui_state,
                        RenderContext {
                            bflyt_view: self.bflyt_view.as_mut(),
                            ui_state: &mut self.ui_state,
                            camera: &self.camera,
                        },
                    );

                    let scan_active = self
                        .ui_state
                        .archive_browser
                        .archive_scan
                        .as_mut()
                        .map(|s| s.poll())
                        .unwrap_or(false);

                    let scan_in_progress = self
                        .ui_state
                        .archive_browser
                        .archive_scan
                        .as_ref()
                        .is_some_and(|s| !s.done && !s.cancelled);

                    if (self.ui_state.timeline.anim_player.is_playing()
                        || scan_active
                        || scan_in_progress)
                        && window.has_focus()
                    {
                        window.request_redraw();
                    }
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.egui_ctx.has_requested_repaint()
            && let Some(window) = &self.window
        {
            window.request_redraw();
        }
    }
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().expect("create event loop");
    let mut app = App::new();

    event_loop.run_app(&mut app).expect("run event loop");
}
