// Live updates for /status: polls /status.json, turns the kernel's
// cumulative counters into CPU percentages and draws sparklines of the time
// the page has been open (up to 10 minutes).
//
// The history lives only in this page: the server keeps none, and reloading
// starts over. Without JavaScript the page shows the server-rendered reading.
(() => {
  const INTERVAL = 5000; // ms; the server reuses a reading for 2 s
  const POINTS = 120; // 10 minutes of readings

  // Same formats as status.rs, so the numbers don't change style on update.
  const formatBytes = (bytes) => {
    const GB = 1e9;
    if (bytes >= 10 * GB) return `${Math.round(bytes / GB)} GB`;
    if (bytes >= GB) return `${(Math.round(bytes / (GB / 10)) / 10).toFixed(1)} GB`;
    return `${Math.round(bytes / 1e6)} MB`;
  };
  const formatDuration = (seconds) => {
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor(seconds / 3600) % 24;
    const minutes = Math.floor(seconds / 60) % 60;
    if (days >= 2) return `${days} days`;
    if (days === 1) return `1 day ${hours} h`;
    if (hours > 0) return `${hours} h ${minutes} min`;
    if (minutes > 0) return `${minutes} min`;
    return `${Math.floor(seconds)} s`;
  };
  const formatPercent = (p) => (p < 10 ? `${p.toFixed(1)} %` : `${Math.round(p)} %`);

  // Recent values by key, oldest first.
  const history = new Map();
  const record = (key, value) => {
    const values = history.get(key) ?? [];
    values.push(value);
    if (values.length > POINTS) values.shift();
    history.set(key, values);
    return values;
  };

  // Lines over the last POINTS readings on one scale, filling the element's
  // width; the first line is drawn first, so later ones are on top. Each line
  // is { values, stroke? (CSS colour), className? }. Decorative: the numbers
  // next to it say the same in words.
  const SVG = "http://www.w3.org/2000/svg";
  function sparklines(element, lines, min, max) {
    if (!element) return;
    let svg = element.querySelector("svg");
    if (!svg) {
      svg = document.createElementNS(SVG, "svg");
      svg.setAttribute("viewBox", `0 0 ${POINTS - 1} 20`);
      svg.setAttribute("preserveAspectRatio", "none");
      svg.setAttribute("aria-hidden", "true");
      element.append(svg);
    }
    while (svg.children.length < lines.length) svg.append(document.createElementNS(SVG, "polyline"));
    while (svg.children.length > lines.length) svg.lastChild.remove();
    const range = max - min || 1;
    lines.forEach(({ values, stroke, className }, n) => {
      const start = POINTS - values.length; // new readings enter from the right
      const points = values.map((v, i) => {
        const y = 19 - (18 * (Math.min(Math.max(v, min), max) - min)) / range;
        return `${start + i},${y.toFixed(2)}`;
      });
      const line = svg.children[n];
      line.setAttribute("points", points.join(" "));
      if (className) line.setAttribute("class", className);
      if (stroke) line.style.stroke = stroke;
    });
  }
  const sparkline = (element, values, min, max) => sparklines(element, [{ values }], min, max);

  const setText = (element, text) => {
    const value = element?.querySelector(".value");
    if (value) value.textContent = text;
  };

  // Memory moves little, so its line spans its own range instead of 0–max.
  const memoryLine = (element, values) => {
    const low = Math.min(...values);
    const high = Math.max(...values);
    const pad = Math.max((high - low) * 0.1, 1e6);
    sparkline(element, values, Math.max(0, low - pad), high + pad);
  };
  // CPU from 0, at least 0–5 %, so idle noise stays a flat line.
  const cpuLine = (element, values) => sparkline(element, values, 0, Math.max(5, ...values));

  let previous = null;

  function update(now) {
    // Elapsed time by the host's own clock, in microseconds.
    const elapsed = previous && (now.uptime_cs - previous.uptime_cs) * 1e4;
    const cpus = now.host.cpus || 1;

    for (const site of now.sites) {
      const row = document.querySelector(`#sites tr[data-site="${CSS.escape(site.name)}"]`);
      if (!row || !site.running || !row.querySelector(".memory")) continue;
      const memory = row.querySelector(".memory");
      setText(memory, formatBytes(site.memory));
      memoryLine(memory.querySelector(".spark"), record(`memory:${site.name}`, site.memory));
      if (site.up_cs != null) setText(row.querySelector(".up"), formatDuration(site.up_cs / 100));

      const before = previous?.sites.find((s) => s.name === site.name);
      // A restarted service starts counting from zero: skip that interval.
      if (elapsed > 0 && before?.running && site.cpu_usec >= before.cpu_usec) {
        const percent = (100 * (site.cpu_usec - before.cpu_usec)) / (elapsed * cpus);
        const cpu = row.querySelector(".cpu");
        setText(cpu, formatPercent(percent));
        cpuLine(cpu.querySelector(".spark"), record(`cpu:${site.name}`, percent));
      }
    }

    const host = document.getElementById("host");
    const { load, memory_total: total, memory_available: available } = now.host;
    if (load) {
      const cpusText = now.host.cpus ? ` (${now.host.cpus} CPUs)` : "";
      setText(host.querySelector(".load"), `${load.map((l) => l.toFixed(2)).join(" · ")}${cpusText}`);
    }
    if (total != null && available != null) {
      const memory = host.querySelector(".memory");
      setText(memory, `${formatBytes(total - available)} of ${formatBytes(total)}`);
      memoryLine(memory.querySelector(".spark"), record("memory:host", total - available));
    }
    if (now.uptime_cs != null) setText(host.querySelector(".up"), formatDuration(now.uptime_cs / 100));
    const busy = now.host.cpu_busy - previous?.host.cpu_busy;
    const ticks = now.host.cpu_total - previous?.host.cpu_total;
    if (ticks > 0 && busy >= 0) {
      const percent = (100 * busy) / ticks;
      const cpu = host.querySelector(".cpu");
      setText(cpu, formatPercent(percent));
      // The whole server and, on the same scale, every site in its colour
      // from the table (data-series). Sites are part of the whole, so they
      // stay under its line.
      const total = record("cpu:host", percent);
      const sites = [...document.querySelectorAll("#sites tr[data-series]")]
        .map((row) => ({
          values: history.get(`cpu:${row.dataset.site}`) ?? [],
          stroke: `var(--series-${row.dataset.series})`,
        }))
        .filter((line) => line.values.length > 0);
      sparklines(cpu.querySelector(".spark"), [{ values: total, className: "total" }, ...sites], 0,
        Math.max(5, ...total));
    }
  }

  async function poll() {
    if (document.hidden) return; // nobody is looking
    let now;
    try {
      const response = await fetch("/status.json", { cache: "no-store" });
      if (!response.ok) return;
      now = await response.json();
    } catch {
      return; // offline or restarting: try again next time
    }
    if (now.uptime_cs == null || now.uptime_cs === previous?.uptime_cs) return;
    update(now);
    previous = now;
  }

  document.addEventListener("DOMContentLoaded", () => {
    if (!document.getElementById("host")) return;
    for (const element of document.querySelectorAll(".status .live")) element.hidden = false;
    // Percentages instead of total CPU time, from the second reading on.
    const header = document.querySelector("#sites th.cpu");
    if (header) header.textContent = "CPU";
    for (const cell of document.querySelectorAll(".status .cpu .value")) cell.textContent = "…";
    poll();
    setInterval(poll, INTERVAL);
    document.addEventListener("visibilitychange", poll);
  });
})();
