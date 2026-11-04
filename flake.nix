{
  description = "Portfolio site (Rust + Axum + Maud)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane/v0.24.0";
  };

  outputs = { self, nixpkgs, crane, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAll = f: nixpkgs.lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});

      # Crane builds the dependencies in their own derivations, so changing the
      # site's code or assets only rebuilds the site itself.
      build = pkgs:
        let
          inherit (pkgs) lib;
          craneLib = crane.mkLib pkgs;

          # Only what the build needs: editing the justfile, hooks or deploy
          # example doesn't trigger a rebuild. build.rs reads static/.
          src = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              (craneLib.fileset.commonCargoSources ./.)
              ./static
            ];
          };

          commonArgs = {
            inherit src;
            strictDeps = true;
          };

          # Release dependencies (default features) for the package.
          releaseDeps = craneLib.buildDepsOnly commonArgs;

          # Dev-profile dependencies with all features for clippy and tests,
          # matching the pre-commit hook (debug assertions on, no LTO link).
          devArgs = commonArgs // {
            CARGO_PROFILE = "dev";
            cargoExtraArgs = "--locked --all-features";
          };
          devDeps = craneLib.buildDepsOnly (devArgs // { pname = "portfolio-dev"; });
        in
        {
          package = craneLib.buildPackage (commonArgs // {
            cargoArtifacts = releaseDeps;
            doCheck = false; # tests run in checks.test
            postInstall = ''
              mkdir -p $out/share/portfolio
              cp -r static $out/share/portfolio/
            '';
            meta.mainProgram = "portfolio";
          });

          checks = {
            clippy = craneLib.cargoClippy (devArgs // {
              cargoArtifacts = devDeps;
              cargoClippyExtraArgs = "--all-targets -- -D warnings";
            });
            test = craneLib.cargoTest (devArgs // { cargoArtifacts = devDeps; });
            fmt = craneLib.cargoFmt { inherit src; };
          };
        };
    in
    {
      packages = forAll (pkgs: { default = (build pkgs).package; });

      # `nix flake check`: clippy (warnings as errors), tests, rustfmt and the
      # release build, all reusing the cached dependency builds.
      checks = forAll (pkgs: (build pkgs).checks // { build = (build pkgs).package; });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [ cargo rustc rustfmt clippy rust-analyzer just watchexec ];
          # Use the versioned hooks in .githooks/ (pre-commit: fmt, clippy, flake eval).
          shellHook = ''
            if [ -d .git ] && [ "$(git config --get core.hooksPath)" != .githooks ]; then
              git config core.hooksPath .githooks
              echo "git hooks enabled (.githooks/)"
            fi
          '';
        };
      });

      nixosModules.default = { config, lib, pkgs, ... }:
        let
          cfg = config.services.portfolio;
        in
        {
          options.services.portfolio = {
            enable = lib.mkEnableOption "the portfolio site";

            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
              description = "The portfolio package to run.";
            };

            address = lib.mkOption {
              type = lib.types.str;
              default = "127.0.0.1:3000";
              description = "Address the app listens on (behind nginx).";
            };

            logLevel = lib.mkOption {
              type = lib.types.str;
              default = "info";
              example = "warn,portfolio=info";
              description = ''
                `RUST_LOG` filter for the app. `info` logs every request
                (method, path, status, latency); use `warn` if nginx's access
                log is enough.
              '';
            };

            domain = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              example = "portfolio.example.com";
              description = ''
                Domain to serve through nginx with a Let's Encrypt certificate.
                Leave null to configure the reverse proxy yourself.
              '';
            };
          };

          config = lib.mkIf cfg.enable (lib.mkMerge [
            {
              systemd.services.portfolio = {
                description = "Portfolio site";
                wantedBy = [ "multi-user.target" ];
                after = [ "network.target" ];
                environment = {
                  ADDR = cfg.address;
                  RUST_LOG = cfg.logLevel;
                };
                serviceConfig = {
                  ExecStart = lib.getExe cfg.package;
                  # The app serves ./static relative to its working directory.
                  WorkingDirectory = "${cfg.package}/share/portfolio";
                  Restart = "on-failure";

                  # Hardening
                  DynamicUser = true;
                  ProtectSystem = "strict";
                  ProtectHome = true;
                  PrivateTmp = true;
                  PrivateDevices = true;
                  NoNewPrivileges = true;
                  ProtectKernelTunables = true;
                  ProtectKernelModules = true;
                  ProtectControlGroups = true;
                  RestrictNamespaces = true;
                  LockPersonality = true;
                  MemoryDenyWriteExecute = true;
                  RestrictAddressFamilies = [ "AF_INET" "AF_INET6" ];
                  CapabilityBoundingSet = "";
                  SystemCallFilter = [ "@system-service" "~@privileged" ];
                };
              };
            }

            (lib.mkIf (cfg.domain != null) {
              services.nginx = {
                enable = true;
                recommendedProxySettings = true;
                recommendedTlsSettings = true;
                recommendedGzipSettings = true;
                recommendedOptimisation = true;

                virtualHosts.${cfg.domain} = {
                  enableACME = true;
                  forceSSL = true;

                  locations."/".proxyPass = "http://${cfg.address}";

                  # nginx serves static files straight from the Nix store.
                  # Same policy as the app: versioned URLs (?v=<content hash>)
                  # are cached forever, anything else must be revalidated.
                  # ETag/Last-Modified are off because every file in the Nix
                  # store has the same mtime, so they can't detect changes.
                  locations."/static/" = {
                    alias = "${cfg.package}/share/portfolio/static/";
                    extraConfig = ''
                      etag off;
                      if_modified_since off;
                      add_header Cache-Control $portfolio_static_cache_control;
                    '';
                  };
                };

                appendHttpConfig = ''
                  map $arg_v $portfolio_static_cache_control {
                    ""      "no-cache";
                    default "public, max-age=31536000, immutable";
                  }
                '';
              };

              networking.firewall.allowedTCPPorts = [ 80 443 ];
            })
          ]);
        };
    };
}
