use aether_raster::text::FontStore;
use aether_raster::windowed::Windowed;
use std::fs;
use vello_cpu::Glyph;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

pub struct GpuPresenter {
    pub windowed: Windowed,
    pub fonts: FontStore,
    pub primary_font_id: u32,
    pub mono_font_id: u32,
}

impl GpuPresenter {
    pub fn attach(hwnd: HWND, width: u16, height: u16) -> Result<Self, String> {
        let hinstance = unsafe { GetModuleHandleW(None).map_err(|e| e.to_string())? };
        let windowed = Windowed::attach(hwnd.0 as isize, hinstance.0 as isize, width, height)
            .ok_or_else(|| "Failed to attach aether_raster Windowed GPU presenter to HWND".to_string())?;

        let mut fonts = FontStore::default();

        // Load System Fonts (Segoe UI for UI labels, Segoe UI Semibold for bold, Consolas/Cascadia for timer)
        let primary_font_id = if let Ok(data) = fs::read("C:\\Windows\\Fonts\\segoeui.ttf") {
            fonts.load(data, 0)
        } else {
            0
        };

        let mono_font_id = if let Ok(data) = fs::read("C:\\Windows\\Fonts\\consola.ttf") {
            fonts.load(data, 0)
        } else if let Ok(data) = fs::read("C:\\Windows\\Fonts\\CascadiaCode.ttf") {
            fonts.load(data, 0)
        } else {
            primary_font_id
        };

        Ok(Self {
            windowed,
            fonts,
            primary_font_id,
            mono_font_id,
        })
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

    pub fn draw_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        is_mono: bool,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    ) {
        let font_id = if is_mono {
            self.mono_font_id
        } else {
            self.primary_font_id
        };

        if let Some(font_entry) = self.fonts.get(font_id) {
            if let Some(run) = self.fonts.layout(font_id, size, text) {
                let baseline_y = y + run.ascent;
                let glyphs_iter = run.glyphs.iter().map(|g| Glyph {
                    id: g.id,
                    x: x + g.x,
                    y: baseline_y + g.y,
                });

                self.windowed
                    .glyphs(&font_entry.data, size, (r, g, b, a), glyphs_iter);
            }
        }
    }

    pub fn end_frame(&mut self) -> bool {
        self.windowed.present()
    }
}
