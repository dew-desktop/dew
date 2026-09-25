//! `wgpu` presentation for a Win32 window, composed through `DirectComposition`.
//!
//! Milestone 24: this replaces `blit`'s `SetDIBitsToDevice` path, the one GDI
//! measured to cost 25-50ms per present regardless of resize.
//!
//! ## Why `DirectComposition` for an ORDINARY window too, not only a layered one
//!
//! The milestone's original plan bound an ordinary `Window` surface's swap
//! chain directly to its `HWND`, on the assumption that `DirectComposition`
//! was only needed for a layered surface's per-pixel alpha. Measured, not
//! assumed, that assumption was wrong: a flip-model swap chain bound directly
//! to an `HWND` is a well-documented source of a "wrong direction" resize
//! artifact -- during a live drag, Windows composites the swap chain's back
//! buffer against the window's CURRENT client rect independent of when the
//! app calls `Present`, so a buffer sized for the last configured size gets
//! transiently stretched against a rect that has already moved on. The fix
//! the industry actually uses is `DirectComposition`
//! (`WS_EX_NOREDIRECTIONBITMAP` plus an `IDCompositionVisual`) even for an
//! opaque window, because it is what removes the swap chain from the
//! window's own redirection surface entirely -- not a transparency feature
//! here, a resize-correctness one.
//!
//! `wgpu`'s DX12 backend accepts an `IDCompositionVisual` directly
//! (`SurfaceTargetUnsafe::CompositionVisual`), so this still does not need a
//! hand-rolled DXGI swap chain -- only the composition device, target and
//! visual DirectComposition itself requires, which `wgpu` cannot set up on
//! this crate's behalf because it has no notion of the window's message loop
//! or its `WS_EX_NOREDIRECTIONBITMAP` style.
//!
//! NO RENDER PIPELINE, NO SHADER. `dew_raster` already produced a finished
//! BGRA buffer; the only job here is getting it onto the screen, so the
//! surface's current texture is configured with `COPY_DST` and written to
//! directly with `Queue::write_texture`, the same shape of operation the GDI
//! blit it replaces performed.

use windows::core::Interface;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice3, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual,
};

/// Presents a BGRA buffer to a window through a `wgpu` swap chain composed
/// as a `DirectComposition` visual's content.
pub struct Presenter {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    // KEPT ALIVE FOR THE VISUAL TREE'S LIFETIME, NEVER READ AGAIN. Dropping
    // any of these tears down what `Commit` published: the target owns the
    // window's composition tree, and the visual is what the swap chain's
    // content was set onto.
    _dcomp_device: IDCompositionDevice,
    _dcomp_target: IDCompositionTarget,
    _visual: IDCompositionVisual,
}

impl Presenter {
    /// `hwnd` must have been created with `WS_EX_NOREDIRECTIONBITMAP` --
    /// without it, Windows still allocates the ordinary GDI redirection
    /// surface behind the composition visual, which is the exact thing this
    /// module exists to bypass.
    pub fn new(hwnd: HWND, width: u32, height: u32) -> Result<Presenter, String> {
        let (dcomp_device, dcomp_target, visual) = unsafe {
            // `None`: DirectComposition is allowed to own its own rendering
            // device rather than share `wgpu`'s. The two devices never touch
            // the same resource -- the visual tree is DirectComposition's,
            // the swap chain's pixels are `wgpu`'s -- so there is nothing to
            // keep in sync between them.
            let dcomp_device: IDCompositionDevice = DCompositionCreateDevice3(None)
                .map_err(|e| format!("creating a DirectComposition device: {e}"))?;
            let dcomp_target = dcomp_device
                .CreateTargetForHwnd(hwnd, true)
                .map_err(|e| format!("binding DirectComposition to this window: {e}"))?;
            let visual = dcomp_device
                .CreateVisual()
                .map_err(|e| format!("creating a DirectComposition visual: {e}"))?;
            dcomp_target
                .SetRoot(&visual)
                .map_err(|e| format!("setting this window's composition root: {e}"))?;
            (dcomp_device, dcomp_target, visual)
        };

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());

        // SAFETY: `visual` is a valid `IDCompositionVisual`, kept alive for
        // exactly as long as the `Presenter` that owns both it and the
        // surface `wgpu` creates from it.
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CompositionVisual(
                Interface::as_raw(&visual),
            ))
        }
        .map_err(|e| format!("creating a wgpu surface from this window's visual: {e:?}"))?;

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
            _dcomp_device: dcomp_device,
            _dcomp_target: dcomp_target,
            _visual: visual,
        };
        presenter.configure(width, height);

        // PUBLISH THE VISUAL TREE. Nothing set above is visible on screen
        // until this call -- `SetRoot` and the swap chain `wgpu` bound to
        // the visual both stage changes that `Commit` is what applies.
        unsafe { presenter._dcomp_device.Commit() }
            .map_err(|e| format!("publishing this window's composition tree: {e}"))?;

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
