# Example NixOS host configuration using the portfolio module.
#
# In the server's flake:
#   inputs.portfolio.url = "github:makela/portfolio";
#   ...
#   nixosConfigurations.myserver = nixpkgs.lib.nixosSystem {
#     specialArgs = { inherit inputs; };
#     modules = [ ./configuration.nix ./portfolio.nix ];   # this file as portfolio.nix
#   };
{ inputs, ... }:
{
  imports = [ inputs.portfolio.nixosModules.default ];

  services.portfolio = {
    enable = true;
    domain = "makela.in"; # nginx + Let's Encrypt for this domain

    # Public URL for the canonical link, link previews, robots.txt and the
    # sitemap. Defaults to "https://<domain>", here "https://makela.in".
    # siteUrl = "https://makela.in";

    # Address the app listens on; passed to the app as ADDR, and nginx
    # proxies to it automatically. Defaults to "127.0.0.1:3000".
    # Change the port if 3000 is taken, e.g. by another site on this host.
    # Keep it on 127.0.0.1 so the app is only reachable through nginx.
    address = "127.0.0.1:3001";

    # RUST_LOG filter. The default "info" logs every request (method, path,
    # status, latency) to the journal: `journalctl -u portfolio`. nginx keeps
    # its own access log too, so "warn" is enough if you don't need both.
    # logLevel = "warn";
  };

  # Optional: send www.makela.in to makela.in, so search engines see one
  # address (needs a DNS record for www.makela.in pointing to this host).
  # services.nginx.virtualHosts."www.makela.in" = {
  #   enableACME = true;
  #   forceSSL = true;
  #   globalRedirect = "makela.in";
  # };

  # Required once per host for Let's Encrypt certificates.
  security.acme = {
    acceptTerms = true;
    defaults.email = "you@example.com";
  };
}
