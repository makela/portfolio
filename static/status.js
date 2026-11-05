// Live updates for /status and the front page's footer charts: polls
// /status.json, turns the kernel's cumulative counters into CPU percentages
// and draws sparklines of the time the page has been open (up to 10 minutes).
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

  // Colours of the sites' lines (--series-1 … in style.css), by their order
  // in /status.json, which is the table's order. Same limit as status.rs.
  const SERIES_COLORS = 8;

  function update(now) {
    // Elapsed time by the host's own clock, in microseconds.
    const elapsed = previous && (now.uptime_cs - previous.uptime_cs) * 1e4;
    const cpus = now.host.cpus || 1;

    for (const site of now.sites) {
      const row = document.querySelector(`#sites tr[data-site="${CSS.escape(site.name)}"]`);
      const shown = site.running && row?.querySelector(".memory");
      if (site.running) {
        const values = record(`memory:${site.name}`, site.memory);
        if (shown) {
          const memory = row.querySelector(".memory");
          setText(memory, formatBytes(site.memory));
          memoryLine(memory.querySelector(".spark"), values);
          if (site.up_cs != null) setText(row.querySelector(".up"), formatDuration(site.up_cs / 100));
        }
      }

      const before = previous?.sites.find((s) => s.name === site.name);
      // A restarted service starts counting from zero: skip that interval.
      if (elapsed > 0 && site.running && before?.running && site.cpu_usec >= before.cpu_usec) {
        const percent = (100 * (site.cpu_usec - before.cpu_usec)) / (elapsed * cpus);
        const values = record(`cpu:${site.name}`, percent);
        if (shown) {
          const cpu = row.querySelector(".cpu");
          setText(cpu, formatPercent(percent));
          cpuLine(cpu.querySelector(".spark"), values);
        }
      }
    }

    const host = document.getElementById("host"); // only on /status
    const { load, memory_total: total, memory_available: available } = now.host;
    if (host && load) {
      const cpusText = now.host.cpus ? ` (${now.host.cpus} CPUs)` : "";
      setText(host.querySelector(".load"), `${load.map((l) => l.toFixed(2)).join(" · ")}${cpusText}`);
    }
    if (total != null && available != null) {
      if (host) setText(host.querySelector(".memory"), `${formatBytes(total - available)} of ${formatBytes(total)}`);
      for (const bar of document.querySelectorAll(".memory-chart")) {
        memoryBar(bar, now.sites, total - available, total);
      }
    }
    if (host && now.uptime_cs != null) setText(host.querySelector(".up"), formatDuration(now.uptime_cs / 100));

    const busy = now.host.cpu_busy - previous?.host.cpu_busy;
    const ticks = now.host.cpu_total - previous?.host.cpu_total;
    if (ticks > 0 && busy >= 0) {
      const percent = (100 * busy) / ticks;
      if (host) setText(host.querySelector(".cpu"), formatPercent(percent));
      const whole = record("cpu:host", percent);
      combined(".cpu-chart", "cpu", now.sites, whole, Math.max(5, ...whole));
    }
  }

  // The whole server's line and, on the same scale from zero, every site's in
  // its colour, in each `selector` element (on /status and in the front page's footer).
  // Sites are part of the whole, so they stay under its line.
  function combined(selector, measure, sites, whole, max) {
    const lines = sites
      .slice(0, SERIES_COLORS)
      .map((site, i) => ({ values: history.get(`${measure}:${site.name}`) ?? [], stroke: `var(--series-${i + 1})` }))
      .filter((line) => line.values.length > 0);
    for (const chart of document.querySelectorAll(selector)) {
      sparklines(chart, [{ values: whole, className: "total" }, ...lines], 0, max);
    }
  }

  // The server's memory as one bar: every running site in its colour, the
  // rest of the used memory, and free memory, with widths in proportion.
  // Memory hardly moves within minutes and the sites are a small part of
  // it, so this shows it better than lines over time. Sites past the last
  // colour are counted in the rest. Hovering a part gives its numbers; the
  // bar's accessible name lists them all.
  function memoryBar(bar, sites, used, total) {
    const parts = sites
      .slice(0, SERIES_COLORS)
      .map((site, i) => ({ name: site.name, bytes: site.running ? site.memory : 0, color: `var(--series-${i + 1})` }));
    const inSites = parts.reduce((sum, part) => sum + part.bytes, 0);
    parts.push(
      // Sites count their file cache, which "used" leaves out, so this can't go below zero.
      { name: "rest of the server", bytes: Math.max(0, used - inSites), className: "other" },
      { name: "free", bytes: Math.max(0, total - used), className: "free" },
    );
    const shown = parts.filter((part) => part.bytes > 0); // a part at zero draws nothing
    const all = shown.reduce((sum, part) => sum + part.bytes, 0) || 1;
    const describe = (part) => `${part.name} ${formatBytes(part.bytes)} (${Math.round((100 * part.bytes) / all)} %)`;

    while (bar.children.length < shown.length) bar.append(document.createElement("span"));
    while (bar.children.length > shown.length) bar.lastChild.remove();
    shown.forEach((part, i) => {
      const segment = bar.children[i];
      segment.className = part.className ?? "";
      segment.style.background = part.color ?? "";
      segment.style.flex = `${part.bytes} 1 0`;
      segment.title = describe(part);
    });
    bar.setAttribute("role", "img");
    bar.setAttribute("aria-label", `Memory: ${shown.map(describe).join(", ")}`);
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
    // /status, or the front page's footer charts.
    if (!document.getElementById("host") && !document.querySelector(".cpu-chart")) return;
    for (const element of document.querySelectorAll(".hero .live, .status .live, .footer-charts")) element.hidden = false;
    // Percentages instead of total CPU time, from the second reading on.
    const header = document.querySelector("#sites th.cpu");
    if (header) header.textContent = "CPU";
    for (const cell of document.querySelectorAll(".status .cpu .value")) cell.textContent = "…";
    poll();
    setInterval(poll, INTERVAL);
    document.addEventListener("visibilitychange", poll);
  });
})();
