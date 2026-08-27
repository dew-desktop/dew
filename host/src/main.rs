pub mod luau;
pub mod presenter;
pub mod window;

use luau::LuauRuntime;
use presenter::GpuPresenter;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use window::NativeWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE, WM_QUIT,
};

fn find_mods_dir() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("mods"),
        PathBuf::from("../mods"),
        PathBuf::from("../../mods"),
        PathBuf::from("../../../mods"),
    ];
    for c in &candidates {
        if c.exists() {
            return Some(c.clone());
        }
    }
    if let Ok(exe_path) = std::env::current_exe() {
        let mut cur = exe_path.parent();
        while let Some(parent) = cur {
            let candidate = parent.join("mods");
            if candidate.exists() {
                return Some(candidate);
            }
            cur = parent.parent();
        }
    }
    None
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("💧 Dew Native Host (Vello GPU + Luau Runtime) Starting...");

    let width = 380;
    let height = 56;

    // 1. Create Native Borderless Window with Hardware Rounded Pill Clipping
    let mut window = NativeWindow::new("Dew HUD", width, height)?;
    println!("[Dew Host] Created native Win32 window (HWND: {:?})", window.hwnd.0);

    // 2. Attach aether_raster Vello GPU swapchain
    let mut presenter = GpuPresenter::attach(window.hwnd, width, height)?;
    println!("[Dew Host] Attached aether_raster Vello GPU presenter");

    // 3. Initialize Luau Runtime
    let runtime = LuauRuntime::new()?;
    println!("[Dew Host] Initialized embedded Luau VM");

    // Discover & load mods
    if let Some(mods_path) = find_mods_dir() {
        println!("[Dew Host] Discovered mods directory at: {}", mods_path.display());
        runtime.load_mods_from_dir(&mods_path);
    } else {
        println!("[Dew Host] Warning: mods directory not found");
    }

    println!("[Dew Host] Entering 120 FPS GPU presentation loop...");

    let frame_duration = Duration::from_micros(8333); // ~120 FPS
    let mut last_frame = Instant::now();

    // 4. Main Event & Render Loop
    loop {
        // Process Windows OS messages
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    println!("[Dew Host] Exiting cleanly.");
                    return Ok(());
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        // Process Clicks
        let clicks = window.drain_clicks();
        for (click_x, click_y) in clicks {
            println!("[Dew Host] Click detected at ({}, {}) -> Triggering mod click", click_x, click_y);
            let _ = runtime.handle_click("timetracker");
        }

        // 1. Clear background surface (Deep OLED Obsidian)
        presenter.begin_frame(13, 17, 23);

        // 2. Draw Frosted HUD Pill Card
        presenter.draw_rounded_rect(
            0.0,
            0.0,
            width as f32,
            height as f32,
            28.0,
            18,
            24,
            38,
            245,
        );

        // 3. Draw Ambient Glow Border
        presenter.stroke_rounded_rect(
            0.5,
            0.5,
            (width - 1) as f32,
            (height - 1) as f32,
            27.5,
            1.0,
            56,
            189,
            248,
            140, // Cyan Ambient Glow
        );

        // 4. Render dynamic commands emitted by Luau/Aether
        let commands = runtime.render_frame();
        for cmd in commands {
            match cmd.kind.as_str() {
                "rounded_rect" => {
                    presenter.draw_rounded_rect(
                        cmd.x,
                        cmd.y,
                        cmd.w,
                        cmd.h,
                        cmd.radius,
                        cmd.color_r,
                        cmd.color_g,
                        cmd.color_b,
                        cmd.color_a,
                    );
                }
                "stroke" => {
                    presenter.stroke_rounded_rect(
                        cmd.x,
                        cmd.y,
                        cmd.w,
                        cmd.h,
                        cmd.radius,
                        cmd.stroke_width,
                        cmd.color_r,
                        cmd.color_g,
                        cmd.color_b,
                        cmd.color_a,
                    );
                }
                "text" => {
                    if let Some(ref text) = cmd.text {
                        let is_mono = cmd.radius > 0.0;
                        presenter.draw_text(
                            text,
                            cmd.x,
                            cmd.y,
                            cmd.stroke_width.max(11.0),
                            is_mono,
                            cmd.color_r,
                            cmd.color_g,
                            cmd.color_b,
                            cmd.color_a,
                        );
                    }
                }
                _ => {}
            }
        }

        // 5. Present to Win32 Surface via Vello GPU Swapchain
        presenter.end_frame();

        // Frame rate pacing
        let elapsed = last_frame.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
        last_frame = Instant::now();
    }
}
