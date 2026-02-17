{
  pkgs,
  config,
  lib,
  inputs,
  ...
}:
let
  ic-nix-release = "20260203";

  ic-nix =
    import
      (fetchTarball {
        url = "https://github.com/ninegua/ic-nix/archive/refs/tags/${ic-nix-release}.tar.gz";
        sha256 = "0vydcs3m46rwm3z6d4mqvxsa8s26rl5qw3apy5lpvg8146h2y54g";
      })
      {
        pkgs = pkgs.appendOverlays [
          (_self: _super: {
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

  pkgsUnstable = import inputs.nixpkgs-unstable {
    config.allowUnfree = true;
  };

  #packages = with pkgs; [ ];

  packagesUnstable = with pkgsUnstable; [
    tailwindcss_4
    gemini-cli-bin
    claude-code
  ];

  devPackages =
    with pkgs;
    [
      # AI Agents
      #gemini-cli-bin
      #claude-code

      # General
      bash
      coreutils
      dig
      figlet
      gcc
      git
      git
      hello
      jq
      multitail
      openssl
      ripgrep
      yq-go

      # Devenv
      direnv
      secretspec

      # Rust
      cargo-audit
      cargo-bump
      cargo-edit
      cargo-update
      cargo-watch
      toml-cli

      # Astro
      astro-language-server
      nodePackages.postcss
      #tailwindcss_4
      npm-check-updates

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
    pull = [
      "pre-commit-hooks"
      "irken-empire"
    ];
    push = "irken-empire";
  };

  devenv = {
    warnOnNewVersion = true;
  };

  dotenv = {
    enable = true;
    disableHint = false;
  };

  packages =
    packagesUnstable
    ++ lib.optionals (!config.container.isBuilding || config.name == "devenv") devPackages;

  enterShell = ''
    if [[ "''${CI:-false}" == "true" ]];
    then
      echo "devenv running in CI"
    else
      figlet -f starwars -w 180 $PROJECT

      hello --greeting="Hello ''${USER:-user}, welcome to the $PROJECT project!"

      echo ""
      echo "#########################"
      echo "#### Helper scripts #####"
      echo "#########################"
      echo "🦾"
      ${lib.concatStrings (
        lib.mapAttrsToList (
          name: value: "printf '🦾 %-20s  %s\\n' '${name}' '${value.description}'\n"
        ) config.scripts
      )}
      echo "🦾"
      echo "#########################"
    fi
  '';

  # AI - Claude Code Integration
  # See: https://devenv.sh/integrations/claude-code/
  claude.code = {
    enable = true;
    mcpServers = {
      devenv = {
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

  languages = {
    nix = {
      enable = true;
    };
    shell = {
      enable = true;
    };
    javascript = {
      enable = true;
      bun = {
        enable = true;
      };
      npm = {
        enable = true;
      };
    };
    rust = {
      enable = true;
      channel = "stable";
      components = [
        "rustc"
        "cargo"
        "clippy"
        "rustfmt"
        "rust-analyzer"
      ];
      #rustflags = "--cfg getrandom_backend=\"wasm_js\"";
      targets = [ "wasm32-unknown-unknown" ];
    };
    solidity = {
      enable = true;
      foundry = {
        enable = true;
      };
    };
  };

  difftastic = {
    enable = true;
  };

  git-hooks = {
    excludes = [
      ".direnv/"
      ".dist/"
      ".git/"
      ".vscode/"
      "^docs/agents/.*"
    ];
    hooks = {
      actionlint.enable = true;
      action-validator.enable = true;
      convco = {
        enable = true;
        settings = {
          configPath = ".versionrc";
        };
      };
      cargo-check.enable = true;
      clippy = {
        enable = false; # ic-nix is using older toolchain.
        settings = {
          denyWarnings = true;
          offline = true;
          allFeatures = true;
          extraArgs = "--target wasm32-unknown-unknown";
        };
      };
      check-json.enable = true;
      check-merge-conflicts.enable = true;
      check-shebang-scripts-are-executable = {
        enable = true;
        excludes = [
          "\\.rs$"
        ];
      };
      check-symlinks.enable = true;
      check-yaml.enable = true;
      commitizen.enable = true;
      deadnix.enable = true;
      editorconfig-checker.enable = true;
      eslint.enable = false;
      eslint-hack = {
        enable = true;
        name = "eslint-hack";
        entry = "eslint-check";
        files = "^(canisters|libs)/.*$";
        pass_filenames = false;
      };
      # Astro type checking for frontend canister
      astro-check = {
        enable = true;
        name = "astro-check";
        entry = "bash -c 'cd canisters/test_canister_ts && bun run astro check'";
        files = "^canisters/test_canister_ts/.*\\.(astro|ts|tsx)$";
        pass_filenames = false;
      };
      # TypeScript type checking for ic-siwa library
      tsc-lib = {
        enable = true;
        name = "tsc-lib";
        entry = "bash -c 'cd libs/ic_siwa_ts && bun run tsc --noEmit'";
        files = "^libs/ic_siwa_ts/.*\\.ts$";
        pass_filenames = false;
      };
      markdownlint = {
        excludes = [
          "^docs/issues/todo/.*\\.md$" # Ignore todo notes.
        ];
        enable = true;
        settings = {
          configuration = {
            MD013 = {
              line_length = 200;
              tables = false;
              code_blocks = false;
            };
            MD033 = {
              allowed_elements = [
                "a"
                "br"
                "nobr"
                "pre"
                "sup"
              ];
            };
          };
        };
      };
      mixed-line-endings.enable = true;
      nixfmt.enable = true;
      prettier = {
        enable = true;
        settings = {
          configPath = ".prettierrc.yaml";
        };
      };
      pretty-format-json = {
        enable = false;
      };
      revive = {
        enable = true;
        fail_fast = false;
      };
      ripsecrets = {
        enable = true;
      };
      rustfmt.enable = true;
      shellcheck = {
        enable = true;
      };
      shfmt.enable = true;
      staticcheck.enable = true;
      statix.enable = true;
      trim-trailing-whitespace = {
        excludes = [
          # Ignore generated candid files
          ".*\\.did$"
          ".*/candid/.*\\.ts$"
        ];
        enable = true;
      };
      trufflehog.enable = true;
      cspell = {
        enable = true;
        excludes = [
          "\\.webp$"
          "\\.png$"
          "\\.jpg$"
          "\\.jpeg$"
          "\\.gif$"
          "\\.ico$"
          "\\.svg$"
          "\\.woff2?$"
          "\\.ttf$"
          "\\.eot$"
          "\\.mp3$"
          "\\.mp4$"
          "\\.ogg$"
          "\\.wav$"
          "\\.wasm$"
        ];
      };
      yamllint = {
        enable = true;
        settings = {
          configuration = ''
            extends: relaxed
            rules:
              line-length: disable
              indentation: enable
          '';
        };
      };
      version-reset = {
        enable = true;
        name = "version-reset";
        description = "Reset all version files to 0.0.0 (CI/convco is the source of truth)";
        entry = "ic-siwa version --reset";
        files = "(^Cargo\\.toml$|^package\\.json$|libs/ic_siwa_ts/package\\.json$|canisters/test_canister_ts/package\\.json$)";
        pass_filenames = false;
      };
      # Regenerate Candid when Rust canister code changes
      candid-gen = {
        enable = true;
        name = "candid-gen";
        description = "Regenerate Candid interface from canister code";
        entry = "ic-siwa candid";
        files = "^canisters/ic_siwa_provider/src/.*\\.rs$";
        pass_filenames = false;
      };
    };
  };

  starship = {
    enable = true;
    config = {
      enable = false;
    };
  };

  devcontainer = {
    enable = true;
    settings = {
      customizations = {
        vscode = {
          extensions = [
            "arrterian.nix-env-selector"
            "astro-build.astro-vscode"
            "esbenp.prettier-vscode"
            "github.vscode-github-actions"
            "gruntfuggly.todo-tree"
            "johnpapa.vscode-peacock"
            "mkhl.direnv"
            "nhoizey.gremlins"
            "pinage404.nix-extension-pack"
            "redhat.vscode-yaml"
            "streetsidesoftware.code-spell-checker"
            "timonwong.shellcheck"
            "tuxtina.json2yaml"
            "vscodevim.vim"
            "wakatime.vscode-wakatime"
            "yzhang.markdown-all-in-one"
          ];
        };
      };
    };
  };

  scripts = {
    eslint-check = {
      package = pkgs.bash;
      description = "A workaround to use a more modern version of ESLint.";
      exec = ''
        bun install
        eslint canisters/ libs/
      '';
    };
    ic-siwa = {
      package = pkgs.bash;
      description = "Developer entrypoint script for ic-siwa.";
      exec = ''
        ./scripts/ic-siwa.sh "$@"
      '';
    };
    codeql-run = {
      package = pkgs.bash;
      description = "Run CodeQL static analysis locally.";
      exec = ''
        ./scripts/codeql-run.sh "$@"
      '';
    };
  };

  enterTest = ''
    echo "Running devenv tests..."
  '';
}
