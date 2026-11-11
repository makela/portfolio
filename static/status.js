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
  // Services written in Rust may use a few hundredths of a percent, so small
  // values keep their first digits: "0.03 %", not "0.0 %".
  const formatPercent = (p) => {
    if (p >= 10) return `${Math.round(p)} %`;
    if (p >= 0.1) return `${p.toFixed(1)} %`;
    return `${Number(p.toFixed(3))} %`;
  };

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
    // aligned in time for the chart: 0 while a site isn't running,
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
    if (host && used != null) setText(host.querySelector(".memory"), formatBytes(used));
    for (const bar of document.querySelectorAll(".memory-chart")) memoryBar(bar, now.sites);
    const whole = history.get("cpu:host");
    if (whole?.length) {
      if (host) setText(host.querySelector(".cpu"), formatPercent(latest("cpu:host")));
      footerCpu(now.sites);
    }
    const area = document.getElementById("cpu-area");
    if (area) cpuArea(area, now.sites);

    for (const item of document.querySelectorAll("#sites [data-site]")) {
      const site = now.sites.find((s) => s.name === item.dataset.site);
      if (site) legendItem(item, site);
    }
  }

  // A site in the legend under the charts on /status, as sites_legend in
  // status.rs renders it: a running site's uptime and service count under
  // its name, or "not running" beside it, so it follows a site stopping or
  // starting while the page is open.
  function legendItem(item, site) {
    const wanted = site.running ? "about" : "down";
    const text = site.running ? siteAbout(site) : "not running";
    let span = item.querySelector(".about, .down");
    if (span?.className !== wanted) {
      const name = item.querySelector(".name");
      while (item.lastChild && item.lastChild !== name) item.lastChild.remove();
      if (!site.running) item.append(" ");
      span = document.createElement("span");
      span.className = wanted;
      item.append(span);
    }
    if (span.textContent !== text) span.textContent = text;
  }

  // A running site's uptime, and above it how many services its numbers add
  // up when there are more than one: "2 services\nup 3 days", or "1 of 2
  // services running\nup 3 days". The CSS shows the "\n" as a line break.
  // Like site_about in status.rs.
  function siteAbout(site) {
    const up = `up ${site.up_cs == null ? "n/a" : formatDuration(site.up_cs / 100)}`;
    if (!(site.services > 1)) return up;
    if (site.services_running === site.services) return `${site.services} services\n${up}`;
    return `${site.services_running} of ${site.services} services running\n${up}`;
  }

  // The big CPU chart on /status: the sites' last 10 minutes as lines,
  // each site in its colour, on a scale that fits them (the memory bar
  // under it has the same colours and order).
  // Drawn at the element's real size (redrawn when it changes), so the axis
  // text keeps its shape; as tall as the page leaves room for, see .cpu-area
  // in style.css, and AREA.height at least. Hovering or touching shows every
  // site's value at that time, and the whole server's.
  const AREA = { height: 170, left: 44, right: 8, top: 10, bottom: 22 };
  let areaSites = []; // for redrawing on resize
  let areaPointer = null; // where the pointer is, to keep the tooltip on updates
  let memoryPointer = null; // the same for the memory bar

  // A line of a tooltip: the colour's square (if any), a name and a number.
  const tipRow = (label, number, color) => {
    const line = document.createElement("div");
    if (color) {
      const swatch = document.createElement("span");
      swatch.className = "swatch";
      swatch.style.background = color;
      line.append(swatch);
    }
    const name = document.createElement("span");
    name.textContent = `${label} `;
    const value = document.createElement("b");
    value.textContent = number;
    line.append(name, value);
    return line;
  };
  const tipHeading = (text) => {
    const heading = document.createElement("div");
    heading.className = "when";
    heading.textContent = text;
    return heading;
  };
  // Beside `at` (px from the container's left), flipped to its left near the
  // right edge.
  const placeTip = (tip, at, width) => {
    const flip = at + 12 + tip.offsetWidth > width;
    tip.style.left = `${Math.max(0, flip ? at - 12 - tip.offsetWidth : at + 12)}px`;
  };

  const svgElement = (name, attributes = {}) => {
    const element = document.createElementNS(SVG, name);
    for (const [key, value] of Object.entries(attributes)) element.setAttribute(key, value);
    return element;
  };
  // The top of the scale: the smallest of these that fits, so a nearly idle
  // server still shows its shape, down to a hundredth of a percent.
  const niceMax = (value) =>
    [0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1, 2, 5, 10, 20, 25, 50, 100].find((step) => value <= step) ?? 100;
  // Three decimals for the middle of the smallest scale, 0.005 %.
  const axisPercent = (value) => `${Number(value.toFixed(3))} %`;

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
    const height = Math.max(AREA.height, container.clientHeight);
    const { left, right, top, bottom } = AREA;
    const plotWidth = width - left - right;
    const plotHeight = height - top - bottom;
    // The scale fits the sites' lines.
    const max = niceMax(Math.max(0, ...bands.flatMap((band) => band.values)));
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

    // One line per site, in its colour; later sites on top.
    const lines = n < 2 ? [] : bands.map((band) => {
      const points = band.values.map((value, k) => `${x(k).toFixed(1)},${y(value).toFixed(1)}`);
      const line = svgElement("polyline", { class: "series", points: points.join(" ") });
      line.style.stroke = band.color;
      svg.append(line);
      return line;
    });

    // Hovering: a vertical line at the nearest reading, and the tooltip.
    const cursor = svgElement("line", { class: "cursor", y1: top, y2: top + plotHeight, visibility: "hidden" });
    const hit = svgElement("rect", { class: "hit", x: left, y: top, width: plotWidth, height: plotHeight });
    svg.append(cursor, hit);
    plot.replaceChildren(svg);

    const tip = container.querySelector(".tip");
    // The site whose line is nearest the pointer at reading k is drawn on
    // top and thicker, the others faded, and its row in the tooltip stands
    // out. Of lines that meet (e.g. idle sites at 0), the one drawn on top.
    let focused = -1;
    const focus = (k, pointerY) => {
      let nearest = -1;
      bands.forEach((band, i) => {
        if (nearest < 0 || Math.abs(y(band.values[k]) - pointerY) <= Math.abs(y(bands[nearest].values[k]) - pointerY)) nearest = i;
      });
      if (nearest === focused) return;
      lines[focused]?.classList.remove("on");
      focused = nearest;
      const line = lines[focused];
      if (!line) return;
      line.classList.add("on");
      svg.insertBefore(line, cursor); // above the other lines
      svg.classList.add("focus");
    };
    const unfocus = () => {
      lines[focused]?.classList.remove("on");
      focused = -1;
      svg.classList.remove("focus");
    };

    const show = ({ clientX, clientY }) => {
      if (n < 2) return;
      const box = svg.getBoundingClientRect();
      const slot = Math.round(((clientX - box.left - left) / plotWidth) * (POINTS - 1));
      const k = Math.min(n - 1, Math.max(0, slot - (POINTS - n)));
      cursor.setAttribute("x1", x(k));
      cursor.setAttribute("x2", x(k));
      cursor.setAttribute("visibility", "visible");
      focus(k, clientY - box.top);
      const seconds = ((n - 1 - k) * INTERVAL) / 1000;
      tip.replaceChildren(
        tipHeading(k === n - 1 ? "now" : `${formatDuration(seconds)} ago`),
        tipRow("whole server", formatPercent(total[k])),
        ...bands.map((band, i) => {
          const row = tipRow(band.name, formatPercent(band.values[k]), band.color);
          if (i === focused) row.className = "on";
          return row;
        }),
      );
      tip.hidden = false;
      placeTip(tip, x(k), width);
      areaPointer = { clientX, clientY };
    };
    hit.addEventListener("pointermove", show);
    hit.addEventListener("pointerdown", show);
    hit.addEventListener("pointerleave", () => {
      tip.hidden = true;
      cursor.setAttribute("visibility", "hidden");
      unfocus();
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

  // The footer's CPU sparkline: every site's line in its colour, on the same
  // scale as the chart on /status.
  function footerCpu(sites) {
    const lines = sites
      .slice(0, SERIES_COLORS)
      .map((site, i) => ({ values: history.get(`cpu:${site.name}`) ?? [], stroke: `var(--series-${i + 1})` }))
      .filter((line) => line.values.length > 0);
    const max = niceMax(Math.max(0, ...lines.flatMap((line) => line.values)));
    for (const chart of document.querySelectorAll(".cpu-chart")) sparklines(chart, lines, 0, max);
  }

  // The sites' memory as one bar: every running site in its colour, with
  // widths in proportion (on /status and in the front page's footer).
  // Memory hardly moves within minutes, so this shows it better than lines
  // over time. Sites past the last colour are left out. The bar's accessible
  // name lists them all. On /status, hovering or touching it shows the same
  // tooltip as the CPU chart, with the sites not under the pointer dimmed;
  // in the footer, which is a link, each part has its numbers as a title.
  function memoryBar(bar, sites) {
    const parts = sites
      .slice(0, SERIES_COLORS)
      .map((site, i) => ({ name: site.name, bytes: site.running ? site.memory : 0, color: `var(--series-${i + 1})` }));
    const shown = parts.filter((part) => part.bytes > 0); // a part at zero draws nothing
    const all = shown.reduce((sum, part) => sum + part.bytes, 0);
    const share = (part) => `${formatBytes(part.bytes)} (${Math.round((100 * part.bytes) / (all || 1))} %)`;
    const describe = (part) => `${part.name} ${share(part)}`;
    const tip = bar.parentElement.querySelector(".tip");

    while (bar.children.length < shown.length) bar.append(document.createElement("span"));
    while (bar.children.length > shown.length) bar.lastChild.remove();
    shown.forEach((part, i) => {
      const segment = bar.children[i];
      segment.style.background = part.color;
      segment.style.flex = `${part.bytes} 1 0`;
      if (!tip) segment.title = describe(part);
    });
    bar.setAttribute("role", "img");
    bar.setAttribute("aria-label", `Memory of the sites: ${shown.map(describe).join(", ")}`);
    if (!tip) return;

    const container = bar.parentElement;
    const show = (clientX) => {
      if (!shown.length) return;
      // The part under the pointer, or the nearest across a gap.
      const segments = [...bar.children];
      let on = segments.findIndex((segment) => clientX < segment.getBoundingClientRect().right);
      if (on < 0) on = segments.length - 1;
      tip.replaceChildren(
        tipHeading("now"),
        tipRow("all sites", formatBytes(all)),
        ...shown.map((part, i) => {
          const line = tipRow(part.name, share(part), part.color);
          if (i === on) line.className = "on";
          return line;
        }),
      );
      tip.hidden = false;
      const box = container.getBoundingClientRect();
      placeTip(tip, clientX - box.left, container.clientWidth);
      memoryPointer = clientX;
    };
    // Once per bar; the handlers call the latest `show`.
    bar.showTip = show;
    if (!bar.dataset.tip) {
      bar.dataset.tip = "on";
      bar.addEventListener("pointermove", (event) => bar.showTip(event.clientX));
      bar.addEventListener("pointerdown", (event) => bar.showTip(event.clientX));
      bar.addEventListener("pointerleave", () => {
        tip.hidden = true;
        memoryPointer = null;
      });
    }
    if (memoryPointer != null) show(memoryPointer); // keep the tooltip through updates
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
      let size = `${area.clientWidth}x${area.clientHeight}`;
      new ResizeObserver(() => {
        const current = `${area.clientWidth}x${area.clientHeight}`;
        if (current === size) return;
        size = current;
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
