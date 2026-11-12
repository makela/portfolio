# portfolio

My personal site: a portfolio of projects and a resume. Server-rendered with
[Rust](https://www.rust-lang.org/), [Axum](https://github.com/tokio-rs/axum)
and [Maud](https://maud.lambda.xyz/) templates, built and deployed with
[Nix](https://nixos.org/).

- Two pages, `/` (projects) and `/resume`, plus `robots.txt` and `sitemap.xml`,
  and a not-found page for any other address
- A live `/status` page with the memory and CPU use of the sites on the server,
  read from the Linux kernel like top does
- No JavaScript framework and no build step for the front end; the only
  scripts are a small light/dark theme toggle and the live updates of `/status`
- Content-hashed asset URLs (`/static/style.css?v=…`), cached for a year;
  only the current hash is, not an old or made-up one
- Self-hosted [IBM Plex Sans](https://github.com/IBM/plex) (no third-party
  requests)
- SEO tags, Open Graph link previews and JSON-LD structured data
- A NixOS module that runs the site as a hardened systemd service behind
  nginx with Let's Encrypt and security headers (a strict Content Security
  Policy, HSTS, nosniff)

## Development

You need [Nix](https://nixos.org/download/) with flakes enabled. Everything
else comes from the dev shell.

```sh
nix develop      # or `direnv allow` once, if you use direnv
just watch       # http://127.0.0.1:3000, reloads the browser on changes
```

| Command          | What it does                                                |
| ---------------- | ----------------------------------------------------------- |
| `just watch`     | Dev server: restarts on Rust changes, live-reloads the page |
| `just run`       | Run the server once                                         |
| `just check`     | rustfmt, clippy, tests and a flake evaluation               |
| `just check-all` | Full `nix flake check`: clean build, clippy, rustfmt, tests |
| `just og-image`  | Render `static/og-image.png` from `design/og-image.svg`     |
| `just fmt`       | Format the code                                             |

Entering the dev shell enables the git hooks in `.githooks/`: `just check`
runs before every commit and `just check-all` before every push.

### Configuration

| Variable       | Default          | Purpose                                                       |
| -------------- | ---------------- | ------------------------------------------------------------- |
| `ADDR`         | `127.0.0.1:3000` | Address to listen on                                          |
| `SITE_URL`     | `http://$ADDR`   | Public URL for canonical links, link previews and the sitemap |
| `RUST_LOG`     | `info`           | Log filter                                                    |
| `STATUS_SITES` | none             | Sites on `/status`, see [Server status](#server-status)       |
| `STATUS_CGROUP_ROOT` | `/sys/fs/cgroup/system.slice` | Where the units in `STATUS_SITES` are                  |

### Where things are

| Path                      | Contents                                              |
| ------------------------- | ----------------------------------------------------- |
| `src/site.rs`             | Name, tagline, intro and other site-wide texts        |
| `src/projects.rs`         | The project cards                                     |
| `src/resume.rs`           | The resume page                                       |
| `src/templates.rs`        | Shared layout and the front page                      |
| `src/seo.rs`              | `robots.txt`, `sitemap.xml` and structured data       |
| `src/status.rs`           | `/status`: reading /proc and the services' cgroups    |
| `static/`                 | CSS, theme script, fonts, images                      |
| `deploy/example-host.nix` | Example NixOS configuration using the module          |

## Deployment

The flake exports a NixOS module. Add this repository as an input of your
server's flake and enable the service:

```nix
services.portfolio = {
  enable = true;
  domain = "example.com"; # nginx + Let's Encrypt for this domain
};
```

See [`deploy/example-host.nix`](deploy/example-host.nix) for the full example
and the options in [`flake.nix`](flake.nix) (`address`, `siteUrl`,
`logLevel`, `status.sites`).

### Server status

`/status` shows the memory, CPU and uptime of the sites on the server, and the
host's load, memory and uptime. A CPU chart draws a line per site, on a scale
that fits them, and a bar shows their shares of the memory they use. Every other
page's footer shows both, without the legend, as the link to `/status`. List the
sites in the module, each with the systemd units that serve it:

```nix
services.portfolio.status.sites = {
  "Portfolio" = [ "portfolio" ];
  "My blog" = [ "blog" "blog-admin" ];
  "My shop" = [ "shop" ];
  "Database" = [ "postgresql" ]; # any service works, not only sites
};
```

- Names are free text, without `;` or `=`. Units are system services, with or
  without `.service`; a site's memory and CPU are the sum of its units.
- Only the listed units are read or shown, so the page doesn't reveal what
  else runs on the host. A listed unit that isn't running shows as
  "not running".
- A site's uptime is that of the oldest process in its running units, shown
  under its name in the legend.
- Memory is what systemd counts for the service (`memory.current` of its
  cgroup, file cache included), CPU a share of the whole server.

Outside NixOS, set `STATUS_SITES` yourself: `Name=unit,unit;Name=unit`.

`just watch` runs in its own systemd user scope, `portfolio-watch.scope`, and
lists it as the only site, so `/status` shows what the dev session uses:
watchexec, the cargo builds and the server. Without a systemd user session
(e.g. on macOS) it runs as before and `/status` shows only the host.

The server reads the counters on request (at most every 2 seconds) and keeps
no history. With JavaScript, `static/status.js` polls `/status.json` every 5
seconds, computes the CPU percentages and draws charts of the last 10 minutes.
The tab keeps that history in `sessionStorage`, and the script runs on every
page while a site is running, so the charts carry on while the visitor
browses the site; a new tab starts over. Without JavaScript, the page shows a
single reading.

## License

The code is available under the [MIT License](LICENSE).

The content is not: the resume and other personal texts (`src/site.rs`,
`src/resume.rs`), the project descriptions (`src/projects.rs`) and the images
(`static/images/`, `static/favicon.svg`, `static/og-image.png`, `design/`)
are © Timo Mäkelä, all rights reserved. If you use this code for your own
site, replace them with your own.

IBM Plex Sans in `static/fonts/` is © IBM Corp. and licensed under the
[SIL Open Font License 1.1](static/fonts/LICENSE.txt).
