//! `desktop.System`: what the machine is doing, for an applet granted `system`.
//!
//! ONE SAMPLER, HOWEVER MANY APPLETS ASK. Reading CPU load is a difference
//! between two readings taken some time apart, so it cannot be answered on the
//! spot by a Luau call -- something has to have been watching. That something is
//! one background thread per host process, started by the first applet granted
//! `system` and stopped once the last one is gone. Every applet reads the same
//! published [`Snapshot`], so two monitors on the desktop cost one sampler, not
//! two, and an applet opened late still sees the last minute of [`History`].
//!
//! READS NEVER BLOCK ON THE MACHINE. A Luau call copies the latest snapshot into
//! a table and returns; the thread does the slow part once a second whether
//! anything asks or not. The one exception is the very first read after the
//! sampler starts, which waits for its first snapshot rather than handing back a
//! CPU reading of zero that was never measured.
//!
//! PULL FOR VALUES, PUSH FOR "SOMETHING CHANGED". `Sequence()` counts snapshots
//! and `OnSample(fn)` calls back on the applet's own frame when it advances, so
//! an applet rebuilds once per sample instead of once per frame. That is the
//! shape a later media or audio source should reuse: a host thread publishes,
//! a counter says when, and Luau reads on its own thread.
//!
//! WHAT THE PERMISSION DISCLOSES. Load, memory, disk capacity, network
//! throughput, battery, uptime and the OS name -- and the names of running
//! processes with their share of CPU and memory, which is the part worth
//! reading twice before granting it. No command lines, no paths, no pids, no
//! user names, no hostname, no addresses.

use mlua::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::time::{Duration, Instant};
use sysinfo::{
    CpuRefreshKind, DiskKind, Disks, MemoryRefreshKind, Networks, ProcessRefreshKind,
    ProcessesToUpdate, System,
};

/// How often the sampler reads the machine.
pub const INTERVAL: Duration = Duration::from_secs(1);

/// How many samples [`History`] keeps per metric: a minute at [`INTERVAL`].
pub const HISTORY: usize = 60;

/// How many process groups a snapshot keeps, busiest first.
const PROCESSES: usize = 16;

/// How long the first read waits for the first snapshot before giving up and
/// returning an empty one.
const FIRST_SAMPLE: Duration = Duration::from_secs(3);

#[derive(Clone, Debug, Default)]
pub struct Cpu {
    /// Whole-machine load, 0-100.
    pub usage: f32,
    /// Per logical core, 0-100, in the order the OS numbers them.
    pub cores: Vec<f32>,
    pub brand: String,
    /// MHz as the OS reports it, which on Windows is the nominal clock.
    pub frequency: u64,
    pub physical_cores: Option<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct Memory {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub swap_total: u64,
    pub swap_used: u64,
}

#[derive(Clone, Debug)]
pub struct Disk {
    pub name: String,
    pub mount: String,
    pub total: u64,
    pub available: u64,
    pub removable: bool,
    pub kind: &'static str,
}

#[derive(Clone, Debug, Default)]
pub struct Network {
    /// Bytes per second received, summed over every non-loopback interface.
    pub down: f64,
    /// Bytes per second sent.
    pub up: f64,
    pub received_total: u64,
    pub transmitted_total: u64,
}

/// Every process sharing a name, added together: a browser is one row, not
/// thirty.
#[derive(Clone, Debug)]
pub struct Process {
    pub name: String,
    /// Share of the whole machine, 0-100, not of one core.
    pub cpu: f32,
    pub memory: u64,
    pub count: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub cpu: Cpu,
    pub memory: Memory,
    pub disks: Vec<Disk>,
    pub network: Network,
    pub processes: Vec<Process>,
    pub uptime: u64,
    pub os: String,
}

/// The last [`HISTORY`] samples of each metric, oldest first.
#[derive(Clone, Debug, Default)]
pub struct History {
    pub cpu: VecDeque<f64>,
    pub memory: VecDeque<f64>,
    pub down: VecDeque<f64>,
    pub up: VecDeque<f64>,
}

impl History {
    fn push(&mut self, snapshot: &Snapshot) {
        let memory = if snapshot.memory.total == 0 {
            0.0
        } else {
            snapshot.memory.used as f64 / snapshot.memory.total as f64 * 100.0
        };
        for (series, value) in [
            (&mut self.cpu, snapshot.cpu.usage as f64),
            (&mut self.memory, memory),
            (&mut self.down, snapshot.network.down),
            (&mut self.up, snapshot.network.up),
        ] {
            if series.len() == HISTORY {
                series.pop_front();
            }
            series.push_back(value);
        }
    }

    /// The series a metric names, or `None` for a name that is not one.
    pub fn series(&self, metric: &str) -> Option<&VecDeque<f64>> {
        match metric {
            "cpu" => Some(&self.cpu),
            "memory" => Some(&self.memory),
            "down" => Some(&self.down),
            "up" => Some(&self.up),
            _ => None,
        }
    }
}

#[derive(Default)]
struct Published {
    snapshot: Arc<Snapshot>,
    history: History,
    ready: bool,
}

/// The process-wide sampler. Hold an `Arc` to keep it running.
pub struct Sampler {
    published: Mutex<Published>,
    first: Condvar,
    sequence: AtomicU64,
}

/// THE ONE SAMPLER, HELD WEAKLY. Applets hold the strong references, so the
/// thread notices the last applet going away and stops, and the next applet to
/// ask starts a fresh one.
static SHARED: Mutex<Weak<Sampler>> = Mutex::new(Weak::new());

impl Sampler {
    /// The running sampler, starting it if nothing holds one.
    pub fn shared() -> Arc<Sampler> {
        let mut slot = SHARED.lock().expect("system sampler");
        if let Some(running) = slot.upgrade() {
            return running;
        }
        let sampler = Arc::new(Sampler {
            published: Mutex::new(Published::default()),
            first: Condvar::new(),
            sequence: AtomicU64::new(0),
        });
        *slot = Arc::downgrade(&sampler);
        let weak = Arc::downgrade(&sampler);
        std::thread::Builder::new()
            .name("dew-system-sampler".into())
            .spawn(move || run(weak))
            .expect("spawn the system sampler");
        sampler
    }

    /// The latest snapshot. Waits for the first one if none is published yet.
    pub fn latest(&self) -> Arc<Snapshot> {
        let published = self.published.lock().expect("system snapshot");
        let (published, _) = self
            .first
            .wait_timeout_while(published, FIRST_SAMPLE, |p| !p.ready)
            .expect("system snapshot");
        Arc::clone(&published.snapshot)
    }

    /// How many snapshots have been published. Zero until the first.
    pub fn sequence(&self) -> u64 {
        self.sequence.load(Ordering::Acquire)
    }

    /// A copy of one metric's history, oldest first.
    pub fn history(&self, metric: &str) -> Option<Vec<f64>> {
        let _ = self.latest();
        let published = self.published.lock().expect("system snapshot");
        published
            .history
            .series(metric)
            .map(|s| s.iter().copied().collect())
    }

    fn publish(&self, snapshot: Snapshot) {
        let mut published = self.published.lock().expect("system snapshot");
        published.history.push(&snapshot);
        published.snapshot = Arc::new(snapshot);
        published.ready = true;
        self.sequence.fetch_add(1, Ordering::Release);
        self.first.notify_all();
    }
}

fn run(weak: Weak<Sampler>) {
    let mut probe = Probe::new();
    loop {
        let snapshot = probe.sample();
        let Some(sampler) = weak.upgrade() else {
            return;
        };
        sampler.publish(snapshot);
        drop(sampler);
        std::thread::sleep(INTERVAL);
    }
}

/// What the sampler thread owns: sysinfo's handles and when they were last
/// refreshed, for turning byte counters into rates.
struct Probe {
    system: System,
    disks: Disks,
    networks: Networks,
    last: Instant,
}

impl Probe {
    /// Read everything once, then wait sysinfo's minimum interval, so the first
    /// [`Probe::sample`] has a real CPU difference to report.
    fn new() -> Self {
        let mut system = System::new();
        system.refresh_cpu_specifics(CpuRefreshKind::everything());
        system.refresh_processes_specifics(ProcessesToUpdate::All, true, process_refresh());
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        let last = Instant::now();
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        Probe {
            system,
            disks,
            networks,
            last,
        }
    }

    fn sample(&mut self) -> Snapshot {
        let system = &mut self.system;
        system.refresh_cpu_specifics(CpuRefreshKind::nothing().with_cpu_usage().with_frequency());
        system.refresh_memory_specifics(MemoryRefreshKind::everything());
        system.refresh_processes_specifics(ProcessesToUpdate::All, true, process_refresh());
        self.disks.refresh(true);
        self.networks.refresh(true);
        let now = Instant::now();
        let elapsed = now.duration_since(self.last).as_secs_f64().max(0.001);
        self.last = now;

        let cpus = system.cpus();
        let cpu = Cpu {
            usage: system.global_cpu_usage().clamp(0.0, 100.0),
            cores: cpus
                .iter()
                .map(|c| c.cpu_usage().clamp(0.0, 100.0))
                .collect(),
            brand: cpus
                .first()
                .map(|c| c.brand().trim().to_string())
                .unwrap_or_default(),
            frequency: cpus.first().map(|c| c.frequency()).unwrap_or(0),
            physical_cores: System::physical_core_count(),
        };

        let memory = Memory {
            total: system.total_memory(),
            used: system.used_memory(),
            available: system.available_memory(),
            swap_total: system.total_swap(),
            swap_used: system.used_swap(),
        };

        // ONE ROW PER MOUNT POINT. Linux lists the same filesystem under every
        // bind mount, and a list of eleven identical disks is not information.
        let mut seen = std::collections::HashSet::new();
        let disks = self
            .disks
            .list()
            .iter()
            .filter(|d| d.total_space() > 0)
            .filter(|d| seen.insert(d.mount_point().to_path_buf()))
            .map(|d| Disk {
                name: d.name().to_string_lossy().into_owned(),
                mount: d.mount_point().to_string_lossy().into_owned(),
                total: d.total_space(),
                available: d.available_space(),
                removable: d.is_removable(),
                kind: match d.kind() {
                    DiskKind::SSD => "ssd",
                    DiskKind::HDD => "hdd",
                    DiskKind::Unknown(_) => "unknown",
                },
            })
            .collect();

        let mut network = Network::default();
        for (name, data) in self.networks.list() {
            if is_loopback(name) {
                continue;
            }
            network.down += data.received() as f64 / elapsed;
            network.up += data.transmitted() as f64 / elapsed;
            network.received_total += data.total_received();
            network.transmitted_total += data.total_transmitted();
        }

        Snapshot {
            processes: top_processes(system),
            cpu,
            memory,
            disks,
            network,
            uptime: System::uptime(),
            os: System::long_os_version().unwrap_or_default(),
        }
    }
}

fn process_refresh() -> ProcessRefreshKind {
    ProcessRefreshKind::nothing().with_cpu().with_memory()
}

fn is_loopback(name: &str) -> bool {
    name == "lo" || name.to_ascii_lowercase().starts_with("loopback")
}

/// Processes grouped by name, busiest first, then largest.
fn top_processes(system: &System) -> Vec<Process> {
    let cores = system.cpus().len().max(1) as f32;
    let mut groups: HashMap<String, Process> = HashMap::new();
    for process in system.processes().values() {
        // Windows' "System Idle Process" is pid 0 and reports the idle time
        // as its own load; it would always top the list.
        if process.pid().as_u32() == 0 {
            continue;
        }
        // Linux lists every thread beside its process, under the thread's own
        // name; counting both would put a browser's worker pool in the top
        // five and count its load twice.
        if process.thread_kind().is_some() {
            continue;
        }
        let name = process.name().to_string_lossy().into_owned();
        let group = groups.entry(name.clone()).or_insert(Process {
            name,
            cpu: 0.0,
            memory: 0,
            count: 0,
        });
        group.cpu += process.cpu_usage() / cores;
        group.memory += process.memory();
        group.count += 1;
    }
    let mut list: Vec<Process> = groups.into_values().collect();
    list.sort_by(|a, b| {
        b.cpu
            .total_cmp(&a.cpu)
            .then(b.memory.cmp(&a.memory))
            .then(a.name.cmp(&b.name))
    });
    list.truncate(PROCESSES);
    for process in &mut list {
        process.cpu = process.cpu.clamp(0.0, 100.0);
    }
    list
}

/// Charge, read when asked: it changes slowly and costs one system call.
#[derive(Clone, Debug)]
pub struct Battery {
    pub percent: f64,
    pub charging: bool,
    pub plugged: bool,
    pub seconds_left: Option<u64>,
}

/// The battery, or `None` on a machine without one or where the host cannot
/// read it.
#[cfg(windows)]
pub fn battery() -> Option<Battery> {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut status = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut status) }.ok()?;
    // 128 is "no system battery", 255 "unknown status".
    if status.BatteryFlag & 128 != 0 || status.BatteryFlag == 255 {
        return None;
    }
    if status.BatteryLifePercent > 100 {
        return None;
    }
    Some(Battery {
        percent: status.BatteryLifePercent as f64,
        charging: status.BatteryFlag & 8 != 0,
        plugged: status.ACLineStatus == 1,
        seconds_left: (status.BatteryLifeTime != u32::MAX).then_some(status.BatteryLifeTime as u64),
    })
}

/// The first battery under `/sys/class/power_supply`, on Linux.
#[cfg(not(windows))]
pub fn battery() -> Option<Battery> {
    let entries = std::fs::read_dir("/sys/class/power_supply").ok()?;
    for entry in entries.flatten() {
        let dir = entry.path();
        let read = |file: &str| std::fs::read_to_string(dir.join(file)).ok();
        if read("type").as_deref().map(str::trim) != Some("Battery") {
            continue;
        }
        let percent: f64 = read("capacity")?.trim().parse().ok()?;
        let status = read("status").unwrap_or_default();
        let status = status.trim();
        return Some(Battery {
            percent: percent.clamp(0.0, 100.0),
            charging: status == "Charging",
            plugged: status != "Discharging",
            seconds_left: None,
        });
    }
    None
}

/// Build the `desktop.System` table on `sampler`.
///
/// `desktop` is the table this will be set on; `OnSample` reaches its `Clock`
/// when called rather than now, so the order the host fills `desktop` in does
/// not matter.
pub fn table(lua: &Lua, desktop: &LuaTable, sampler: Arc<Sampler>) -> LuaResult<LuaTable> {
    let system = lua.create_table()?;

    let s = Arc::clone(&sampler);
    system.set(
        "Cpu",
        lua.create_function(move |lua, ()| {
            let snapshot = s.latest();
            let cpu = &snapshot.cpu;
            let t = lua.create_table()?;
            t.set("usage", cpu.usage)?;
            t.set(
                "cores",
                lua.create_sequence_from(cpu.cores.iter().copied())?,
            )?;
            t.set("brand", cpu.brand.as_str())?;
            t.set("frequency", cpu.frequency)?;
            t.set("physicalCores", cpu.physical_cores)?;
            Ok(t)
        })?,
    )?;

    let s = Arc::clone(&sampler);
    system.set(
        "Memory",
        lua.create_function(move |lua, ()| {
            let snapshot = s.latest();
            let m = &snapshot.memory;
            let t = lua.create_table()?;
            t.set("total", m.total as f64)?;
            t.set("used", m.used as f64)?;
            t.set("available", m.available as f64)?;
            t.set("swapTotal", m.swap_total as f64)?;
            t.set("swapUsed", m.swap_used as f64)?;
            Ok(t)
        })?,
    )?;

    let s = Arc::clone(&sampler);
    system.set(
        "Disks",
        lua.create_function(move |lua, ()| {
            let snapshot = s.latest();
            let list = lua.create_table()?;
            for (i, d) in snapshot.disks.iter().enumerate() {
                let t = lua.create_table()?;
                t.set("name", d.name.as_str())?;
                t.set("mount", d.mount.as_str())?;
                t.set("total", d.total as f64)?;
                t.set("available", d.available as f64)?;
                t.set("removable", d.removable)?;
                t.set("kind", d.kind)?;
                list.set(i + 1, t)?;
            }
            Ok(list)
        })?,
    )?;

    let s = Arc::clone(&sampler);
    system.set(
        "Network",
        lua.create_function(move |lua, ()| {
            let snapshot = s.latest();
            let n = &snapshot.network;
            let t = lua.create_table()?;
            t.set("down", n.down)?;
            t.set("up", n.up)?;
            t.set("receivedTotal", n.received_total as f64)?;
            t.set("transmittedTotal", n.transmitted_total as f64)?;
            Ok(t)
        })?,
    )?;

    let s = Arc::clone(&sampler);
    system.set(
        "Processes",
        lua.create_function(move |lua, limit: Option<usize>| {
            let snapshot = s.latest();
            let limit = limit.unwrap_or(5).min(PROCESSES);
            let list = lua.create_table()?;
            for (i, p) in snapshot.processes.iter().take(limit).enumerate() {
                let t = lua.create_table()?;
                t.set("name", p.name.as_str())?;
                t.set("cpu", p.cpu)?;
                t.set("memory", p.memory as f64)?;
                t.set("count", p.count)?;
                list.set(i + 1, t)?;
            }
            Ok(list)
        })?,
    )?;

    let s = Arc::clone(&sampler);
    system.set(
        "Info",
        lua.create_function(move |lua, ()| {
            let snapshot = s.latest();
            let t = lua.create_table()?;
            t.set("os", snapshot.os.as_str())?;
            t.set("uptime", snapshot.uptime)?;
            t.set("cores", snapshot.cpu.cores.len())?;
            Ok(t)
        })?,
    )?;

    system.set(
        "Battery",
        lua.create_function(|lua, ()| {
            let Some(b) = battery() else {
                return Ok(LuaValue::Nil);
            };
            let t = lua.create_table()?;
            t.set("percent", b.percent)?;
            t.set("charging", b.charging)?;
            t.set("plugged", b.plugged)?;
            t.set("secondsLeft", b.seconds_left)?;
            Ok(LuaValue::Table(t))
        })?,
    )?;

    let s = Arc::clone(&sampler);
    system.set(
        "History",
        lua.create_function(move |lua, metric: String| match s.history(&metric) {
            Some(series) => lua.create_sequence_from(series),
            None => Err(LuaError::runtime(format!(
                "desktop.System.History: unknown metric {metric:?} (expected \"cpu\", \"memory\", \"down\" or \"up\")"
            ))),
        })?,
    )?;

    let s = Arc::clone(&sampler);
    let sequence = lua.create_function(move |_, ()| Ok(s.sequence()))?;
    system.set("Sequence", sequence.clone())?;

    // EVERY LISTENER IS A FRAME LISTENER THAT LOOKS AT A COUNTER. The sampler
    // thread cannot call Luau -- the VM belongs to the applet's thread -- so
    // it bumps `Sequence` and each subscriber notices on its next frame. The
    // disposer handed back is `Clock.OnFrame`'s own.
    let on_sample: LuaFunction = lua
        .load(
            r#"
            local desktop, sequence = ...
            return function(listener)
                assert(type(listener) == "function", "desktop.System.OnSample expects a function")
                local seen = sequence()
                return desktop.Clock.OnFrame(function()
                    local now = sequence()
                    if now ~= seen then
                        seen = now
                        listener(now)
                    end
                end)
            end
            "#,
        )
        .set_name("=desktop.System.OnSample")
        .call((desktop.clone(), sequence))?;
    system.set("OnSample", on_sample)?;

    Ok(system)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_snapshot_reads_the_machine() {
        let sampler = Sampler::shared();
        let snapshot = sampler.latest();
        assert!(sampler.sequence() >= 1, "the first read waits for a sample");
        assert!(!snapshot.cpu.cores.is_empty(), "a machine has a core");
        assert!((0.0..=100.0).contains(&snapshot.cpu.usage));
        assert!(snapshot.memory.total > 0);
        assert!(snapshot.memory.used <= snapshot.memory.total);
        for process in &snapshot.processes {
            assert!((0.0..=100.0).contains(&process.cpu), "{process:?}");
        }
        assert!(snapshot.processes.len() <= PROCESSES);
        let history = sampler.history("cpu").expect("cpu is a metric");
        assert!(!history.is_empty() && history.len() <= HISTORY);
        assert!(sampler.history("temperature").is_none());
    }

    #[test]
    fn history_keeps_a_minute() {
        let mut history = History::default();
        for i in 0..(HISTORY + 5) {
            let mut snapshot = Snapshot::default();
            snapshot.cpu.usage = i as f32;
            history.push(&snapshot);
        }
        assert_eq!(history.cpu.len(), HISTORY);
        assert_eq!(history.cpu.front().copied(), Some(5.0), "oldest first");
        assert_eq!(history.memory.back().copied(), Some(0.0), "no total is 0%");
    }

    #[test]
    fn the_table_answers_in_luau() {
        let lua = Lua::new();
        let desktop = lua.create_table().expect("desktop");
        let system = table(&lua, &desktop, Sampler::shared()).expect("table");
        desktop.set("System", system).expect("set");
        lua.globals().set("desktop", desktop).expect("global");
        lua.load(
            r#"
            local s = desktop.System
            local cpu = s.Cpu()
            assert(type(cpu.usage) == "number" and #cpu.cores > 0)
            assert(s.Memory().total > 0)
            assert(type(s.Disks()) == "table")
            assert(type(s.Network().down) == "number")
            assert(#s.Processes(3) <= 3)
            assert(type(s.Info().os) == "string")
            assert(#s.History("cpu") >= 1)
            assert(s.Sequence() >= 1)
            assert(not pcall(s.History, "temperature"))
            "#,
        )
        .exec()
        .expect("desktop.System");
    }
}
