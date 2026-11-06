// Live updates for /status and the front page's footer charts: polls
// /status.json, turns the kernel's cumulative counters into CPU percentages
// and draws sparklines of the last 10 minutes of the visit.
//
// The server keeps no history; this tab does, in sessionStorage, so the
// charts carry on across the site's pages. Without JavaScript the page shows
// the server-rendered reading.
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

  const setText = (element, text) => {
    const value = element?.querySelector(".value");
    if (value) value.textContent = text;
  };

  // Colours of the sites' lines and parts (--series-1 … in style.css), by
  // their order in /status.json, which is the sites table's order. Same limit as
  // status.rs.
  const SERIES_COLORS = 8;

  let previous = null; // the latest reading

  let lastPoll = 0; // when it was asked for, by this browser's clock

  // Adds a reading to the history: each site's memory, and CPU percentages
  // from the difference to the previous reading.
  function addReading(now) {
    // CPU time of the whole server (all cores) since the previous reading,
    // in ticks of a hundredth of a second, i.e. 10 000 µs. A site's share of
    // the server is its CPU microseconds against that.
    const busy = now.host.cpu_busy - previous?.host.cpu_busy;
    const ticks = now.host.cpu_total - previous?.host.cpu_total;
    if (!(ticks > 0 && busy >= 0)) return;
    record("cpu:host", (100 * busy) / ticks);
    // Every site gets a value with every reading, so the histories stay
    // aligned in time for the stacked chart: 0 while a site isn't running,
    // and for the interval in which it restarted (its counter starts over).
    for (const site of now.sites) {
      const before = previous?.sites.find((s) => s.name === site.name);
      const counted = site.running && before?.running && site.cpu_usec >= before.cpu_usec;
      record(`cpu:${site.name}`, counted ? (100 * (site.cpu_usec - before.cpu_usec)) / (ticks * 1e4) : 0);
    }
  }

  // Shows the latest reading and the history on whatever this page has: the
  // list and the sites on /status, the footer charts on the front page.
  function render(now) {
    const latest = (key) => history.get(key)?.at(-1);
    const host = document.getElementById("host"); // only on /status
    const { load, memory_used: used } = now.host;
    if (host && load) setText(host.querySelector(".load"), load.map((l) => l.toFixed(2)).join(" · "));
    if (host && now.uptime_cs != null) setText(host.querySelector(".up"), formatDuration(now.uptime_cs / 100));
    if (used != null) {
      if (host) setText(host.querySelector(".memory"), formatBytes(used));
      // The footer's bar has the rest of the server; /status's only the sites.
      for (const bar of document.querySelectorAll(".memory-chart")) {
        memoryBar(bar, now.sites, used, !bar.closest(".cpu-section"));
      }
    }
    const whole = history.get("cpu:host");
    if (whole?.length) {
      if (host) setText(host.querySelector(".cpu"), formatPercent(latest("cpu:host")));
      combined(".cpu-chart", "cpu", now.sites, whole, Math.max(5, ...whole)); // the footer's
    }
    const area = document.getElementById("cpu-area");
    if (area) cpuArea(area, now.sites);

    // The sites at the end of /status, which are also the charts' legend. A
    // row the server rendered as "not running" keeps that until a reload.
    for (const row of document.querySelectorAll("#sites tr[data-site]")) {
      const site = now.sites.find((s) => s.name === row.dataset.site);
      if (!site?.running || !row.querySelector(".memory")) continue;
      const cpu = latest(`cpu:${site.name}`);
      setText(row.querySelector(".cpu"), cpu == null ? "…" : formatPercent(cpu));
      setText(row.querySelector(".memory"), formatBytes(site.memory));
      if (site.up_cs != null) setText(row.querySelector(".up"), formatDuration(site.up_cs / 100));
    }
  }


  // The big CPU chart on /status: the sites' last 10 minutes as stacked
  // areas, each site a band in its colour from the bottom, on a scale that
  // fits them (the memory bar under it has the same colours and order).
  // Drawn at the element's real size (redrawn when it changes), so the axis
  // text keeps its shape. Hovering or touching shows every site's value at
  // that time, and the whole server's.
  const AREA = { height: 170, left: 44, right: 8, top: 10, bottom: 22 };
  let areaSites = []; // for redrawing on resize
  let areaPointer = null; // where the pointer is, to keep the tooltip on updates

  const svgElement = (name, attributes = {}) => {
    const element = document.createElementNS(SVG, name);
    for (const [key, value] of Object.entries(attributes)) element.setAttribute(key, value);
    return element;
  };
  // The top of the scale: the smallest of these that fits, at least 5 %.
  const niceMax = (value) => [5, 10, 20, 25, 50, 100].find((step) => value <= step) ?? 100;
  const axisPercent = (value) => `${Number.isInteger(value) ? value : value.toFixed(1)} %`;

  function cpuArea(container, sites) {
    areaSites = sites;
    const plot = container.querySelector(".plot");
    const total = history.get("cpu:host") ?? [];
    const n = total.length;
    // The last n values, so every band lines up with the total.
    const align = (values) => [...Array(Math.max(0, n - values.length)).fill(0), ...values.slice(-n)];
    const bands = sites.slice(0, SERIES_COLORS).map((site, i) => ({
      name: site.name,
      color: `var(--series-${i + 1})`,
      values: align(history.get(`cpu:${site.name}`) ?? []),
    }));

    const width = Math.max(200, container.clientWidth);
    const { height, left, right, top, bottom } = AREA;
    const plotWidth = width - left - right;
    const plotHeight = height - top - bottom;
    // Only the sites are stacked, and the scale fits them.
    const tops = total.map((_, k) => bands.reduce((sum, band) => sum + band.values[k], 0));
    const max = niceMax(Math.max(0, ...tops));
    const x = (k) => left + ((POINTS - n + k) / (POINTS - 1)) * plotWidth; // newest on the right
    const y = (value) => top + plotHeight - (Math.min(value, max) / max) * plotHeight;

    const svg = svgElement("svg", { viewBox: `0 0 ${width} ${height}`, width, height, role: "img" });
    const latestOf = (values) => values.at(-1) ?? 0;
    svg.setAttribute(
      "aria-label",
      n === 0
        ? "CPU over the last 10 minutes: no readings yet"
        : `CPU over the last 10 minutes, now: whole server ${formatPercent(latestOf(total))}, ` +
            bands.map((band) => `${band.name} ${formatPercent(latestOf(band.values))}`).join(", "),
    );

    // Grid and axes: 0, half and the top of the scale; time along the bottom.
    for (const value of [0, max / 2, max]) {
      svg.append(svgElement("line", { class: "grid", x1: left, x2: width - right, y1: y(value), y2: y(value) }));
      const label = svgElement("text", { class: "axis", x: left - 6, y: y(value) + 4, "text-anchor": "end" });
      label.textContent = axisPercent(value);
      svg.append(label);
    }
    for (const [text, at, anchor] of [["10 min ago", left, "start"], ["5 min", left + plotWidth / 2, "middle"], ["now", width - right, "end"]]) {
      const label = svgElement("text", { class: "axis", x: at, y: height - 6, "text-anchor": anchor });
      label.textContent = text;
      svg.append(label);
    }

    if (n >= 2) {
      // Bands from the bottom up, each from the top of the ones below it.
      let lower = Array(n).fill(0);
      for (const band of bands) {
        const upper = lower.map((value, k) => value + band.values[k]);
        const edge = upper.map((value, k) => `${x(k).toFixed(1)},${y(value).toFixed(1)}`);
        const base = lower.map((value, k) => `${x(k).toFixed(1)},${y(value).toFixed(1)}`).reverse();
        const path = svgElement("path", { class: `band ${band.className ?? ""}`, d: `M${edge.join("L")}L${base.join("L")}Z` });
        if (band.color) path.style.fill = band.color;
        svg.append(path);
        lower = upper;
      }
    }

    // Hovering: a vertical line at the nearest reading, and the tooltip.
    const cursor = svgElement("line", { class: "cursor", y1: top, y2: top + plotHeight, visibility: "hidden" });
    const hit = svgElement("rect", { class: "hit", x: left, y: top, width: plotWidth, height: plotHeight });
    svg.append(cursor, hit);
    plot.replaceChildren(svg);

    const tip = container.querySelector(".tip");
    const show = (clientX) => {
      if (n < 2) return;
      const box = svg.getBoundingClientRect();
      const slot = Math.round(((clientX - box.left - left) / plotWidth) * (POINTS - 1));
      const k = Math.min(n - 1, Math.max(0, slot - (POINTS - n)));
      cursor.setAttribute("x1", x(k));
      cursor.setAttribute("x2", x(k));
      cursor.setAttribute("visibility", "visible");
      const seconds = ((n - 1 - k) * INTERVAL) / 1000;
      const row = (label, value, color, className) => {
        const line = document.createElement("div");
        const swatch = document.createElement("span");
        swatch.className = `swatch ${className ?? ""}`;
        if (color) swatch.style.background = color;
        const name = document.createElement("span");
        name.textContent = `${label} `;
        const number = document.createElement("b");
        number.textContent = formatPercent(value);
        if (color || className) line.append(swatch);
        line.append(name, number);
        return line;
      };
      const when = document.createElement("div");
      when.className = "when";
      when.textContent = k === n - 1 ? "now" : `${formatDuration(seconds)} ago`;
      tip.replaceChildren(
        when,
        row("whole server", total[k]),
        ...bands.map((band) => row(band.name, band.values[k], band.color, band.color ? "" : "total")),
      );
      tip.hidden = false;
      // Beside the line, flipped to its left near the right edge.
      const at = x(k);
      const flip = at + 12 + tip.offsetWidth > width;
      tip.style.left = `${flip ? at - 12 - tip.offsetWidth : at + 12}px`;
      areaPointer = clientX;
    };
    hit.addEventListener("pointermove", (event) => show(event.clientX));
    hit.addEventListener("pointerdown", (event) => show(event.clientX));
    hit.addEventListener("pointerleave", () => {
      tip.hidden = true;
      cursor.setAttribute("visibility", "hidden");
      areaPointer = null;
    });
    if (areaPointer != null) show(areaPointer); // keep the tooltip through updates
  }

  // The history is kept for this tab in sessionStorage, so the charts carry
  // on from page to page of the site instead of starting over. Older than
  // the charts' 10 minutes, it's dropped. Storage can throw (blocked storage,
  // some private modes); then every page starts over, as before.
  const STORE = "status-history";
  function save() {
    try {
      sessionStorage.setItem(STORE, JSON.stringify({ at: lastPoll, previous, history: [...history] }));
    } catch {}
  }
  function restore() {
    try {
      const saved = JSON.parse(sessionStorage.getItem(STORE));
      if (!saved?.previous || Date.now() - saved.at > POINTS * INTERVAL) return;
      // Only CPU has history now; skip what older versions of this script kept.
      for (const [key, values] of saved.history) if (key.startsWith("cpu:")) history.set(key, values);
      previous = saved.previous;
      lastPoll = saved.at;
    } catch {}
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

  // Memory in use that isn't a site with a colour (sites past the last colour
  // included). Sites count their file cache, which "used" leaves out, so this
  // can't go below zero.
  function restOfServer(sites, used) {
    const inSites = sites
      .slice(0, SERIES_COLORS)
      .reduce((sum, site) => sum + (site.running ? site.memory : 0), 0);
    return Math.max(0, used - inSites);
  }

  // Memory as one bar: every running site in its colour, with widths in
  // proportion, and with the rest of the memory in use in grey (the front
  // page's footer) or without (/status, where it sits under the sites' CPU).
  // Memory hardly moves within minutes, so this shows it better than lines
  // over time. Sites past the last colour count as the rest. Hovering a part
  // gives its numbers; the bar's accessible name lists them all.
  function memoryBar(bar, sites, used, withRest) {
    const parts = sites
      .slice(0, SERIES_COLORS)
      .map((site, i) => ({ name: site.name, bytes: site.running ? site.memory : 0, color: `var(--series-${i + 1})` }));
    if (withRest) parts.push({ name: "rest of the server", bytes: restOfServer(sites, used), className: "other" });
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
    bar.setAttribute("aria-label", `${withRest ? "Memory in use" : "Memory of the sites"}: ${shown.map(describe).join(", ")}`);
  }

  async function poll() {
    // Nobody is looking, or the last reading is recent (e.g. from the
    // previous page): keep to one reading per interval, so the charts' time
    // runs evenly however fast someone clicks around.
    if (document.hidden || Date.now() - lastPoll < INTERVAL - 1000) return;
    lastPoll = Date.now();
    let now;
    try {
      const response = await fetch("/status.json", { cache: "no-store" });
      if (!response.ok) return;
      now = await response.json();
    } catch {
      return; // offline or restarting: try again next time
    }
    if (now.uptime_cs == null || now.uptime_cs === previous?.uptime_cs) return;
    addReading(now);
    previous = now;
    save();
    render(now);
  }

  // The server loads this script on /status and, while a site is running, on
  // the other pages too: the front page draws its footer charts, and the
  // resume only keeps the history going for the next page.
  document.addEventListener("DOMContentLoaded", () => {
    for (const element of document.querySelectorAll(".hero .live, .status .live, .footer-charts")) element.hidden = false;
    // CPU percentages need two readings.
    for (const cell of document.querySelectorAll(".status .cpu .value")) cell.textContent = "…";
    restore();
    if (previous) render(previous); // carry on where the last page left off
    const area = document.getElementById("cpu-area");
    if (area && window.ResizeObserver) {
      let width = area.clientWidth;
      new ResizeObserver(() => {
        if (area.clientWidth === width) return;
        width = area.clientWidth;
        cpuArea(area, areaSites);
      }).observe(area);
    }
    // The first reading when this interval is up, then one every interval.
    setTimeout(() => {
      poll();
      setInterval(poll, INTERVAL);
    }, Math.max(0, INTERVAL - (Date.now() - lastPoll)));
    document.addEventListener("visibilitychange", poll);
  });
})();
