{
  description = "Portfolio site (Rust + Axum + Maud)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAll = f: nixpkgs.lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});
    in
    {
      packages = forAll (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "portfolio";
          version = "0.1.0";
          src = nixpkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          postInstall = ''
            mkdir -p $out/share/portfolio
            cp -r static $out/share/portfolio/
          '';
          meta.mainProgram = "portfolio";
        };
      });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [ cargo rustc rustfmt clippy rust-analyzer ];
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
                environment.ADDR = cfg.address;
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
                  locations."/static/" = {
                    alias = "${cfg.package}/share/portfolio/static/";
                    extraConfig = ''
                      expires 7d;
                      add_header Cache-Control "public";
                    '';
                  };
                };
              };

              networking.firewall.allowedTCPPorts = [ 80 443 ];
            })
          ]);
        };
    };
}
