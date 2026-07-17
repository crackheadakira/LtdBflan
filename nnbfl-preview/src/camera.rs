pub struct Camera {
    pub offset: [f32; 2],
    pub zoom: f32,

    pub is_panning: bool,
    pub last_cursor: Option<[f32; 2]>,
    pub cursor_screen: [f32; 2],
}

impl Camera {
    pub fn new() -> Self {
        Self {
            offset: [0.0, 0.0],
            zoom: 1.0,
            is_panning: false,
            last_cursor: None,
            cursor_screen: [0.0, 0.0],
        }
    }

    pub fn fit(&mut self, layout_w: f32, layout_h: f32, viewport_w: f32, viewport_h: f32) {
        let pad = 40.0;
        let scale_x = (viewport_w - pad * 2.0) / layout_w;
        let scale_y = (viewport_h - pad * 2.0) / layout_h;
        self.zoom = scale_x.min(scale_y);

        let scaled_w = layout_w * self.zoom;
        let scaled_h = layout_h * self.zoom;
        self.offset = [(viewport_w - scaled_w) * 0.5, (viewport_h - scaled_h) * 0.5];
    }

    pub fn zoom_around_cursor(&mut self, delta: f32) {
        let factor = if delta > 0.0 { 1.1f32 } else { 1.0 / 1.1 };
        let new_zoom = (self.zoom * factor).clamp(0.01, 100.0);

        let cx = self.cursor_screen[0];
        let cy = self.cursor_screen[1];
        let ratio = new_zoom / self.zoom;
        self.offset[0] = cx - ratio * (cx - self.offset[0]);
        self.offset[1] = cy - ratio * (cy - self.offset[1]);
        self.zoom = new_zoom;
    }

    pub fn start_pan(&mut self, pos: [f32; 2]) {
        self.is_panning = true;
        self.last_cursor = Some(pos);
    }

    pub fn end_pan(&mut self) {
        self.is_panning = false;
        self.last_cursor = None;
    }

    pub fn pan(&mut self, pos: [f32; 2]) {
        if let Some(last) = self.last_cursor {
            self.offset[0] += pos[0] - last[0];
            self.offset[1] += pos[1] - last[1];
        }
        self.last_cursor = Some(pos);
    }

    pub fn build_matrix(&self, viewport_w: f32, viewport_h: f32) -> [[f32; 4]; 4] {
        puffin::profile_function!();
        let sx = 2.0 / viewport_w;
        let sy = -2.0 / viewport_h;

        //   col0 = [sx*zoom, 0,        0, 0]
        //   col1 = [0,       sy*zoom,  0, 0]
        //   col2 = [0,       0,        1, 0]
        //   col3 = [sx*offset_x - 1,  sy*offset_y + 1, 0, 1]
        let zoom = self.zoom;
        let tx = sx * self.offset[0] - 1.0;
        let ty = sy * self.offset[1] + 1.0;

        [
            [sx * zoom, 0.0, 0.0, 0.0],
            [0.0, sy * zoom, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [tx, ty, 0.0, 1.0],
        ]
    }

    pub fn world_to_screen(
        &self,
        world_pos: [f32; 2],
        viewport_w: f32,
        viewport_h: f32,
    ) -> egui::Pos2 {
        let sx = 2.0 / viewport_w;
        let sy = -2.0 / viewport_h;

        let tx = sx * self.offset[0] - 1.0;
        let ty = sy * self.offset[1] + 1.0;

        let ndc_x = world_pos[0] * sx * self.zoom + tx;
        let ndc_y = world_pos[1] * sy * self.zoom + ty;

        let screen_x = ((ndc_x + 1.0) / 2.0) * viewport_w;
        let screen_y = ((1.0 - ndc_y) / 2.0) * viewport_h;

        egui::pos2(screen_x, screen_y)
    }

    pub fn screen_to_world(&self, screen_pos: [f32; 2]) -> [f32; 2] {
        [
            (screen_pos[0] - self.offset[0]) / self.zoom,
            (screen_pos[1] - self.offset[1]) / self.zoom,
        ]
    }
}
