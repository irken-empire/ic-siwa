{
  pkgs,
  config,
  lib,
  inputs,
  ...
}:
let
  ic-nix-release = "20251223";

  ic-nix =
    import
      (fetchTarball {
        url = "https://github.com/ninegua/ic-nix/archive/refs/tags/${ic-nix-release}.tar.gz";
        sha256 = "0vv46lcl0128p2kw7ml0qi8dhaqgkiw4hzws2jlp1k7v210h6ipp";
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
      cargo-bump
      cargo-watch
      toml-cli

      # Astro
      astro-language-server
      nodePackages.postcss
      #tailwindcss_4
      npm-check-updates

      # Security
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
  # IC-SIWA Development Secrets
  // (lib.optionalAttrs (config.secretspec.secrets ? IC_SIWA_SALT_DEVELOPMENT) {
    inherit (config.secretspec.secrets) IC_SIWA_SALT_DEVELOPMENT;
  })
  // (lib.optionalAttrs (config.secretspec.secrets ? IC_SIWA_DOMAIN_DEVELOPMENT) {
    inherit (config.secretspec.secrets) IC_SIWA_DOMAIN_DEVELOPMENT;
  })
  // (lib.optionalAttrs (config.secretspec.secrets ? IC_SIWA_URI_DEVELOPMENT) {
    inherit (config.secretspec.secrets) IC_SIWA_URI_DEVELOPMENT;
  })
  # IC-SIWA Testnet Secrets
  // (lib.optionalAttrs (config.secretspec.secrets ? IC_SIWA_SALT_TESTNET) {
    inherit (config.secretspec.secrets) IC_SIWA_SALT_TESTNET;
  })
  // (lib.optionalAttrs (config.secretspec.secrets ? IC_SIWA_DOMAIN_TESTNET) {
    inherit (config.secretspec.secrets) IC_SIWA_DOMAIN_TESTNET;
  })
  // (lib.optionalAttrs (config.secretspec.secrets ? IC_SIWA_URI_TESTNET) {
    inherit (config.secretspec.secrets) IC_SIWA_URI_TESTNET;
  })
  # IC-SIWA Mainnet Secrets
  // (lib.optionalAttrs (config.secretspec.secrets ? IC_SIWA_SALT_MAINNET) {
    inherit (config.secretspec.secrets) IC_SIWA_SALT_MAINNET;
  })
  // (lib.optionalAttrs (config.secretspec.secrets ? IC_SIWA_DOMAIN_MAINNET) {
    inherit (config.secretspec.secrets) IC_SIWA_DOMAIN_MAINNET;
  })
  // (lib.optionalAttrs (config.secretspec.secrets ? IC_SIWA_URI_MAINNET) {
    inherit (config.secretspec.secrets) IC_SIWA_URI_MAINNET;
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
    figlet -f starwars -w 180 $PROJECT

    hello --greeting="Hello ''${USER:-user}, welcome to the $PROJECT project!"

    echo ""
    echo "#########################"
    echo "#### Helper scripts #####"
    echo "#########################"
    echo "🦾"
    ${pkgs.gnused}/bin/sed -e 's| |••|g' -e 's|=| |' <<EOF | ${pkgs.util-linuxMinimal}/bin/column -t | ${pkgs.gnused}/bin/sed -e 's|^|🦾 |' -e 's|••| |g'
    ${lib.generators.toKeyValue { } (lib.mapAttrs (_name: value: value.description) config.scripts)}
    EOF
    echo "🦾"
    echo "#########################"
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
        enable = false;
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
      convco = {
        enable = true;
        settings = {
          configPath = ".versionrc";
        };
      };
      cargo-check.enable = true;
      clippy = {
        enable = false; # TODO: Re-enable when ic-nix is v1.9.2
        settings = {
          denyWarnings = true;
          offline = true;
          allFeatures = true;
          #extraArgs = "--target wasm32-unknown-unknown";
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
      # TODO: Update this for frontend astro linting.
      #astro-check = {
      #  enable = true;
      #  name = "astro-check";
      #  entry = "bun run astro check";
      #  files = "^src/.*\\.(astro|ts|tsx)$";
      #  pass_filenames = false;
      #};
      eslint.enable = false;
      # TODO: Update this for frontend typescript linting.
      #eslint-hack = {
      #  enable = true;
      #  name = "eslint-hack";
      #  entry = "eslint-check";
      #  files = "^src/.*$";
      #  pass_filenames = false;
      #};
      markdownlint = {
        excludes = [
          "^docs/tickets/todo/.*\\.md$" # Ignore todo notes.
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
      nixfmt-rfc-style.enable = true;
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
      shellcheck = {
        enable = true;
      };
      shfmt.enable = true;
      staticcheck.enable = true;
      statix.enable = true;
      trim-trailing-whitespace.enable = true;
      trufflehog.enable = true;
      typos.enable = true;
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
            "tekumura.typos-vscode"
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
    ic-siwa = {
      package = pkgs.bash;
      description = "Developer entrypoint script for ic-siwa.";
      exec = ''
        ./scripts/ic-siwa.sh "$@"
      '';
    };

  };

  enterTest = ''
    echo "Running devenv tests..."
  '';
}
