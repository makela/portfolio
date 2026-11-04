# Run inside the dev shell: `nix develop`, then `just <recipe>`.

addr := env("ADDR", "127.0.0.1:3000")

# List recipes
default:
    @just --list

# Dev server: rebuild and restart on Rust changes, reload the browser on any change
watch:
    ADDR={{addr}} watchexec --restart --exts rs,toml --watch src --watch build.rs --watch Cargo.toml -- cargo run --features livereload

# Run the server once, without watching
run:
    ADDR={{addr}} cargo run

# Format the code
fmt:
    cargo fmt

# Same checks as the pre-commit hook
check:
    .githooks/pre-commit

# Full Nix check: clean build, clippy, rustfmt and tests (same as pre-push)
check-all:
    nix flake check

# Render the social preview (og:image) from design/og-image.svg
og-image:
    rsvg-convert --width 1200 --height 630 design/og-image.svg --output static/og-image.png
