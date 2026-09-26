//! Where a dragged widget was left, across relaunches.
//!
//! HOST-INTERNAL, AND NOTHING TO DO WITH `desktop.Storage`. Storage is ephemeral,
//! process-lifetime only, and an applet's own code never reaches this file --
//! matching how `draggable` itself is host-driven rather than scriptable. A
//! flat `{ "applet-id": [x, y] }` map is enough; there is only ever one
//! position to remember per applet.
//!
//! ONE FILE, REWRITTEN WHOLE. This is written only when a drag ends with
//! `savePosition` on, which is rare enough that an incremental update would be
//! solving a problem that does not exist yet.

#![cfg(windows)]

use std::collections::HashMap;
use std::path::PathBuf;

fn path() -> Option<PathBuf> {
    let dir = dirs::data_local_dir()?.join("Dew");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("positions.json"))
}

fn read_all() -> HashMap<String, (i32, i32)> {
    let Some(path) = path() else {
        return HashMap::new();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return HashMap::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

/// Where this applet was last dragged to, if it ever was.
pub fn load(id: &str) -> Option<(i32, i32)> {
    read_all().get(id).copied()
}

/// Remember where this applet was dragged to, for its next launch.
pub fn save(id: &str, position: (i32, i32)) {
    let Some(path) = path() else { return };
    let mut all = read_all();
    all.insert(id.to_string(), position);
    if let Ok(text) = serde_json::to_string_pretty(&all) {
        let _ = std::fs::write(&path, text);
    }
}
