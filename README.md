# portfolio

My personal site: a portfolio of projects and a resume. Server-rendered with
[Rust](https://www.rust-lang.org/), [Axum](https://github.com/tokio-rs/axum)
and [Maud](https://maud.lambda.xyz/) templates, built and deployed with
[Nix](https://nixos.org/).

- Two pages, `/` (projects) and `/resume`, plus `robots.txt` and `sitemap.xml`
- No JavaScript framework and no build step for the front end; the only script
  is a small light/dark theme toggle
- Content-hashed asset URLs (`/static/style.css?v=…`), cached for a year
- Self-hosted [IBM Plex Sans](https://github.com/IBM/plex) (no third-party
  requests)
- SEO tags, Open Graph link previews and JSON-LD structured data
- A NixOS module that runs the site as a hardened systemd service behind
  nginx with Let's Encrypt

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

| Variable   | Default          | Purpose                                                       |
| ---------- | ---------------- | ------------------------------------------------------------- |
| `ADDR`     | `127.0.0.1:3000` | Address to listen on                                          |
| `SITE_URL` | `http://$ADDR`   | Public URL for canonical links, link previews and the sitemap |
| `RUST_LOG` | `info`           | Log filter                                                    |

### Where things are

| Path                      | Contents                                              |
| ------------------------- | ----------------------------------------------------- |
| `src/site.rs`             | Name, tagline, intro and other site-wide texts        |
| `src/projects.rs`         | The project cards                                     |
| `src/resume.rs`           | The resume page                                       |
| `src/templates.rs`        | Shared layout and the front page                      |
| `src/seo.rs`              | `robots.txt`, `sitemap.xml` and structured data       |
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
`logLevel`).

## License

The code is available under the [MIT License](LICENSE).

The content is not: the resume and other personal texts (`src/site.rs`,
`src/resume.rs`), the project descriptions (`src/projects.rs`) and the images
(`static/images/`, `static/favicon.svg`, `static/og-image.png`, `design/`)
are © Timo Mäkelä, all rights reserved. If you use this code for your own
site, replace them with your own.

IBM Plex Sans in `static/fonts/` is © IBM Corp. and licensed under the
[SIL Open Font License 1.1](static/fonts/LICENSE.txt).
