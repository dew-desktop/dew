use aether_raster::windowed::Windowed;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

pub struct GpuPresenter {
    pub windowed: Windowed,
}

impl GpuPresenter {
    pub fn attach(hwnd: HWND, width: u16, height: u16) -> Result<Self, String> {
        let hinstance = unsafe { GetModuleHandleW(None).map_err(|e| e.to_string())? };
        let windowed = Windowed::attach(hwnd.0 as isize, hinstance.0 as isize, width, height)
            .ok_or_else(|| "Failed to attach aether_raster Windowed GPU presenter to HWND".to_string())?;

        Ok(Self { windowed })
    }

    pub fn begin_frame(&mut self, bg_r: u8, bg_g: u8, bg_b: u8) {
        self.windowed.begin(bg_r, bg_g, bg_b);
    }

    pub fn draw_rounded_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    ) {
        if let Some(path) = Windowed::path(x, y, w, h, radius) {
            self.windowed.fill(&path, r, g, b, a);
        }
    }

    pub fn stroke_rounded_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        stroke_width: f32,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    ) {
        if let Some(path) = Windowed::path(x, y, w, h, radius) {
            self.windowed.stroke(&path, stroke_width, r, g, b, a);
        }
    }

    pub fn push_clip(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.windowed.clip_push(x, y, w, h);
    }

    pub fn pop_clip(&mut self) {
        self.windowed.clip_pop();
    }

    pub fn end_frame(&mut self) -> bool {
        self.windowed.present()
    }
}
