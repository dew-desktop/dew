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
use windows::Foundation::Numerics::Matrix3x2;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice3, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual,
};

/// Solid, opaque dark gray -- the same fallback Chrome shows through its own
/// compositor during a resize race, not a colour Dew's design owns.
const BACKDROP_BGRA: [u8; 4] = [32, 32, 32, 255];

/// Write the one solid pixel into `surface`'s current texture and present
/// it. Called every frame, not just once -- see `Presenter::backdrop_surface`'s
/// own doc for why a swap chain that stops presenting is the actual problem
/// this exists to avoid, not a detail cheap enough to skip.
fn present_solid(queue: &wgpu::Queue, surface: &wgpu::Surface<'_>) {
    let frame = match surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(frame)
        | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
        _ => return,
    };
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &frame.texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &BACKDROP_BGRA,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(std::iter::empty());
    frame.present();
}

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
    // window's composition tree, and each visual is what its own swap
    // chain's content was set onto.
    _dcomp_device: IDCompositionDevice,
    _dcomp_target: IDCompositionTarget,
    _root_visual: IDCompositionVisual,
    _content_visual: IDCompositionVisual,
    // BELOW `_content_visual` IN THE TREE, NEVER RESIZED. See its own setup
    // below for why a solid 1x1 surface stretched by a transform, rather
    // than something sized to the window, is what makes it immune to the
    // exact race it exists to paper over.
    _backdrop_visual: IDCompositionVisual,
    // RE-PRESENTED EVERY FRAME ALONGSIDE `surface`, NOT JUST ONCE AT
    // CONSTRUCTION. A composition swap chain that stops presenting while
    // ANOTHER on the same window keeps advancing at the display's refresh
    // rate is not a configuration wgpu or DirectComposition document
    // support for -- measured, not assumed, presenting it once and never
    // again is what preceded seconds-long stalls in this window's own
    // message pump, on messages that never touch either swap chain
    // directly (idle repaints, plain window moves).
    backdrop_surface: wgpu::Surface<'static>,
}

impl Presenter {
    /// `hwnd` must have been created with `WS_EX_NOREDIRECTIONBITMAP` --
    /// without it, Windows still allocates the ordinary GDI redirection
    /// surface behind the composition visual, which is the exact thing this
    /// module exists to bypass.
    pub fn new(hwnd: HWND, width: u32, height: u32) -> Result<Presenter, String> {
        let (dcomp_device, dcomp_target, root_visual, content_visual, backdrop_visual) = unsafe {
            // `None`: DirectComposition is allowed to own its own rendering
            // device rather than share `wgpu`'s. The two devices never touch
            // the same resource -- the visual tree is DirectComposition's,
            // the swap chains' pixels are `wgpu`'s -- so there is nothing to
            // keep in sync between them.
            let dcomp_device: IDCompositionDevice = DCompositionCreateDevice3(None)
                .map_err(|e| format!("creating a DirectComposition device: {e}"))?;
            let dcomp_target = dcomp_device
                .CreateTargetForHwnd(hwnd, true)
                .map_err(|e| format!("binding DirectComposition to this window: {e}"))?;

            // A ROOT WITH TWO CHILDREN, NOT ONE VISUAL DOING BOTH JOBS.
            // `content_visual` is resized every live-resize message and can
            // therefore lag the window's own on-screen bounds by one message
            // during a top or left edge drag, when the window's ORIGIN moves
            // and not only its far edge -- see this module's own resize
            // notes. `backdrop_visual` sits behind it and is never resized at
            // all, so it has nothing to lag: whatever gap `content_visual`
            // exposes for a moment shows this solid colour instead of the
            // desktop behind the window.
            let root_visual = dcomp_device
                .CreateVisual()
                .map_err(|e| format!("creating this window's root visual: {e}"))?;
            dcomp_target
                .SetRoot(&root_visual)
                .map_err(|e| format!("setting this window's composition root: {e}"))?;

            let backdrop_visual = dcomp_device
                .CreateVisual()
                .map_err(|e| format!("creating the backdrop visual: {e}"))?;
            root_visual
                .AddVisual(&backdrop_visual, false, None)
                .map_err(|e| format!("adding the backdrop visual: {e}"))?;

            let content_visual = dcomp_device
                .CreateVisual()
                .map_err(|e| format!("creating the content visual: {e}"))?;
            root_visual
                .AddVisual(&content_visual, true, &backdrop_visual)
                .map_err(|e| format!("adding the content visual above the backdrop: {e}"))?;

            (
                dcomp_device,
                dcomp_target,
                root_visual,
                content_visual,
                backdrop_visual,
            )
        };

        // VALIDATION OFF BY DEFAULT, EVEN IN A DEV BUILD. `wgpu`'s own
        // default (`InstanceFlags::from_build_config`) turns D3D12
        // validation on whenever `debug_assertions` is set, which is every
        // `cargo run`/`cargo build` in this workspace regardless of the
        // `[profile.dev.package."*"]` opt-level override -- and validation
        // is documented as adding real per-call overhead to exactly the
        // calls a live resize repeats every message (`configure`,
        // `write_texture`, `present`). `with_env()` still honours
        // `WGPU_VALIDATION=1` for whoever is actually chasing a `wgpu`-level
        // bug, which is what the flag is for; a live-resize feel test is not
        // that, and should not pay development tooling's cost by default.
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            flags: wgpu::InstanceFlags::empty().with_env(),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        // SAFETY: `content_visual` is a valid `IDCompositionVisual`, kept
        // alive for exactly as long as the `Presenter` that owns both it and
        // the surface `wgpu` creates from it.
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CompositionVisual(
                Interface::as_raw(&content_visual),
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

        // THE BACKDROP: A 1x1 SOLID SURFACE, STRETCHED, NEVER RESIZED AGAIN.
        // Sizing it to the window the way `content_visual` is sized would
        // just give it the same resize race to lose -- the point is that it
        // has no size to be wrong about. `SetTransform2` scales it far
        // beyond any real window instead; `DirectComposition` clips a
        // visual's content to the window's actual on-screen bounds as part
        // of the window's own compositing, which is exactly the step that
        // races `content_visual` and exactly the step this visual never
        // needs to keep up with.
        let backdrop_surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CompositionVisual(
                Interface::as_raw(&backdrop_visual),
            ))
        }
        .map_err(|e| format!("creating the backdrop's wgpu surface: {e:?}"))?;
        backdrop_surface.configure(
            &device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
                format,
                width: 1,
                height: 1,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            },
        );
        present_solid(&queue, &backdrop_surface);
        // FAR LARGER THAN ANY REAL WINDOW, DELIBERATELY. The visual's true
        // extent is whatever the window's own bounds clip it to -- this
        // number only has to be big enough never to be the limiting edge.
        const BACKDROP_SCALE: f32 = 1.0e5;
        unsafe {
            backdrop_visual
                .SetTransform2(&Matrix3x2 {
                    M11: BACKDROP_SCALE,
                    M12: 0.0,
                    M21: 0.0,
                    M22: BACKDROP_SCALE,
                    M31: 0.0,
                    M32: 0.0,
                })
                .map_err(|e| format!("scaling the backdrop visual: {e}"))?;
        }

        let mut presenter = Presenter {
            device,
            queue,
            surface,
            format,
            width: 0,
            height: 0,
            _dcomp_device: dcomp_device,
            _dcomp_target: dcomp_target,
            _root_visual: root_visual,
            _content_visual: content_visual,
            _backdrop_visual: backdrop_visual,
            backdrop_surface,
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

        present_solid(&self.queue, &self.backdrop_surface);
    }
}
