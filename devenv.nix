{
  pkgs,
  config,
  lib,
  inputs,
  ...
}:
let
  ic-nix = import inputs.ic-nix {
    pkgs = pkgs.appendOverlays [
      (_self: _prev: {
        pkgsHostHost = _prev;
        rust-stable = config.languages.rust.toolchainPackage;
      })
    ];
  };

  ic-nix-packages = with ic-nix; [
    # SDK
    dfx
    # Utils
    icx-proxy
    idl2json
    vessel
    ic-repl
    ic-wasm
    candid
    candid-extractor
    agent-rs
    dfx-extensions
    # IC
    #binaries
    #wasm-binaries
    #canisters
  ];

  #packages = with pkgs; [ ];

  devPackages =
    with pkgs;
    [
      # General
      act
      bash
      bc
      coreutils
      dig
      figlet
      gcc
      git
      hello
      jq
      multitail
      openssl
      ripgrep
      yq-go

      # Astro
      astro-language-server
      nodePackages.postcss
      tailwindcss_4
      npm-check-updates

      # Nix
      nixd
      nil
      nixfmt

      # Rust
      cargo-audit
      cargo-bump
      cargo-edit
      cargo-update
      cargo-watch
      toml-cli

      # Security
      codeql
      trivy
    ]
    ++ ic-nix-packages;

in
{
  name = "ic-siwa";

  env = {
    #########################
    # General
    #########################
    PROJECT = config.name;

    #########################
    # DFX Configuration
    #########################
    DFX_PORT = "4943";
    JUNO_PORT = "5987";
  }
  # IC-SIWA Salt Secrets (domain/uri are in config/*.yaml)
  // (lib.optionalAttrs (config.secretspec.secrets ? IC_SIWA_SALT_DEVELOPMENT) {
    inherit (config.secretspec.secrets) IC_SIWA_SALT_DEVELOPMENT;
  })
  // (lib.optionalAttrs (config.secretspec.secrets ? IC_SIWA_SALT_TESTNET) {
    inherit (config.secretspec.secrets) IC_SIWA_SALT_TESTNET;
  })
  // (lib.optionalAttrs (config.secretspec.secrets ? IC_SIWA_SALT_MAINNET) {
    inherit (config.secretspec.secrets) IC_SIWA_SALT_MAINNET;
  })
  # DaisyUI secrets
  // (lib.optionalAttrs (config.secretspec.secrets ? DAISYUI_LICENSE) {
    inherit (config.secretspec.secrets) DAISYUI_LICENSE;
  })
  // (lib.optionalAttrs (config.secretspec.secrets ? DAISYUI_EMAIL) {
    inherit (config.secretspec.secrets) DAISYUI_EMAIL;
  })
  # GitHub Token
  // (lib.optionalAttrs (config.secretspec.secrets ? GITHUB_TOKEN) {
    inherit (config.secretspec.secrets) GITHUB_TOKEN;
  })
  // (lib.optionalAttrs (config.secretspec.secrets ? PUBLIC_WALLETCONNECT_PROJECT_ID) {
    inherit (config.secretspec.secrets) PUBLIC_WALLETCONNECT_PROJECT_ID;
  });

  cachix = {
    enable = false; # TODO: Re-enable when fixed.
    pull = [
      "irken-empire"
      "pre-commit-hooks"
      "devenv.cachix.org"
      "cache.nixos.org"
      "nix-community.cachix.org"
    ];
    push = "irken-empire";
  };

  devenv = {
    warnOnNewVersion = true;
  };

  dotenv = {
    enable = true;
    disableHint = true;
  };

  #packages = lib.optionals (!config.container.isBuilding || config.name == "devenv") devPackages;

  # AI - Claude Code Integration
  # See: https://devenv.sh/integrations/claude-code/
  claude.code = {
    enable = true;
    mcpServers = {
      devenv = {
        type = "http";
        url = "https://mcp.devenv.sh";
      };
      devenv-cli = {
        type = "stdio";
        command = "devenv";
        args = [ "mcp" ];
        env = {
          DEVENV_ROOT = config.devenv.root;
        };
      };
      astroDocs = {
        type = "http";
        url = "https://mcp.docs.astro.build/mcp";
      };
      github = {
        type = "http";
        url = "https://api.githubcopilot.com/mcp/";
        headers = {
          Authorization = lib.optionalString (config.env ? GITHUB_TOKEN) "Bearer ${config.env.GITHUB_TOKEN}";
        };
      };
      daisyui-blueprint = {
        type = "stdio";
        command = "bunx";
        args = [
          "-y"
          "daisyui-blueprint@latest"
        ];
        env =
          lib.optionalAttrs (config.env ? DAISYUI_LICENSE) { LICENSE = config.env.DAISYUI_LICENSE; }
          // lib.optionalAttrs (config.env ? DAISYUI_EMAIL) { EMAIL = config.env.DAISYUI_EMAIL; };
      };
    };
  };

}
