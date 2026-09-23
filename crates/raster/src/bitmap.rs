//! Uploaded images, and the one thing a painter has to do with them.
//!
//! WHY A STORE AND NOT A POINTER PER DRAW. Both backends want an image in a form
//! neither the display list nor a PNG has: vello wants an `Arc<Pixmap>` of
//! PREMULTIPLIED `PremulRgba8`, and tiny-skia wants its own `Pixmap`, also
//! premultiplied. Converting on every draw would premultiply a whole asset per
//! node per frame — a 256x256 icon is 65,536 pixels of arithmetic to draw a
//! picture that did not change — so the conversion happens ONCE, on upload, and
//! the ABI passes an id afterwards.
//!
//! That is the same shape `ar_font_load` and `ar_fill_text` already have, and for
//! the same reason. It is also why nothing here decodes a PNG: the host resolves
//! an asset to pixels (that is `Content` resolution, and it is the host's
//! business), and this takes the pixels. `image` stays where it was, writing
//! output.
//!
//! THE STORE IS PROCESS-WIDE, like the font store, and for the argument written
//! there: an asset's identity has nothing to do with which rasteriser draws it,
//! and the two surfaces a comparison run creates want the same one.

use std::collections::HashMap;
use std::sync::Arc;
use vello_cpu::peniko::color::PremulRgba8;
use vello_cpu::Pixmap as VPixmap;

///
/// BOTH, EAGERLY, AND THAT IS DELIBERATE. A surface picks its backend at
/// construction and a process can hold one of each; keeping only the form the
/// first caller happened to want would make the second one pay a conversion at
/// draw time, which is the cost this whole module exists to remove. An icon is
/// kilobytes twice over.
pub struct Stored {
    pub width: u16,
    pub height: u16,
    pub vello: Arc<VPixmap>,
    pub skia: tiny_skia::Pixmap,
}

#[derive(Default)]
pub struct BitmapStore {
    images: HashMap<u32, Arc<Stored>>,
    next: u32,
}

impl BitmapStore {
    /// Take straight (non-premultiplied) RGBA and return an id, or 0.
    ///
    /// STRAIGHT IN, PREMULTIPLIED OUT. `dew_runtime::frame::Bitmap` says why
    /// the display list carries the straight form: it is what a decoder produces
    /// and what a second decoder would produce, and premultiplying at the source
    /// would put a lossy step between resolving an asset and drawing it. This is
    /// the one place that step happens, which is what makes it one place to get
    /// right.
    pub fn upload(&mut self, rgba: &[u8], width: u32, height: u32) -> u32 {
        if width == 0 || height == 0 || width > u16::MAX as u32 || height > u16::MAX as u32 {
            return 0;
        }
        if rgba.len() != width as usize * height as usize * 4 {
            return 0;
        }

        let mut may_have_transparency = false;
        let mut vello = Vec::with_capacity(rgba.len() / 4);
        let mut skia = match tiny_skia::Pixmap::new(width, height) {
            Some(p) => p,
            None => return 0,
        };
        let skia_data = skia.pixels_mut();

        for (i, px) in rgba.chunks_exact(4).enumerate() {
            let a = px[3];
            may_have_transparency |= a != 255;
            // Rounded rather than truncated: `(c * a) / 255` alone drifts a
            // channel down by up to one step per pixel, which is invisible on one
            // image and is a visible tint difference between two backends that
            // rounded differently.
            let mul = |c: u8| (((c as u32 * a as u32) + 127) / 255) as u8;
            let (r, g, b) = (mul(px[0]), mul(px[1]), mul(px[2]));
            vello.push(PremulRgba8 { r, g, b, a });
            // `from_rgba` refuses a premultiplied triple that exceeds its alpha,
            // which the rounding above cannot produce; the fallback is a
            // transparent pixel rather than an unwrap that takes a desktop down
            // because one asset was odd.
            skia_data[i] = tiny_skia::PremultipliedColorU8::from_rgba(r, g, b, a)
                .unwrap_or_else(|| tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 0, 0).unwrap());
        }

        let (w16, h16) = (width as u16, height as u16);
        let pixmap = VPixmap::from_parts_with_opacity(vello, w16, h16, may_have_transparency);

        self.next += 1;
        let id = self.next;
        self.images.insert(
            id,
            Arc::new(Stored {
                width: w16,
                height: h16,
                vello: Arc::new(pixmap),
                skia,
            }),
        );
        id
    }

    pub fn get(&self, id: u32) -> Option<Arc<Stored>> {
        self.images.get(&id).cloned()
    }

    pub fn free(&mut self, id: u32) {
        self.images.remove(&id);
    }
}
