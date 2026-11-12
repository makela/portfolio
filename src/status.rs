//! Resource usage of the sites on this server, for `/status` and the front
//! page's footer.
//!
//! The numbers come from the kernel, the same places top reads: memory and CPU
//! time of each systemd service from its cgroup (what `systemd-cgtop` shows),
//! and the host's load, memory and CPU time from /proc. The server only
//! reports cumulative counters and keeps no history: `/static/status.js` turns
//! two readings into CPU percentages and keeps the recent history in the
//! browser.

use std::{
    fmt::Write as _,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant},
};

use axum::{
    extract::State,
    http::header::{CACHE_CONTROL, CONTENT_TYPE},
    response::IntoResponse,
};
use maud::{html, Markup};

use crate::{
    assets, json,
    site::{name, Config},
    templates::{self, Page},
};

/// Where systemd puts the cgroups of system services. `STATUS_CGROUP_ROOT`
/// overrides it, e.g. for `just watch`, which runs in a user scope.
pub(crate) const CGROUP_ROOT: &str = "/sys/fs/cgroup/system.slice";
const PROC_ROOT: &str = "/proc";

/// Readings are reused for this long, so a burst of requests (or a crawler)
/// costs one round of file reads. status.js polls less often than this.
const MAX_AGE: Duration = Duration::from_secs(2);

/// A site and the systemd units that serve it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Site {
    pub(crate) name: String,
    /// Unit names in `system.slice`, e.g. `blog.service`.
    pub(crate) units: Vec<String>,
}

/// Parses `STATUS_SITES`: `Name=unit,unit;Name=unit`. A unit without a
/// suffix is a service (`blog` is `blog.service`). The NixOS module writes
/// this from `services.portfolio.status.sites`.
pub(crate) fn parse_sites(spec: &str) -> Result<Vec<Site>, String> {
    spec.split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            let (name, units) = entry
                .split_once('=')
                .ok_or_else(|| format!("{entry:?}: expected Name=unit,unit"))?;
            let name = name.trim();
            if name.is_empty() {
                return Err(format!("{entry:?}: the site has no name"));
            }
            let units = units
                .split(',')
                .map(str::trim)
                .filter(|unit| !unit.is_empty())
                .map(unit_name)
                .collect::<Result<Vec<_>, _>>()?;
            if units.is_empty() {
                return Err(format!("{name:?}: the site has no units"));
            }
            Ok(Site {
                name: name.to_string(),
                units,
            })
        })
        .collect()
}

/// A unit name that is safe to use as a directory name under the cgroup root.
fn unit_name(unit: &str) -> Result<String, String> {
    let valid = !unit.starts_with('.')
        && unit
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "@._:-".contains(c));
    if !valid {
        return Err(format!("{unit:?} is not a systemd unit name"));
    }
    Ok(if unit.contains('.') {
        unit.to_string()
    } else {
        format!("{unit}.service")
    })
}

/// One reading of every counter. Missing values (e.g. no cgroups on a
/// development machine) are `None` and shown as "n/a".
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Snapshot {
    /// Seconds since the host booted, in hundredths: the clock that status.js
    /// measures CPU percentages against.
    pub(crate) uptime_cs: Option<u64>,
    pub(crate) host: Host,
    pub(crate) sites: Vec<SiteUsage>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Host {
    /// Load averages over 1, 5 and 15 minutes.
    pub(crate) load: Option<[f64; 3]>,
    /// Memory in use (total minus available) in bytes. The total isn't
    /// shown or reported, only how the used memory splits.
    pub(crate) memory_used: Option<u64>,
    /// CPU time of all cores, busy and in total, in clock ticks.
    pub(crate) cpu_busy: Option<u64>,
    pub(crate) cpu_total: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SiteUsage {
    pub(crate) name: String,
    /// Whether any of its units is running (has a cgroup).
    pub(crate) running: bool,
    /// How many units its numbers add up: those configured for it, and of
    /// them those running.
    pub(crate) services: usize,
    pub(crate) services_running: usize,
    /// Memory of its units in bytes, file cache included, as systemd counts it.
    pub(crate) memory: u64,
    /// CPU time of its units in microseconds.
    pub(crate) cpu_usec: u64,
    /// How long the oldest process of its running units has run, in
    /// hundredths of a second.
    pub(crate) up_cs: Option<u64>,
}

/// The latest reading and when it was taken.
type Cache = Arc<Mutex<Option<(Instant, Arc<Snapshot>)>>>;

/// Reads the counters, at most once every `MAX_AGE`. Cheap to clone.
#[derive(Debug, Clone)]
pub(crate) struct Monitor {
    sites: Arc<[Site]>,
    cgroups: PathBuf,
    proc: PathBuf,
    cache: Cache,
}

impl Monitor {
    /// Reads the units' cgroups in `cgroups`, normally [`CGROUP_ROOT`].
    pub(crate) fn new(sites: Vec<Site>, cgroups: &str) -> Self {
        Self::with_roots(sites, cgroups, PROC_ROOT)
    }

    fn with_roots(sites: Vec<Site>, cgroups: impl Into<PathBuf>, proc: impl Into<PathBuf>) -> Self {
        Self {
            sites: sites.into(),
            cgroups: cgroups.into(),
            proc: proc.into(),
            cache: Arc::default(),
        }
    }

    /// The latest reading. The files are small and live in kernel memory, so
    /// reading them inline is cheaper than handing them to a blocking thread.
    pub(crate) fn snapshot(&self) -> Arc<Snapshot> {
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((read_at, snapshot)) = cache.as_ref() {
            if read_at.elapsed() < MAX_AGE {
                return Arc::clone(snapshot);
            }
        }
        let snapshot = Arc::new(self.read());
        *cache = Some((Instant::now(), Arc::clone(&snapshot)));
        snapshot
    }

    fn read(&self) -> Snapshot {
        let proc_file = |name: &str| fs::read_to_string(self.proc.join(name)).ok();
        let uptime_cs = proc_file("uptime").as_deref().and_then(parse_uptime);
        let cpu = proc_file("stat").as_deref().map(parse_stat);
        let host = Host {
            load: proc_file("loadavg").as_deref().and_then(parse_loadavg),
            memory_used: proc_file("meminfo").as_deref().and_then(parse_memory_used),
            cpu_busy: cpu.and_then(|(busy, _)| busy),
            cpu_total: cpu.and_then(|(_, total)| total),
        };
        let sites = self
            .sites
            .iter()
            .map(|site| self.site_usage(site, uptime_cs))
            .collect();
        Snapshot {
            uptime_cs,
            host,
            sites,
        }
    }

    fn site_usage(&self, site: &Site, uptime_cs: Option<u64>) -> SiteUsage {
        let mut usage = SiteUsage {
            name: site.name.clone(),
            services: site.units.len(),
            ..SiteUsage::default()
        };
        let mut started: Option<u64> = None;
        for unit in &site.units {
            let dir = self.cgroups.join(unit);
            if !dir.is_dir() {
                continue; // not running
            }
            usage.running = true;
            usage.services_running += 1;
            let file = |name: &str| fs::read_to_string(dir.join(name)).ok();
            usage.memory += file("memory.current")
                .and_then(|s| s.trim().parse::<u64>().ok())
                .unwrap_or(0);
            usage.cpu_usec += file("cpu.stat")
                .as_deref()
                .and_then(parse_cpu_usage)
                .unwrap_or(0);
            let oldest = file("cgroup.procs").as_deref().and_then(|pids| {
                pids.split_whitespace()
                    .filter_map(|pid| fs::read_to_string(self.proc.join(pid).join("stat")).ok())
                    .filter_map(|stat| parse_start_time(&stat))
                    .min()
            });
            started = started.into_iter().chain(oldest).min();
        }
        usage.up_cs = uptime_cs
            .zip(started)
            .map(|(now, started)| now.saturating_sub(started));
        usage
    }
}

/// `/proc/uptime`: "12345.67 54321.00" → 1234567.
fn parse_uptime(contents: &str) -> Option<u64> {
    let seconds = contents.split_whitespace().next()?;
    let (whole, fraction) = seconds.split_once('.').unwrap_or((seconds, "0"));
    let hundredths: u64 = format!("{fraction:0<2}").get(..2)?.parse().ok()?;
    Some(whole.parse::<u64>().ok()? * 100 + hundredths)
}

/// `/proc/loadavg`: "0.42 0.30 0.25 1/123 4567".
fn parse_loadavg(contents: &str) -> Option<[f64; 3]> {
    let mut fields = contents.split_whitespace().map(str::parse::<f64>);
    Some([
        fields.next()?.ok()?,
        fields.next()?.ok()?,
        fields.next()?.ok()?,
    ])
}

/// `/proc/meminfo`: memory in use, `MemTotal` minus `MemAvailable`, in bytes.
fn parse_memory_used(contents: &str) -> Option<u64> {
    let field = |key: &str| {
        contents.lines().find_map(|line| {
            let kib = line.strip_prefix(key)?.strip_prefix(':')?;
            let kib: u64 = kib.trim().strip_suffix("kB")?.trim().parse().ok()?;
            Some(kib * 1024)
        })
    };
    Some(field("MemTotal")?.saturating_sub(field("MemAvailable")?))
}

/// `/proc/stat`: busy and total CPU ticks of all cores together. A tick is a
/// hundredth of a second (see [`parse_start_time`]), so status.js can turn a
/// site's CPU microseconds into a share of the whole server without knowing
/// how many cores it has.
fn parse_stat(contents: &str) -> (Option<u64>, Option<u64>) {
    let ticks = contents
        .lines()
        .find_map(|line| line.strip_prefix("cpu "))
        .map(|line| {
            line.split_whitespace()
                .map(|field| field.parse::<u64>().unwrap_or(0))
                .collect::<Vec<_>>()
        });
    // user nice system idle iowait irq softirq steal; guest time is already
    // counted in user.
    let total = ticks
        .as_ref()
        .map(|ticks| ticks.iter().take(8).sum::<u64>());
    let idle = ticks
        .as_ref()
        .map(|ticks| ticks.get(3).copied().unwrap_or(0) + ticks.get(4).copied().unwrap_or(0));
    let busy = total
        .zip(idle)
        .map(|(total, idle)| total.saturating_sub(idle));
    (busy, total)
}

/// A cgroup's `cpu.stat`: `usage_usec`.
fn parse_cpu_usage(contents: &str) -> Option<u64> {
    contents.lines().find_map(|line| {
        line.strip_prefix("usage_usec ")
            .and_then(|usec| usec.trim().parse().ok())
    })
}

/// `/proc/<pid>/stat`: when the process started, in clock ticks since boot
/// (field 22). Linux reports these in `USER_HZ`, which is 100 on every
/// architecture NixOS runs on, so they are hundredths of a second.
fn parse_start_time(stat: &str) -> Option<u64> {
    // The process name (field 2) may contain spaces and parentheses, so count
    // the fields after its closing parenthesis; the first one is field 3.
    let (_, rest) = stat.rsplit_once(')')?;
    rest.split_whitespace().nth(22 - 3)?.parse().ok()
}

/// "162 MB", "1.4 GB": decimal units, like the sizes people see elsewhere.
pub(crate) fn format_bytes(bytes: u64) -> String {
    const MB: u64 = 1_000_000;
    const GB: u64 = 1_000_000_000;
    if bytes >= 10 * GB {
        format!("{} GB", (bytes + GB / 2) / GB)
    } else if bytes >= GB {
        let tenths = (bytes + GB / 20) / (GB / 10);
        format!("{}.{} GB", tenths / 10, tenths % 10)
    } else {
        format!("{} MB", (bytes + MB / 2) / MB)
    }
}

/// "45 s", "12 min", "3 h 5 min", "12 days": the two largest units at most.
pub(crate) fn format_duration(seconds: u64) -> String {
    let (days, hours, minutes) = (seconds / 86_400, seconds / 3600 % 24, seconds / 60 % 60);
    match () {
        () if days >= 2 => format!("{days} days"),
        () if days == 1 => format!("1 day {hours} h"),
        () if hours > 0 => format!("{hours} h {minutes} min"),
        () if minutes > 0 => format!("{minutes} min"),
        () => format!("{seconds} s"),
    }
}

/// Whether any configured site is running, i.e. there is something to draw
/// in the footer charts.
pub(crate) fn any_running(snapshot: &Snapshot) -> bool {
    snapshot.sites.iter().any(|site| site.running)
}

/// `/status.json` for status.js.
pub(crate) fn to_json(snapshot: &Snapshot) -> String {
    fn number(value: Option<impl std::fmt::Display>) -> String {
        value.map_or_else(|| "null".to_string(), |v| v.to_string())
    }
    let host = &snapshot.host;
    let load = host
        .load
        .map(|[one, five, fifteen]| format!("[{one},{five},{fifteen}]"));
    let mut json = format!(
        "{{\"uptime_cs\":{},\"host\":{{\"load\":{},\"memory_used\":{},\
         \"cpu_busy\":{},\"cpu_total\":{}}},\"sites\":[",
        number(snapshot.uptime_cs),
        load.as_deref().unwrap_or("null"),
        number(host.memory_used),
        number(host.cpu_busy),
        number(host.cpu_total),
    );
    for (i, site) in snapshot.sites.iter().enumerate() {
        let _ = write!(
            json,
            "{}{{\"name\":{},\"running\":{},\"services\":{},\"services_running\":{},\
             \"memory\":{},\"cpu_usec\":{},\"up_cs\":{}}}",
            if i == 0 { "" } else { "," },
            json::string(&site.name),
            site.running,
            site.services,
            site.services_running,
            site.memory,
            site.cpu_usec,
            number(site.up_cs),
        );
    }
    json.push_str("]}");
    json
}

pub(crate) const PAGE: Page = Page {
    path: "/status",
    title: concat!("Server status — ", name!()),
    description: "Live memory and CPU use of the sites on this server, read straight \
         from the Linux kernel by a small Rust web server, like top does.",
    indexed: true,
    name_is_heading: false,
};

pub(crate) async fn page(State(config): State<Config>) -> impl IntoResponse {
    let snapshot = config.status.snapshot();
    (
        [(CACHE_CONTROL, assets::NO_CACHE)],
        templates::layout(&config, &PAGE, &status_page(&snapshot)),
    )
}

pub(crate) async fn json(State(config): State<Config>) -> impl IntoResponse {
    (
        [
            (CONTENT_TYPE, "application/json"),
            (CACHE_CONTROL, "no-store"),
        ],
        to_json(&config.status.snapshot()),
    )
}

const NA: &str = "n/a";

/// Colours for the sites' lines and parts in the charts (`--series-1` … in
/// style.css), in the order of `STATUS_SITES`. The NixOS module writes it in
/// alphabetical order, so a new site takes its place there and the sites
/// after it change colour. The sites past the last colour share a grey (`--series-other`): the
/// charts show them together as one, as more colours would be too alike to
/// tell apart. status.js has the same limit.
const SERIES_COLORS: usize = 8;

fn series_style(index: usize) -> String {
    if index < SERIES_COLORS {
        format!("--series: var(--series-{})", index + 1)
    } else {
        "--series: var(--series-other)".to_string()
    }
}

fn status_page(snapshot: &Snapshot) -> Markup {
    let host = &snapshot.host;
    let sites = &snapshot.sites;
    let load = host
        .load
        .map(|[one, five, fifteen]| format!("{one:.2} · {five:.2} · {fifteen:.2}"));
    html! {
        // No visible headings: the server's numbers, the charts, and under
        // them the sites as the charts' legend. The h1 is for screen readers.
        main.status {
            h1.visually-hidden { "Server status" }
            section.cpu-section {
                // The whole server's numbers as one row on top of the charts.
                // CPU takes two readings, so it's only there with JavaScript.
                dl.host #host {
                    div {
                        dt { "Load average" }
                        dd.load { span.value { (load.as_deref().unwrap_or(NA)) } }
                    }
                    div.live hidden {
                        dt { "CPU" }
                        dd.cpu { span.value { (NA) } }
                    }
                    div {
                        dt { "Memory in use" }
                        dd.memory {
                            span.value { (host.memory_used.map_or_else(|| NA.to_string(), format_bytes)) }
                        }
                    }
                    div {
                        dt { "Up" }
                        dd.up { span.value { (uptime(snapshot.uptime_cs)) } }
                    }
                }
                // The sites' CPU over the last 10 minutes as lines, and under
                // it their memory now as one bar, in the same colours and
                // order. Drawn by status.js, so hidden without JavaScript.
                div.charts.live hidden {
                    div.cpu-area #cpu-area {
                        div.plot {}
                        div.tip hidden {}
                    }
                    div.area-memory {
                        span.label { "Memory" }
                        div.membar.memory-chart {}
                        div.tip hidden {}
                    }
                }
            }
            @if sites.is_empty() {
                p.none { "No sites are configured." }
            } @else {
                (sites_legend(sites))
            }
        }
    }
}

fn uptime(cs: Option<u64>) -> String {
    cs.map_or_else(|| NA.to_string(), |cs| format_duration(cs / 100))
}

/// The sites as the charts' legend: each one's colour (the grey of the
/// others' past the first `SERIES_COLORS`) and name, wrapping onto more lines as needed, with
/// its uptime under the name, see `site_about`, as a tooltip would not show
/// on a phone. Rendered here, so it's there without JavaScript too; status.js
/// keeps it up to date, also when a site stops or starts while the page is
/// open. CPU and memory are in the charts' tooltips.
fn sites_legend(sites: &[SiteUsage]) -> Markup {
    html! {
        ul.sites-legend #sites aria-label="Sites" {
            @for (i, site) in sites.iter().enumerate() {
                li data-site=(site.name) style=(series_style(i)) {
                    span.swatch aria-hidden="true" {}
                    span.name { (site.name) }
                    @if let Some(about) = site_about(site) {
                        span.about { (about) }
                    } @else {
                        " "
                        span.down { "not running" }
                    }
                }
            }
        }
    }
}

/// What the legend tells of a running site under its name: its uptime, and
/// below it on a line of its own how many services its numbers add up when
/// there are more than one, "2 services", or "1 of 2 services running" when
/// some aren't. Only a count, so the page doesn't tell what runs on the
/// host. The lines are split by "\n", see .about in style.css. status.js
/// writes the same.
fn site_about(site: &SiteUsage) -> Option<String> {
    if !site.running {
        return None;
    }
    let up = format!("up {}", uptime(site.up_cs));
    Some(if site.services <= 1 {
        up
    } else if site.services_running == site.services {
        format!("{up}\n{} services", site.services)
    } else {
        format!(
            "{up}\n{} of {} services running",
            site.services_running, site.services
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(name: &str, units: &[&str]) -> Site {
        Site {
            name: name.to_string(),
            units: units.iter().map(ToString::to_string).collect(),
        }
    }

    #[test]
    fn parses_sites() {
        assert_eq!(
            parse_sites(" Blog = blog, blog-worker.service ;Shop=shop@1;"),
            Ok(vec![
                site("Blog", &["blog.service", "blog-worker.service"]),
                site("Shop", &["shop@1.service"]),
            ])
        );
        assert_eq!(parse_sites(""), Ok(vec![]));
    }

    #[test]
    fn rejects_bad_sites() {
        for spec in [
            "Blog",
            "=blog",
            "Blog=",
            "Blog=../etc",
            "Blog=a/b",
            "Blog=..",
        ] {
            assert!(parse_sites(spec).is_err(), "{spec:?} was accepted");
        }
    }

    #[test]
    fn parses_proc_files() {
        assert_eq!(parse_uptime("12345.67 54321.00\n"), Some(1_234_567));
        assert_eq!(parse_uptime("12.5 1.0"), Some(1250));
        assert_eq!(
            parse_loadavg("0.42 0.30 0.25 1/123 4567\n"),
            Some([0.42, 0.30, 0.25])
        );
        assert_eq!(
            parse_memory_used(
                "MemTotal:        2000 kB\nMemFree: 1 kB\nMemAvailable:    1000 kB\n"
            ),
            Some(1_024_000)
        );
        let stat = "cpu  10 1 5 100 4 1 1 0 3 0\ncpu0 5 0 2 50 2 0 1 0 0 0\ncpu1 5 1 3 50 2 1 0 0 3 0\nintr 1\n";
        assert_eq!(parse_stat(stat), (Some(18), Some(122)));
        assert_eq!(
            parse_cpu_usage("usage_usec 1234\nuser_usec 1000\n"),
            Some(1234)
        );
        let pid_stat =
            "42 (my (odd) name) S 1 42 42 0 -1 4194560 1 0 0 0 1 2 0 0 20 0 1 0 98765 1000";
        assert_eq!(parse_start_time(pid_stat), Some(98765));
    }

    #[test]
    fn formats_sizes_and_durations() {
        assert_eq!(format_bytes(161_600_000), "162 MB");
        assert_eq!(format_bytes(1_449_000_000), "1.4 GB");
        assert_eq!(format_bytes(12_600_000_000), "13 GB");
        assert_eq!(format_duration(45), "45 s");
        assert_eq!(format_duration(12 * 60 + 5), "12 min");
        assert_eq!(format_duration(3 * 3600 + 5 * 60), "3 h 5 min");
        assert_eq!(format_duration(86_400 + 7200), "1 day 2 h");
        assert_eq!(format_duration(12 * 86_400), "12 days");
    }

    /// A fake /proc and cgroup tree: one site running, one not.
    #[test]
    fn reads_cgroups_and_proc() {
        let root = std::env::temp_dir().join(format!("portfolio-status-{}", std::process::id()));
        let write = |path: &str, contents: &str| {
            let path = root.join(path);
            assert!(path
                .parent()
                .is_some_and(|dir| fs::create_dir_all(dir).is_ok()));
            assert!(fs::write(path, contents).is_ok());
        };
        write("proc/uptime", "1000.00 0.00");
        write("proc/loadavg", "0.10 0.20 0.30 1/2 3");
        write(
            "proc/7/stat",
            "7 (blog) S 1 7 7 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 40000 0",
        );
        write(
            "proc/8/stat",
            "8 (blog) S 1 8 8 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 60000 0",
        );
        write("cgroup/blog.service/memory.current", "1000000\n");
        write("cgroup/blog.service/cpu.stat", "usage_usec 3000000\n");
        write("cgroup/blog.service/cgroup.procs", "8\n7\n");
        write("cgroup/blog-worker.service/memory.current", "500000\n");
        write("cgroup/blog-worker.service/cgroup.procs", "9\n");
        write(
            "proc/9/stat",
            "9 (worker) S 1 9 9 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 30000 0",
        );
        write("cgroup/wiki-worker.service/cgroup.procs", "9\n");
        write(
            "cgroup/blog-worker.service/cpu.stat",
            "usage_usec 1000000\n",
        );

        let monitor = Monitor::with_roots(
            vec![
                site("Blog", &["blog.service", "blog-worker.service"]),
                site("Shop", &["shop.service"]),
                // Its first unit isn't running: the uptime is the second's
                site("Wiki", &["wiki.service", "wiki-worker.service"]),
            ],
            root.join("cgroup"),
            root.join("proc"),
        );
        let snapshot = monitor.snapshot();
        let _ = fs::remove_dir_all(&root);

        assert_eq!(snapshot.uptime_cs, Some(100_000));
        assert_eq!(snapshot.host.load, Some([0.1, 0.2, 0.3]));
        assert_eq!(snapshot.host.memory_used, None);
        assert_eq!(
            snapshot.sites,
            vec![
                SiteUsage {
                    name: "Blog".to_string(),
                    running: true,
                    services: 2,
                    services_running: 2,
                    memory: 1_500_000,
                    cpu_usec: 4_000_000,
                    // The oldest process, the worker's, started at 300 s
                    up_cs: Some(70_000),
                },
                SiteUsage {
                    name: "Shop".to_string(),
                    services: 1,
                    ..SiteUsage::default()
                },
                SiteUsage {
                    name: "Wiki".to_string(),
                    running: true,
                    services: 2,
                    services_running: 1,
                    up_cs: Some(70_000),
                    ..SiteUsage::default()
                },
            ]
        );
        assert!(any_running(&snapshot));
        // Cached: the files are gone, but the reading is still fresh.
        assert_eq!(monitor.snapshot(), snapshot);
    }

    #[test]
    fn json_has_every_field() {
        let snapshot = Snapshot {
            uptime_cs: Some(100),
            host: Host {
                load: Some([0.5, 0.25, 1.0]),
                ..Host::default()
            },
            sites: vec![SiteUsage {
                name: "A \"quoted\"\nname".to_string(),
                running: true,
                services: 2,
                services_running: 1,
                memory: 10,
                cpu_usec: 20,
                up_cs: None,
            }],
        };
        assert_eq!(
            to_json(&snapshot),
            "{\"uptime_cs\":100,\"host\":{\"load\":[0.5,0.25,1],\"memory_used\":null,\
             \"cpu_busy\":null,\"cpu_total\":null},\"sites\":[\
             {\"name\":\"A \\\"quoted\\\"\\nname\",\"running\":true,\"services\":2,\
             \"services_running\":1,\"memory\":10,\"cpu_usec\":20,\"up_cs\":null}]}"
        );
        assert!(!any_running(&Snapshot::default()));
    }

    #[test]
    fn legend_tells_how_many_services_a_site_adds_up() {
        let site = |services, services_running| SiteUsage {
            name: "Blog".to_string(),
            running: services_running > 0,
            services,
            services_running,
            up_cs: Some(3 * 86_400 * 100),
            ..SiteUsage::default()
        };
        assert_eq!(site_about(&site(1, 1)).as_deref(), Some("up 3 days"));
        assert_eq!(
            site_about(&site(2, 2)).as_deref(),
            Some("up 3 days\n2 services")
        );
        assert_eq!(
            site_about(&site(3, 1)).as_deref(),
            Some("up 3 days\n1 of 3 services running")
        );
        assert_eq!(site_about(&site(2, 0)), None);
    }

    /// The first 8 sites in their colours, the rest in the others' grey
    #[test]
    fn legend_colours_past_the_eighth_site_are_grey() {
        let sites: Vec<SiteUsage> = (1..=10)
            .map(|n| SiteUsage {
                name: format!("Site {n}"),
                ..SiteUsage::default()
            })
            .collect();
        let legend = sites_legend(&sites).into_string();
        for (n, colour) in [
            (1, "--series-1"),
            (8, "--series-8"),
            (9, "--series-other"),
            (10, "--series-other"),
        ] {
            assert!(
                legend.contains(&format!(
                    r#"<li data-site="Site {n}" style="--series: var({colour})"><span class="swatch" aria-hidden="true"></span>"#
                )),
                "{n}: {legend}"
            );
        }
    }
}
