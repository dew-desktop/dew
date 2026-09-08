//! Does a dependency load under a real host that is not Roblox?
//!
//! WHY THIS EXISTS. A library written for Roblox has seams that are invisible on
//! Roblox and load-bearing off it. Dew supplies the real globals -- `Instance`,
//! `Enum`, `Color3`, a DataModel -- and no `game`, which is an environment the
//! library's own CI cannot produce and its authors cannot easily test. That is
//! the position from which vide's leaf modules were found unable to require at
//! all, and this binary is that discovery turned into a measurement.
//!
//! It reports rather than judges. A module that fails here is a CANDIDATE, not a
//! verdict: the next step is `.artifacts/project/upstream/README.md`, which says
//! what makes one worth sending.
//!
//! THE NUMBER IS WORTHLESS WITHOUT THE CONTROL. "every module loads" is also
//! what a probe that loads nothing prints. `--control` re-runs the probe with
//! `game` installed as a truthy value, which is what the Roblox-shaped guard
//! expects; a defect of this class disappears under it. A probe whose result
//! does not move between the two runs is not measuring what it claims.

use dew_host::datamodel::{self, SharedDom};
use dew_host::services::{self, Clock, SharedClock};
use dew_runtime::{modules, Capabilities, Vm};
use mlua::Value as LuaValue;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The host capability set, matching `dew_host::mods` in content and order.
///
/// `game` IS NOT HERE, and that is the whole point of the probe.
fn install_host(vm: &Vm) -> mlua::Result<()> {
    let dom: SharedDom = Arc::new(Mutex::new(Default::default()));
    datamodel::install(vm.lua(), &dom)?;
    let clock: SharedClock = Arc::new(Mutex::new(Clock::default()));
    services::install(vm.lua(), &clock)?;
    datamodel::install_vocabulary(vm.lua())?;
    std::mem::forget(dom);
    std::mem::forget(clock);
    Ok(())
}

fn luau_files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            luau_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "luau") {
            out.push(path);
        }
    }
}

/// Load one module in its own VM. A fresh VM per module is deliberate: a shared
/// one would let an earlier require populate the cache and hide a later failure.
fn probe_one(root: &Path, file: &Path, with_game: bool) -> Result<(), String> {
    let caps = Capabilities {
        require_roots: vec![root.to_path_buf()],
        print: false,
        aliases: Default::default(),
    };
    let vm = Vm::new(caps.clone()).map_err(|e| e.to_string())?;
    install_host(&vm).map_err(|e| e.to_string())?;
    if with_game {
        vm.lua()
            .globals()
            .set("game", true)
            .map_err(|e| e.to_string())?;
    }
    modules::install(&vm, &caps).map_err(|e| e.to_string())?;
    let chunk = modules::load_entry(&vm, file).map_err(|e| e.to_string())?;
    let _: LuaValue = chunk.call(()).map_err(|e| e.to_string())?;
    Ok(())
}

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or(s).trim()
}

struct Run {
    ok: usize,
    failures: Vec<(String, String)>,
}

fn run(root: &Path, files: &[PathBuf], with_game: bool) -> Run {
    let mut out = Run {
        ok: 0,
        failures: Vec::new(),
    };
    for file in files {
        let shown = file
            .strip_prefix(root)
            .unwrap_or(file)
            .display()
            .to_string()
            .replace(char::from(92u8), "/");
        match probe_one(root, file, with_game) {
            Ok(()) => out.ok += 1,
            Err(e) => out.failures.push((shown, first_line(&e).to_string())),
        }
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut path: Option<PathBuf> = None;
    let mut control = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--path" => {
                i += 1;
                path = args.get(i).map(PathBuf::from);
            }
            "--control" => control = true,
            other => {
                eprintln!("unknown flag: {other}");
                std::process::exit(2);
            }
        }
        i += 1;
    }

    let root = match path {
        Some(p) => p,
        None => match dew_runtime::installed_package("vide") {
            Some(v) => v.join("src"),
            None => {
                eprintln!("no --path given and no installed vide; run `pesde install`");
                std::process::exit(2);
            }
        },
    };

    if !root.is_dir() {
        eprintln!("not a directory: {}", root.display());
        std::process::exit(2);
    }

    let mut files = Vec::new();
    luau_files(&root, &mut files);
    files.sort();
    if files.is_empty() {
        eprintln!("no .luau modules under {}", root.display());
        std::process::exit(2);
    }

    println!("UPSTREAM PROBE: {}", root.display());
    let bare = run(&root, &files, false);
    println!(
        "  {} of {} modules load under a Dew host, with no `game`",
        bare.ok,
        files.len()
    );
    for (module, err) in &bare.failures {
        println!("    FAILS  {module}");
        println!("           {err}");
    }

    if control {
        let with = run(&root, &files, true);
        println!(
            "  CONTROL: {} of {} load with `game` present",
            with.ok,
            files.len()
        );

        let failed_bare: Vec<&String> = bare.failures.iter().map(|(m, _)| m).collect();
        let failed_with: Vec<&String> = with.failures.iter().map(|(m, _)| m).collect();

        // Fails without `game`, loads with it. This is the defect class.
        let candidates: Vec<&String> = failed_bare
            .iter()
            .filter(|m| !failed_with.contains(m))
            .copied()
            .collect();

        // Loads without `game`, fails with it. NOT a finding about the library:
        // the control installs `game` as a bare truthy value, so anything that
        // calls a method on it -- `game:GetService` -- fails against the stand-in
        // rather than against Roblox. Reported so the asymmetry is never read as
        // a result.
        let artifacts: Vec<&String> = failed_with
            .iter()
            .filter(|m| !failed_bare.contains(m))
            .copied()
            .collect();

        if candidates.is_empty() {
            println!("  No module depends on `game` merely being present.");
        } else {
            println!(
                "  {} module(s) load ONLY because `game` is there. That gap is the candidate:",
                candidates.len()
            );
            for m in &candidates {
                println!("    {m}");
            }
        }

        if !artifacts.is_empty() {
            println!(
                "  {} module(s) fail only under the control, which is the control's",
                artifacts.len()
            );
            println!("  limitation and not a finding: `game` is a bare truthy value here, so a");
            println!("  call like `game:GetService` has nothing to answer it.");
            for m in &artifacts {
                println!("    {m}");
            }
        }
    }
}
