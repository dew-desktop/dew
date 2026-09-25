//! `wgpu` presentation for an ordinary `Window` surface.
//!
//! Milestone 24, sprint 1: this replaces `blit`'s `SetDIBitsToDevice` path, the
//! one GDI measured to cost 25-50ms per present regardless of resize. `wgpu`
//! rather than raw DXGI because Dew's ordinary-window rendering path is meant
//! to carry over to Linux and Mac unchanged, not be re-derived per platform --
//! see the milestone's vision overview for why the layered surface (widgets,
//! overlays) does NOT follow this crate and stays `DirectComposition`.
//!
//! NO RENDER PIPELINE, NO SHADER. `dew_raster` already produced a finished
//! BGRA buffer; the only job here is getting it onto the screen, so the
//! surface's current texture is configured with `COPY_DST` and written to
//! directly with `Queue::write_texture`, the same shape of operation the GDI
//! blit it replaces performed.

use windows::Win32::Foundation::HWND;

/// An owned copy of a Win32 window handle, independent of the `Window` that
/// produced it.
///
/// `wgpu::Instance::create_surface` wants a target that can outlive the
/// surface. `Window` itself cannot be handed over — the caller keeps using it
/// for input and lifetime — but an `HWND` is already just an integer the OS
/// resolves, so a copy of it is exactly as valid as the original for as long
/// as the real window is alive, which the caller (this crate's own `Window`)
/// already guarantees.
#[derive(Clone, Copy)]
struct RawWin32Handle(isize);

impl raw_window_handle::HasWindowHandle for RawWin32Handle {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        let hwnd = std::num::NonZeroIsize::new(self.0)
            .ok_or(raw_window_handle::HandleError::Unavailable)?;
        let handle = raw_window_handle::Win32WindowHandle::new(hwnd);
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(handle.into()) })
    }
}

impl raw_window_handle::HasDisplayHandle for RawWin32Handle {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        let handle = raw_window_handle::WindowsDisplayHandle::new();
        Ok(unsafe { raw_window_handle::DisplayHandle::borrow_raw(handle.into()) })
    }
}

/// Presents a BGRA buffer to an ordinary window through a `wgpu` swap chain.
pub struct Presenter {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
}

impl Presenter {
    pub fn new(hwnd: HWND, width: u32, height: u32) -> Result<Presenter, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());

        let surface = instance
            .create_surface(RawWin32Handle(hwnd.0 as isize))
            .map_err(|e| format!("creating a wgpu surface for this window: {e:?}"))?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .map_err(|e| format!("no graphics adapter can present to this window: {e:?}"))?;

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("dew_window::Presenter"),
            ..Default::default()
        }))
        .map_err(|e| format!("opening a device on the chosen adapter: {e:?}"))?;

        let caps = surface.get_capabilities(&adapter);
        if !caps.usages.contains(wgpu::TextureUsages::COPY_DST) {
            // MEASURED, NOT ASSUMED: `RENDER_ATTACHMENT` alone is what wgpu
            // guarantees a swap-chain texture supports. Writing a finished
            // BGRA buffer straight into it also needs `COPY_DST`, which this
            // backend does not report here -- a render pipeline would be the
            // fallback, and is deliberately not built speculatively for a
            // backend nothing has yet exercised.
            return Err(format!(
                "this adapter's surface does not support COPY_DST, only {:?}; \
                 direct-write presentation needs a render-pipeline fallback \
                 this sprint does not build speculatively",
                caps.usages
            ));
        }
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| *f == wgpu::TextureFormat::Bgra8Unorm)
            .unwrap_or(caps.formats[0]);

        let mut presenter = Presenter {
            device,
            queue,
            surface,
            format,
            width: 0,
            height: 0,
        };
        presenter.configure(width, height);
        Ok(presenter)
    }

    /// (Re)configure the swap chain at a new size. A no-op at zero either way
    /// -- `wgpu` panics on a zero-sized configuration, and a window mid-resize
    /// can report one for a single message.
    pub fn configure(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.width = width;
        self.height = height;
        self.surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
                format: self.format,
                width,
                height,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            },
        );
    }

    /// Copy a BGRA buffer into the swap chain's current texture and present
    /// it. `bgra` must be `width * height * 4` bytes.
    pub fn present(&mut self, bgra: &[u8], width: u32, height: u32) {
        if bgra.len() < (width * height * 4) as usize {
            return;
        }
        if width != self.width || height != self.height {
            self.configure(width, height);
        }

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            // OUTDATED IS EXPECTED DURING A LIVE RESIZE: the surface changed
            // size since it was last configured. Reconfigure and skip this
            // frame rather than presenting a stale one.
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.configure(width, height);
                return;
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Lost
            | wgpu::CurrentSurfaceTexture::Validation => return,
        };

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &frame.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bgra,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::empty());
        frame.present();
    }
}
