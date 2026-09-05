//! Turning what a guest named into pixels a painter can draw.
//!
//! WHY THIS IS NOT UNDER `datamodel/`, WHICH IS WHERE IT WAS FIRST WRITTEN
//! `scripts/verify_boundaries.luau` section 3 refused it there, and correctly:
//! only the render bridge may touch `dew_runtime`'s display list, because a
//! DataModel module deferring to the render IR is how the standard ends up with
//! two implementations inside one host. Resolving a `Content` PRODUCES render IR
//! -- a `frame::Bitmap` -- so it is part of the bridge and not part of the
//! DataModel, whatever the property that triggered it is called. The file moved
//! rather than the rule.
//!
//! WHAT THIS IS, AND WHAT SPRINT 5 MAKES OF IT
//! One scheme, hard-coded, with no registry, no permission, no fetch and no
//! content-addressed cache. Those four are Sprint 5 and they are deliberately
//! absent: ADR-003's registry is the deliverable there, and building an empty one
//! here to hold a single entry would be the shape this project has spent a
//! milestone removing -- a mechanism described in the present tense with one
//! caller and no second case to justify its joints.
//!
//! What this sprint owed was a ROUTE. `Image` and `ImageContent` were accepted by
//! the DataModel and there was no path from either to a pixel on any path, with
//! or without fetching solved. This is the shortest route that is not a
//! throwaway: `mod://` resolves against the directory the mod was loaded from,
//! which is exactly Sprint 5's task 4 -- "a local scheme, ungated, mod-relative"
//! -- so that sprint keeps this rule and grows a registry around it rather than
//! unpicking a stub.
//!
//! WHY `mod://` AND NOT `rbxasset://`
//! `rbxasset://` is the engine's own local scheme and it resolves against the
//! CLIENT's content folder, not against anything a place ships. A mod using it
//! would not be portable in the direction that matters, and squatting the name
//! for a different meaning is the kind of near-parity that costs more than it
//! buys. `dew://` is left free for a registry that Sprint 5's cache may earn.
//! `mod://` says what it resolves against, which is the whole of what it does.
//!
//! WHY ANYTHING ELSE IS AN OUTCOME AND NOT AN ERROR
//! ADR-003: "an unresolvable `Content` is a rendering outcome, not a property
//! error -- the assignment succeeds and the host reports why nothing was drawn."
//! `rbxassetid://7` assigns, reaches here, resolves to nothing, is REPORTED BY
//! NAME once, and is drawn as a missing-image box. A Roblox application moved to
//! Dew before Sprint 5 is a correct application missing an image.

use dew_runtime::frame::Bitmap;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The scheme this host resolves without asking anyone's permission.
pub const LOCAL_SCHEME: &str = "mod://";

/// What one mod can turn into pixels.
#[derive(Default)]
pub struct Assets {
    /// The directory `mod://` is relative to. `None` for a DOM that was never
    /// given one -- a unit test, or a guest with no files of its own -- and every
    /// lookup then reports itself rather than guessing at a working directory.
    root: Option<PathBuf>,
    /// Decoded pixels by URI, INCLUDING THE FAILURES.
    ///
    /// A miss is cached as `None` on purpose. The renderer runs per frame and a
    /// mod that names an asset that is not there would otherwise stat a missing
    /// file forever -- and, worse, print about it forever. Caching the answer
    /// makes "we already said this" a property of the data rather than of a
    /// separate flag someone has to remember to set.
    cache: HashMap<String, Option<Arc<Bitmap>>>,
    /// Things already said out loud, so a per-frame path can speak once.
    said: HashSet<String>,
}

impl Assets {
    /// Where `mod://` points. Set by whoever loaded the guest.
    pub fn set_root(&mut self, dir: PathBuf) {
        self.root = Some(dir);
    }

    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// Say something once per lifetime of this DOM.
    ///
    /// THE RENDERER RUNS PER FRAME, so anything it wants to tell an author has to
    /// pass through here or become a scrolling wall that trains everyone to
    /// ignore the console. `key` is what makes two reports the same report --
    /// usually the URI, or the property that was not honoured.
    pub fn note_once(&mut self, key: String, message: &str) {
        if self.said.insert(key) {
            eprintln!("[dew] {message}");
        }
    }

    /// Resolve what a guest named into pixels, or `None` with a reason said once.
    pub fn resolve(&mut self, uri: &str) -> Option<Arc<Bitmap>> {
        if let Some(hit) = self.cache.get(uri) {
            return hit.clone();
        }
        let resolved = self.load(uri);
        self.cache.insert(uri.to_string(), resolved.clone());
        resolved
    }

    fn load(&mut self, uri: &str) -> Option<Arc<Bitmap>> {
        let Some(relative) = uri.strip_prefix(LOCAL_SCHEME) else {
            // NAMED, NOT DISMISSED. Which scheme it was is the useful half: an
            // author who wrote `rbxassetid://` should be told that this host does
            // not resolve it yet, not that "an image failed".
            let scheme = uri.split_once("://").map(|(s, _)| s).unwrap_or(uri);
            self.note_once(
                uri.to_string(),
                &format!(
                    "{uri}: this host resolves only `{LOCAL_SCHEME}` so far, and `{scheme}` is \
                     not it -- the property keeps its value and the node draws as missing"
                ),
            );
            return None;
        };

        let Some(root) = self.root.clone() else {
            self.note_once(
                uri.to_string(),
                &format!("{uri}: `{LOCAL_SCHEME}` needs a mod directory and this guest has none"),
            );
            return None;
        };

        // A URI IS NOT A PATH UNTIL IT HAS BEEN CHECKED. `mod://../../secrets`
        // is a well-formed URI, and a mod's own files are the only files it is
        // allowed to read -- the same rule `Capabilities::require_roots` enforces
        // for `require`, which images would otherwise be the way around. Rejected
        // by COMPONENT rather than by string matching, so neither a Windows
        // separator nor a doubled-up form slips past a search for "..".
        let candidate = Path::new(relative);
        let safe = !relative.is_empty()
            && !candidate.is_absolute()
            && candidate.components().all(|c| {
                matches!(
                    c,
                    std::path::Component::Normal(_) | std::path::Component::CurDir
                )
            });
        if !safe {
            self.note_once(
                uri.to_string(),
                &format!("{uri}: a `{LOCAL_SCHEME}` path must stay inside the mod's own directory"),
            );
            return None;
        }

        let path = root.join(candidate);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) => {
                self.note_once(
                    uri.to_string(),
                    &format!("{uri}: {} -- {e}", path.display()),
                );
                return None;
            }
        };

        // STRAIGHT RGBA, because that is what the display list carries and what
        // `dew_raster` premultiplies once on upload. Converting here would put
        // the lossy step in front of the seam rather than behind it.
        let decoded = match image::load_from_memory(&bytes) {
            Ok(decoded) => decoded.to_rgba8(),
            Err(e) => {
                self.note_once(
                    uri.to_string(),
                    &format!("{uri}: not a readable image -- {e}"),
                );
                return None;
            }
        };
        let (width, height) = decoded.dimensions();
        match Bitmap::new(width, height, decoded.into_raw()) {
            Some(bitmap) => Some(Arc::new(bitmap)),
            None => {
                self.note_once(
                    uri.to_string(),
                    &format!("{uri}: decoded to {width}x{height}, which is not drawable"),
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-pixel PNG, encoded here rather than committed, so these tests need
    /// no fixture on disk and still go through the real decoder.
    fn png(rgba: [u8; 4]) -> Vec<u8> {
        let mut buffer = std::io::Cursor::new(Vec::new());
        let img = image::RgbaImage::from_raw(1, 1, rgba.to_vec()).expect("1x1");
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut buffer, image::ImageFormat::Png)
            .expect("encode");
        buffer.into_inner()
    }

    struct Fixture(PathBuf);

    impl Fixture {
        fn new(name: &str) -> Fixture {
            let dir = std::env::temp_dir().join(format!("dew-assets-test-{name}"));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("temp dir");
            Fixture(dir)
        }

        fn assets(&self) -> Assets {
            let mut assets = Assets::default();
            assets.set_root(self.0.clone());
            assets
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_local_uri_resolves_to_pixels_beside_the_mod() {
        let fixture = Fixture::new("local");
        std::fs::write(fixture.0.join("dot.png"), png([10, 20, 30, 255])).expect("write");
        let mut assets = fixture.assets();
        let bitmap = assets.resolve("mod://dot.png").expect("resolved");
        assert_eq!((bitmap.width, bitmap.height), (1, 1));
        assert_eq!(bitmap.rgba, vec![10, 20, 30, 255]);
    }

    #[test]
    fn a_nested_path_resolves_and_a_traversal_does_not() {
        let fixture = Fixture::new("nested");
        std::fs::create_dir_all(fixture.0.join("icons")).expect("subdir");
        std::fs::write(fixture.0.join("icons/dot.png"), png([1, 2, 3, 255])).expect("write");
        // The file the traversal would reach really exists, so this fails for the
        // rule rather than for the absence.
        let outside = fixture.0.join("..").join("dew-assets-outside.png");
        std::fs::write(&outside, png([9, 9, 9, 255])).expect("write");

        let mut assets = fixture.assets();
        assert!(assets.resolve("mod://icons/dot.png").is_some());
        assert!(assets.resolve("mod://../dew-assets-outside.png").is_none());
        let _ = std::fs::remove_file(&outside);
    }

    #[test]
    fn an_unknown_scheme_resolves_to_nothing_rather_than_erroring() {
        // ADR-003's rule, as a test: the value was valid, the host could not
        // resolve it, and that is a rendering outcome.
        let fixture = Fixture::new("scheme");
        let mut assets = fixture.assets();
        assert!(assets.resolve("rbxassetid://12345").is_none());
    }

    #[test]
    fn a_missing_file_is_answered_once_and_remembered() {
        let fixture = Fixture::new("missing");
        let mut assets = fixture.assets();
        assert!(assets.resolve("mod://absent.png").is_none());
        // The second call must not re-read the disk, which is what the cached
        // `None` is for; observable here as the answer surviving the file
        // appearing afterwards.
        std::fs::write(fixture.0.join("absent.png"), png([5, 5, 5, 255])).expect("write");
        assert!(
            assets.resolve("mod://absent.png").is_none(),
            "a resolved answer is cached, including a failure"
        );
    }

    #[test]
    fn a_file_that_is_not_an_image_is_refused_rather_than_drawn() {
        let fixture = Fixture::new("garbage");
        std::fs::write(fixture.0.join("not.png"), b"this is not a png").expect("write");
        let mut assets = fixture.assets();
        assert!(assets.resolve("mod://not.png").is_none());
    }

    #[test]
    fn a_dom_with_no_root_reports_rather_than_guessing_a_directory() {
        let mut assets = Assets::default();
        assert!(assets.root().is_none());
        assert!(assets.resolve("mod://anything.png").is_none());
    }
}
