//! What a display must be able to do, and nothing about how.
//!
//! DELIBERATELY NOT COUPLED TO `aether_raster`. The display list has now survived
//! four rasterisers without the framework changing a line, and that property is
//! worth preserving one level up too: a snapshot painter writing a PNG, a GPU
//! painter owning a swapchain, and a test painter recording calls are all the
//! same shape, and none of them should require the others to be compiled.
//!
//! It is also what lets this crate be finished before the surface question is.
//! Whether the desktop shell ends up on a plain `wgpu` swapchain or on
//! DirectComposition for per-pixel window transparency changes how a painter is
//! CONSTRUCTED and nothing about this trait.

use crate::frame::{Delta, Frame, Gradient, Image, Node, Rect, Rgb};

/// A display that paints frames.
///
/// Implementors get `paint_frame` for free and should override `paint_delta` only
/// when they can genuinely repaint a region — a retained display can, an
/// immediate-mode one over a rotating swapchain cannot, because "the surface still
/// holds the previous frame" is false there and the dirty rect has nothing to
/// repair.
pub trait Painter {
    /// Begin a frame.
    ///
    /// `None` means clear to NOTHING — a surface composited from its own alpha,
    /// where a pixel the tree did not paint is a pixel the window does not
    /// occupy. It does not mean "no colour supplied, pick one": a painter that
    /// substitutes black there turns every floating surface into a black
    /// rectangle.
    fn begin(&mut self, width: f32, height: f32, background: Option<Rgb>);

    fn fill_rounded_rect(&mut self, rect: Rect, radius: f32, colour: Rgb, alpha: f32);

    fn stroke_rounded_rect(
        &mut self,
        rect: Rect,
        radius: f32,
        thickness: f32,
        colour: Rgb,
        alpha: f32,
    );

    /// Draw text. `align_x`/`align_y` are already resolved against the engine's
    /// defaults by the time they arrive — a painter that re-decides alignment is
    /// the bug this signature exists to prevent.
    fn draw_text(&mut self, node: &Node);

    /// Fill with a gradient.
    ///
    /// The default FALLS BACK TO THE FLAT FILL rather than drawing nothing: a
    /// backend that cannot ramp should render the node's own colour, which is
    /// what it looked like before gradients existed. Drawing nothing would make
    /// a gradient node disappear, which is worse than an unramped one and much
    /// harder to spot.
    fn fill_gradient(
        &mut self,
        rect: Rect,
        radius: f32,
        gradient: &Gradient,
        fallback: Option<Rgb>,
        alpha: f32,
    ) {
        let _ = gradient;
        if let Some(colour) = fallback {
            self.fill_rounded_rect(rect, radius, colour, alpha);
        }
    }

    /// Draw a node's image.
    ///
    /// THE DEFAULT DEGRADES RATHER THAN DISAPPEARS, for the reason
    /// [`Painter::fill_gradient`] gives one signature above: a backend that
    /// cannot do the thing should draw what the thing occupies, because a node
    /// that vanishes looks like a layout bug and is hunted for in the wrong file.
    /// So a painter with no image support fills the destination rectangle with
    /// the node's tint, or with a neutral grey when there is none — a flat
    /// silhouette exactly where the picture belongs.
    ///
    /// `src` and `dst` are already resolved by [`Image::placement`], so
    /// `ScaleType`, `ImageRectOffset` and `ImageRectSize` are decided before this
    /// is called and an implementation never re-decides them. An implementor
    /// overrides this and maps `src` onto `dst`; nothing else.
    fn draw_image(&mut self, image: &Image, src: Rect, dst: Rect) {
        let _ = src;
        self.fill_rounded_rect(
            dst,
            0.0,
            image.tint.unwrap_or(Rgb(128, 128, 128)),
            image.alpha,
        );
    }

    /// Draw a node whose image the host could not resolve.
    ///
    /// NOT NOTHING, AND THE SAME ARGUMENT AS ABOVE WITH HIGHER STAKES. ADR-003
    /// makes an unresolvable `Content` "a rendering outcome, not a property
    /// error": the assignment succeeded, the guest's value is intact, and the
    /// host owes an answer about why no picture appeared. Drawing nothing gives
    /// that answer as a blank space, which is indistinguishable from an element
    /// that was never created, positioned off-screen, or made invisible — three
    /// places an author will look before suspecting the asset.
    ///
    /// So: a hollow box the size of the node, at a quarter of its own alpha. It
    /// reads as "something belongs here and did not arrive" at a glance, it
    /// cannot be mistaken for a drawn image, and it needs nothing of a painter
    /// beyond the two primitives every one of them already has. Sprint 5, which
    /// owns resolution, is what makes it say WHICH asset by name in the log; this
    /// is what makes the pixel say that anything is missing at all.
    fn draw_missing_image(&mut self, image: &Image, rect: Rect, radius: f32) {
        let colour = image.tint.unwrap_or(Rgb(148, 158, 176));
        let alpha = image.alpha * 0.25;
        self.fill_rounded_rect(rect, radius, colour, alpha * 0.4);
        self.stroke_rounded_rect(rect, radius, 1.0, colour, alpha);
    }

    fn clip_push(&mut self, rect: Rect);

    /// A clip with rounded corners.
    ///
    /// DEFAULTED TO THE SQUARE PUSH so an implementor that has no rounded
    /// clipping keeps compiling and keeps its old behaviour. A painter that can
    /// mask corners overrides it; one that cannot leaks them exactly as it did
    /// before, which is the honest fallback.
    fn clip_push_rounded(&mut self, rect: Rect, _radius: f32) {
        self.clip_push(rect);
    }

    fn clip_pop(&mut self);

    /// Present. Returns whether the surface accepted it.
    fn end(&mut self) -> bool;

    /// Paint a whole frame in paint order.
    ///
    /// The traversal lives here rather than in each painter because it encodes
    /// the ORDER things happen in — clip, fill, gradient, image, stroke, text — and
    /// three painters independently discovering that order is three chances to get
    /// it wrong in a way that only shows up on one backend.
    fn paint_frame(&mut self, frame: &Frame, background: Option<Rgb>) -> bool {
        self.begin(frame.width, frame.height, background);
        for node in &frame.nodes {
            paint_node(self, node);
        }
        self.end()
    }

    /// Paint only what changed. The default repaints everything, which is always
    /// correct and never wrong — just wasteful. Override it when the surface can
    /// actually hold a previous frame.
    fn paint_delta(&mut self, delta: &Delta, background: Option<Rgb>) -> bool {
        self.paint_frame(&delta.frame, background)
    }
}

/// Paint one node, in the order its parts must be drawn.
///
/// `pub(crate)` so an implementor overriding `paint_delta` reuses this rather
/// than writing the order out again — clip, fill or gradient, image, stroke,
/// text. Three painters independently rediscovering that order is three chances
/// to get it wrong on one backend only.
pub(crate) fn paint_node<P: Painter + ?Sized>(painter: &mut P, node: &Node) {
    let clipped = node.clip.is_some();
    if let Some(clip) = node.clip {
        painter.clip_push_rounded(clip, node.clip_radius);
    }

    // A NODE WITH NO FILL IS NOT A BLACK NODE. Live.luau emits nil when nothing
    // set a colour, and the engine draws nothing for it; inventing a default here
    // would paint rectangles the engine leaves empty.
    //
    // A GRADIENT REPLACES THE FLAT FILL, and is checked first for that reason. It
    // can also be an ALPHA ramp over the node's own colour with no colour ramp of
    // its own, which is why the fill travels with it rather than being skipped.
    if node.alpha > 0.0 {
        match &node.gradient {
            Some(gradient) => {
                painter.fill_gradient(node.rect, node.radius, gradient, node.fill, node.alpha)
            }
            None => {
                if let Some(fill) = node.fill {
                    painter.fill_rounded_rect(node.rect, node.radius, fill, node.alpha);
                }
            }
        }
    }

    // AFTER THE FILL AND BEFORE THE STROKE, which is where the engine puts it: an
    // `ImageLabel` draws its background, then its image over it, and a `UIStroke`
    // outlines the whole element on top of both. Painting the image first would
    // hide it behind any background the element also has, and painting it after
    // the stroke would let a picture spill over its own border.
    //
    // AN IMAGE IS NOT GATED ON `node.alpha`, unlike the fill above.
    // `BackgroundTransparency = 1` is the ordinary way to write an image with no
    // plate behind it, and it is what every icon in every the engine UI does; folding
    // the image into that check would make the common case draw nothing.
    //
    // THE MISSING BOX IS FOR A MISSING ASSET AND NOTHING ELSE. `placement` also
    // answers `None` for a degenerate rectangle — a zero `ImageRectSize`, say —
    // and that is a guest asking for no pixels rather than a host failing to
    // find any. Keying the placeholder on the bitmap's absence keeps the two
    // apart; treating them alike would put a "missing" marker on screen for an
    // asset that resolved perfectly well.
    if let Some(image) = &node.image {
        if image.bitmap.is_none() {
            painter.draw_missing_image(image, node.rect, node.radius);
        } else if let Some((src, dst)) = image.placement(node.rect) {
            painter.draw_image(image, src, dst);
        }
    }

    if let Some(stroke) = &node.stroke {
        if let Some(colour) = stroke.colour {
            painter.stroke_rounded_rect(
                node.rect,
                node.radius,
                stroke.thickness,
                colour,
                stroke.alpha,
            );
        }
    }

    if node.text.is_some() {
        painter.draw_text(node);
    }

    if clipped {
        painter.clip_pop();
    }
}
