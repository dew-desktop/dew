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
//! THE SCHEME REGISTRY AND CONTENT-ADDRESSED CACHE (Milestone 2, Sprint 5)
//! `mod://` is the primary, local, ungated scheme resolving mod-relative assets.
//! `rbxassetid://` is optional developer tooling for local testing and
//! conformance parity, off by default, and gated behind the `rbxassetid`
//! permission.
//!
//! The cache is content-addressed by SHA-256 hash, not keyed by asset id.
//! Future schemes (`dewassetid://`, plain URLs) attach as registry entries
//! rather than reopening resolution architecture.

use crate::manifest::Permission;
use dew_runtime::frame::Bitmap;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The primary local scheme this host resolves without asking anyone's permission.
pub const LOCAL_SCHEME: &str = "mod://";

/// The remote the engine asset scheme, gated behind `rbxassetid` permission.
pub const RBX_SCHEME: &str = "rbxassetid://";

/// Cryptographic hash (SHA-256 in hex) for content addressing.
pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// A content-addressed cache keyed by cryptographic hash.
///
/// Holds raw bytes indexed by content hash (SHA-256). An auxiliary index maps
/// asset URIs or identifiers to the content hash. Cache hits and misses are
/// tracked and countable.
#[derive(Default, Debug, Clone)]
pub struct ContentCache {
    /// In-memory content-addressed store: hash -> raw bytes.
    objects: HashMap<String, Vec<u8>>,
    /// Mapping of asset URI/identifier -> content hash.
    index: HashMap<String, String>,
    /// Optional directory on disk for persistent content storage.
    dir: Option<PathBuf>,
    /// Countable metrics for assertions and observability.
    hits: usize,
    misses: usize,
}

impl ContentCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_dir(dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(dir.join("objects"));
        Self {
            dir: Some(dir),
            ..Default::default()
        }
    }

    pub fn hits(&self) -> usize {
        self.hits
    }

    pub fn misses(&self) -> usize {
        self.misses
    }

    pub fn stats(&self) -> (usize, usize) {
        (self.hits, self.misses)
    }

    pub fn reset_stats(&mut self) {
        self.hits = 0;
        self.misses = 0;
    }

    pub fn contains_hash(&self, hash: &str) -> bool {
        if self.objects.contains_key(hash) {
            return true;
        }
        if let Some(dir) = &self.dir {
            return dir.join("objects").join(hash).is_file();
        }
        false
    }

    pub fn hash_for_uri(&self, uri: &str) -> Option<&str> {
        self.index.get(uri).map(|s| s.as_str())
    }

    /// Retrieve content bytes by URI, recording a hit or miss.
    pub fn get_by_uri(&mut self, uri: &str) -> Option<Vec<u8>> {
        if let Some(hash) = self.index.get(uri).cloned() {
            self.get_by_hash(&hash)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Retrieve content bytes by content hash, recording a hit or miss.
    pub fn get_by_hash(&mut self, hash: &str) -> Option<Vec<u8>> {
        if let Some(bytes) = self.objects.get(hash) {
            self.hits += 1;
            return Some(bytes.clone());
        }
        if let Some(dir) = &self.dir {
            let path = dir.join("objects").join(hash);
            if let Ok(bytes) = std::fs::read(&path) {
                self.objects.insert(hash.to_string(), bytes.clone());
                self.hits += 1;
                return Some(bytes);
            }
        }
        self.misses += 1;
        None
    }

    /// Store content bytes for a URI, returning its content hash.
    pub fn store(&mut self, uri: &str, bytes: Vec<u8>) -> String {
        let hash = hash_bytes(&bytes);
        if let Some(dir) = &self.dir {
            let path = dir.join("objects").join(&hash);
            if !path.is_file() {
                let _ = std::fs::write(&path, &bytes);
            }
        }
        self.objects.insert(hash.clone(), bytes);
        self.index.insert(uri.to_string(), hash.clone());
        hash
    }
}

/// Transport abstraction for fetching remote content.
pub trait Transport: Send + Sync {
    fn fetch(&self, url: &str) -> Result<Vec<u8>, String>;
}

/// Production HTTP transport using `ureq` with a configurable timeout.
pub struct HttpTransport {
    timeout: Duration,
}

impl Default for HttpTransport {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
        }
    }
}

impl Transport for HttpTransport {
    fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
        let response = ureq::AgentBuilder::new()
            .timeout(self.timeout)
            .build()
            .get(url)
            .call()
            .map_err(|e| format!("offline or network error: {e}"))?;

        let mut bytes = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut bytes)
            .map_err(|e| format!("failed to read response bytes: {e}"))?;

        Ok(bytes)
    }
}

/// A test mock transport providing canned responses by URL without network calls.
pub struct MockTransport<F: Fn(&str) -> Result<Vec<u8>, String> + Send + Sync>(pub F);

impl<F: Fn(&str) -> Result<Vec<u8>, String> + Send + Sync> Transport for MockTransport<F> {
    fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
        (self.0)(url)
    }
}

/// Context handed to a scheme handler when resolving a URI.
pub struct ResolveContext<'a> {
    pub root: Option<&'a Path>,
    pub transport: &'a dyn Transport,
    pub content_cache: &'a mut ContentCache,
}

/// Handler for a registered URI scheme.
pub trait SchemeHandler: Send + Sync {
    fn resolve(
        &self,
        ctx: &mut ResolveContext<'_>,
        rest: &str,
        full_uri: &str,
    ) -> Result<Vec<u8>, String>;
}

/// Handler for `mod://` paths, relative to the mod's own directory.
pub struct ModSchemeHandler;

impl SchemeHandler for ModSchemeHandler {
    fn resolve(
        &self,
        ctx: &mut ResolveContext<'_>,
        relative: &str,
        full_uri: &str,
    ) -> Result<Vec<u8>, String> {
        let Some(root) = ctx.root else {
            return Err(format!(
                "{full_uri}: `{LOCAL_SCHEME}` needs a mod directory and this guest has none"
            ));
        };

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
            return Err(format!(
                "{full_uri}: a `{LOCAL_SCHEME}` path must stay inside the mod's own directory"
            ));
        }

        let path = root.join(candidate);
        std::fs::read(&path).map_err(|e| format!("{full_uri}: {} -- {e}", path.display()))
    }
}

/// Handler for `rbxassetid://` asset identifiers.
pub struct RbxAssetSchemeHandler {
    base_url: String,
}

impl Default for RbxAssetSchemeHandler {
    fn default() -> Self {
        Self {
            base_url: "https://assetdelivery.roblox.com/v1/asset/?id=".to_string(),
        }
    }
}

impl SchemeHandler for RbxAssetSchemeHandler {
    fn resolve(
        &self,
        ctx: &mut ResolveContext<'_>,
        rest: &str,
        full_uri: &str,
    ) -> Result<Vec<u8>, String> {
        let id_str = rest.trim();
        if id_str.is_empty() || !id_str.chars().all(|c| c.is_ascii_digit()) {
            return Err(format!(
                "{full_uri}: `{rest}` is not a valid numeric asset id"
            ));
        }

        let url = format!("{}{id_str}", self.base_url);
        ctx.transport.fetch(&url)
    }
}

/// A registered scheme entry.
pub struct SchemeEntry {
    pub prefix: &'static str,
    pub permission: Option<Permission>,
    pub handler: Arc<dyn SchemeHandler>,
}

/// The scheme registry.
pub struct SchemeRegistry {
    entries: Vec<SchemeEntry>,
}

impl Default for SchemeRegistry {
    fn default() -> Self {
        let mut registry = Self {
            entries: Vec::new(),
        };
        // Register mod:// (primary local, ungated)
        registry.register(LOCAL_SCHEME, None, Arc::new(ModSchemeHandler));
        // Register rbxassetid:// (developer tooling, requires Permission::RbxAssetId)
        registry.register(
            RBX_SCHEME,
            Some(Permission::RbxAssetId),
            Arc::new(RbxAssetSchemeHandler::default()),
        );
        registry
    }
}

impl SchemeRegistry {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn register(
        &mut self,
        prefix: &'static str,
        permission: Option<Permission>,
        handler: Arc<dyn SchemeHandler>,
    ) {
        self.entries.push(SchemeEntry {
            prefix,
            permission,
            handler,
        });
    }

    pub fn find<'a>(&self, uri: &'a str) -> Option<(&SchemeEntry, &'a str)> {
        for entry in &self.entries {
            if let Some(rest) = uri.strip_prefix(entry.prefix) {
                return Some((entry, rest));
            }
        }
        None
    }
}

/// Helper function to decode raw image bytes into straight RGBA `Bitmap`.
fn decode_bitmap(bytes: &[u8], uri: &str, said: &mut HashSet<String>) -> Option<Arc<Bitmap>> {
    let decoded = match image::load_from_memory(bytes) {
        Ok(decoded) => decoded.to_rgba8(),
        Err(e) => {
            if said.insert(uri.to_string()) {
                eprintln!("[dew] {uri}: not a readable image -- {e}");
            }
            return None;
        }
    };
    let (width, height) = decoded.dimensions();
    match Bitmap::new(width, height, decoded.into_raw()) {
        Some(bitmap) => Some(Arc::new(bitmap)),
        None => {
            if said.insert(uri.to_string()) {
                eprintln!("[dew] {uri}: decoded to {width}x{height}, which is not drawable");
            }
            None
        }
    }
}

/// What one mod or runtime session can turn into pixels.
pub struct Assets {
    /// The directory `mod://` is relative to.
    root: Option<PathBuf>,
    /// Permissions granted to the active guest.
    permissions: HashSet<Permission>,
    /// Extensible scheme registry.
    registry: SchemeRegistry,
    /// Shared content-addressed cache (keyed by hash).
    content_cache: Arc<Mutex<ContentCache>>,
    /// Network transport.
    transport: Arc<dyn Transport>,
    /// Decoded pixels memo by URI, INCLUDING THE FAILURES.
    cache: HashMap<String, Option<Arc<Bitmap>>>,
    /// Set of URIs currently being fetched in the background.
    in_flight: HashSet<String>,
    /// Things already said out loud to avoid scrolling console noise.
    said: HashSet<String>,
    /// Optional notification hook when a background fetch completes.
    dirty_hook: Option<Arc<dyn Fn() + Send + Sync>>,
    /// Whether resolution should block synchronously (default true for headless/tests).
    blocking: bool,
}

impl Default for Assets {
    fn default() -> Self {
        Self {
            root: None,
            permissions: HashSet::new(),
            registry: SchemeRegistry::default(),
            content_cache: Arc::new(Mutex::new(ContentCache::default())),
            transport: Arc::new(HttpTransport::default()),
            cache: HashMap::new(),
            in_flight: HashSet::new(),
            said: HashSet::new(),
            dirty_hook: None,
            blocking: true,
        }
    }
}

impl Assets {
    /// Where `mod://` points. Set by whoever loaded the guest.
    pub fn set_root(&mut self, dir: PathBuf) {
        self.root = Some(dir);
    }

    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// Set granted permissions for this asset resolver.
    pub fn set_permissions(&mut self, perms: impl IntoIterator<Item = Permission>) {
        self.permissions = perms.into_iter().collect();
    }

    pub fn has_permission(&self, perm: Permission) -> bool {
        self.permissions.contains(&perm)
    }

    pub fn registry_mut(&mut self) -> &mut SchemeRegistry {
        &mut self.registry
    }

    pub fn content_cache(&self) -> Arc<Mutex<ContentCache>> {
        self.content_cache.clone()
    }

    pub fn set_content_cache(&mut self, cache: Arc<Mutex<ContentCache>>) {
        self.content_cache = cache;
    }

    pub fn set_transport(&mut self, transport: Arc<dyn Transport>) {
        self.transport = transport;
    }

    pub fn set_blocking(&mut self, blocking: bool) {
        self.blocking = blocking;
    }

    pub fn set_dirty_hook(&mut self, hook: Arc<dyn Fn() + Send + Sync>) {
        self.dirty_hook = Some(hook);
    }

    /// Say something once per lifetime of this DOM.
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
        let (entry_prefix, required_permission, handler) = match self.registry.find(uri) {
            Some((entry, _)) => (entry.prefix, entry.permission, entry.handler.clone()),
            None => {
                let scheme = uri.split_once("://").map(|(s, _)| s).unwrap_or(uri);
                self.note_once(
                    uri.to_string(),
                    &format!(
                        "{uri}: this host has no resolver registered for scheme `{scheme}` -- \
                         the property keeps its value and the node draws as missing"
                    ),
                );
                return None;
            }
        };

        let rest = uri.strip_prefix(entry_prefix).unwrap_or("");

        // Gating: check permission requirement.
        if let Some(required) = required_permission {
            if !self.permissions.contains(&required) {
                self.note_once(
                    uri.to_string(),
                    &format!(
                        "{uri}: requires permission `{}` which was not granted in dew.toml -- \
                         the property keeps its value and the node draws as missing",
                        required.name()
                    ),
                );
                return None;
            }
        }

        // 1. Content cache lookup (by URI -> content hash).
        if let Some(bytes) = self.content_cache.lock().unwrap().get_by_uri(uri) {
            return decode_bitmap(&bytes, uri, &mut self.said);
        }

        // 2. Fetch / resolve content bytes.
        if self.blocking || entry_prefix == LOCAL_SCHEME {
            let res = {
                let cache_arc = self.content_cache.clone();
                let mut cache_guard = cache_arc.lock().unwrap();
                let mut ctx = ResolveContext {
                    root: self.root.as_deref(),
                    transport: self.transport.as_ref(),
                    content_cache: &mut cache_guard,
                };
                handler.resolve(&mut ctx, rest, uri)
            };
            match res {
                Ok(bytes) => {
                    let _hash = self.content_cache.lock().unwrap().store(uri, bytes.clone());
                    decode_bitmap(&bytes, uri, &mut self.said)
                }
                Err(e) => {
                    self.note_once(uri.to_string(), &e);
                    None
                }
            }
        } else {
            // Asynchronous fetch in background thread.
            if self.in_flight.contains(uri) {
                return None;
            }

            self.in_flight.insert(uri.to_string());
            let uri_clone = uri.to_string();
            let rest_clone = rest.to_string();
            let transport = self.transport.clone();
            let content_cache = self.content_cache.clone();
            let hook = self.dirty_hook.clone();

            std::thread::spawn(move || {
                let res = {
                    let mut cache_guard = content_cache.lock().unwrap();
                    let mut ctx = ResolveContext {
                        root: None,
                        transport: transport.as_ref(),
                        content_cache: &mut cache_guard,
                    };
                    handler.resolve(&mut ctx, &rest_clone, &uri_clone)
                };

                if let Ok(bytes) = res {
                    content_cache.lock().unwrap().store(&uri_clone, bytes);
                }

                if let Some(h) = hook {
                    h();
                }
            });

            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-pixel PNG, encoded in memory so tests need no fixture on disk.
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

        let outside = fixture.0.join("..").join("dew-assets-outside.png");
        std::fs::write(&outside, png([9, 9, 9, 255])).expect("write");

        let mut assets = fixture.assets();
        assert!(assets.resolve("mod://icons/dot.png").is_some());
        assert!(assets.resolve("mod://../dew-assets-outside.png").is_none());
        let _ = std::fs::remove_file(&outside);
    }

    #[test]
    fn an_unknown_scheme_resolves_to_nothing_rather_than_erroring() {
        let fixture = Fixture::new("unknown-scheme");
        let mut assets = fixture.assets();
        assert!(assets.resolve("unknown://12345").is_none());
    }

    #[test]
    fn rbxassetid_without_permission_is_refused() {
        let fixture = Fixture::new("refused");
        let mut assets = fixture.assets();
        // Permission::RbxAssetId is NOT granted
        assert!(!assets.has_permission(Permission::RbxAssetId));
        assert!(assets.resolve("rbxassetid://12345").is_none());
    }

    #[test]
    fn rbxassetid_with_permission_resolves_and_caches_by_hash() {
        let fixture = Fixture::new("resolved-rbx");
        let mut assets = fixture.assets();
        assets.set_permissions([Permission::RbxAssetId]);

        let test_png = png([100, 150, 200, 255]);
        let expected_hash = hash_bytes(&test_png);

        let png_bytes = test_png.clone();
        assets.set_transport(Arc::new(MockTransport(move |_url| Ok(png_bytes.clone()))));

        // First resolve: cache miss, fetches and stores in content-addressed cache
        let bitmap = assets.resolve("rbxassetid://12345").expect("bitmap");
        assert_eq!((bitmap.width, bitmap.height), (1, 1));
        assert_eq!(bitmap.rgba, vec![100, 150, 200, 255]);

        let cache = assets.content_cache();
        assert_eq!(cache.lock().unwrap().misses(), 1);
        assert_eq!(cache.lock().unwrap().hits(), 0);
        assert!(cache.lock().unwrap().contains_hash(&expected_hash));

        // A second resolver sharing the content cache should HIT on content lookup!
        let mut assets2 = Assets::default();
        assets2.set_permissions([Permission::RbxAssetId]);
        assets2.set_content_cache(cache.clone());
        assets2.set_transport(Arc::new(MockTransport(|_| {
            panic!("should not fetch over network when content cache hits")
        })));

        let bitmap2 = assets2.resolve("rbxassetid://12345").expect("bitmap2");
        assert_eq!(bitmap2.rgba, vec![100, 150, 200, 255]);
        assert_eq!(cache.lock().unwrap().hits(), 1);
    }

    #[test]
    fn content_cache_deduplicates_identical_assets_across_different_uris() {
        let fixture = Fixture::new("dedupe");
        let mut assets = fixture.assets();
        assets.set_permissions([Permission::RbxAssetId]);

        let shared_png = png([42, 42, 42, 255]);
        let expected_hash = hash_bytes(&shared_png);

        let bytes_clone = shared_png.clone();
        assets.set_transport(Arc::new(MockTransport(move |_| Ok(bytes_clone.clone()))));

        assets.resolve("rbxassetid://111").expect("first");
        assets.resolve("rbxassetid://222").expect("second");

        let cache = assets.content_cache();
        let guard = cache.lock().unwrap();
        // Both URIs map to the exact same content hash
        assert_eq!(
            guard.hash_for_uri("rbxassetid://111"),
            Some(expected_hash.as_str())
        );
        assert_eq!(
            guard.hash_for_uri("rbxassetid://222"),
            Some(expected_hash.as_str())
        );
    }

    #[test]
    fn offline_or_network_error_fails_cleanly_without_hang() {
        let fixture = Fixture::new("offline");
        let mut assets = fixture.assets();
        assets.set_permissions([Permission::RbxAssetId]);

        assets.set_transport(Arc::new(MockTransport(|_| {
            Err("connection refused (host offline)".to_string())
        })));

        // Clean failure, returns None, does not panic or hang
        assert!(assets.resolve("rbxassetid://99999").is_none());
        // Second resolve reads cached None failure
        assert!(assets.resolve("rbxassetid://99999").is_none());
    }

    #[test]
    fn a_missing_file_is_answered_once_and_remembered() {
        let fixture = Fixture::new("missing");
        let mut assets = fixture.assets();
        assert!(assets.resolve("mod://absent.png").is_none());

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
