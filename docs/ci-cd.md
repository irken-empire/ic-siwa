# CI/CD Architecture

## Release Flow

```mermaid
graph TD
    C["Manual dispatch"] --> B["cd.yaml"]

    B --> D{"Environment?"}
    D -->|"Testnet"| E["pre-release job"]
    D -->|"Mainnet"| F["promote job"]

    E --> G["Create GitHub Pre-Release<br/>(with convco changelog)"]
    G --> H["cd-testnet.yaml<br/>(workflow_call)"]

    F --> I["Auto-discover or validate tag"]
    I --> J["Promote Pre-Release → Release<br/>(with convco changelog)"]
    J --> K["cd-mainnet.yaml<br/>(workflow_call)"]

    H --> L["Build → Deploy → Verify → npm (dry run)"]
    K --> M["Build → Deploy → Verify → npm (publish)"]
```

## Developer Workflow

```mermaid
graph LR
    A["Create branch"] --> B["Open PR"]
    B --> C["CI runs<br/>(lint, test, build, security)"]
    C --> D["PR approved"]
    D --> E["Merge queue"]
    E --> F["Merged to trunk"]
    F --> G["Merge more PRs<br/>(accumulate changes)"]
    G --> H["Run cd.yaml<br/>(Testnet — manual)"]
    H --> I["Pre-release created<br/>+ Testnet deployed"]
    I --> J["UAT / QA"]
    J --> K["Run cd.yaml<br/>(Mainnet)"]
    K --> L["Promoted + deployed"]
```

## Manually Runnable Workflows

Only these workflows can be triggered manually via `workflow_dispatch`:

| Workflow                   | Purpose                                           |
| -------------------------- | ------------------------------------------------- |
| `cd.yaml`                  | Create pre-release (Testnet) or promote (Mainnet) |
| `chore-devenv-update.yaml` | Update devenv.lock, create PR                     |

All other workflows are triggered automatically by events (PR, push, merge_group, schedule, workflow_call).

## Mainnet Promotion

When running `cd.yaml` with `Environment: Mainnet`:

- **With `release_tag`**: Promotes that specific pre-release
- **Without `release_tag`**: Auto-discovers the latest pre-release and promotes it
- **If latest release is already promoted**: Errors with a helpful message

The GitHub `Mainnet` environment requires manual approval before the promote job runs.

## CI Pipeline

```mermaid
graph LR
    PR["Pull Request"] --> CC["check-cache<br/>(lookup-only, ~5s)"]

    CC --> |cache hit| CI["ci.yaml jobs<br/>(all parallel)"]
    CC --> |cache miss| SE["setup-environment<br/>(warm cache, ~60m)"]
    SE --> CI

    CI --> |workflow_call| CI_LINT["ci-lint.yaml"]
    CI --> |workflow_call| CI_TEST["ci-test.yaml"]
    CI --> |workflow_call| CI_BUILD_CAN["ci-build-canisters.yaml"]
    CI --> |workflow_call| CI_BUILD_TS["ci-build-typescript.yaml"]
    CI --> |workflow_call| CI_PR_TITLE["ci-pr-title.yaml"]
    CI --> |workflow_call| CI_TRIVY["ci-trivy.yaml"]
    CI --> |workflow_call| CI_CODEQL["sec-codeql.yaml"]
    BC --> |workflow_call| CI_VERIFY_CAN["ci-verify-candid.yaml"]
```

### The "Gatekeeper" Strategy

We use a cache-aware orchestration pipeline that avoids redundant warm-up work.

1. `check-cache` runs a **lookup-only** probe (`actions/cache` with `lookup-only: true`) against
   the Nix store key `${{ runner.os }}-nix-${{ hashFiles('devenv.lock', 'devenv.nix') }}`.
   This takes ~5 seconds and does not download anything.
2. If the cache **exists** (common case Mon–Fri): `setup-environment` is skipped and all CI
   jobs fire immediately in parallel.
3. If the cache **is missing** (new `devenv.lock` hash, or after the weekly chore): `setup-environment`
   runs synchronously (~60 min) to warm the cache, then all CI jobs fan out.

The weekly `chore-devenv-update.yaml` runs every Sunday night and warms the cache for the coming
week, so developer PRs consistently hit the fast (~10 min) path.

**Cache Key (`devenv.lock` + `devenv.nix` Hash)**
The `setup-devenv` action uses `hashFiles('devenv.lock', 'devenv.nix')` for the primary cache key,
achieving a **0-second upload penalty** on subsequent restores.

## Workflow Inventory

| Workflow                   | Prefix | Trigger                     | Purpose                                          |
| -------------------------- | ------ | --------------------------- | ------------------------------------------------ |
| `cd.yaml`                  | cd     | dispatch only               | Create pre-release or promote to release         |
| `cd-testnet.yaml`          | cd     | workflow_call only          | Build, deploy, verify on IC Testnet              |
| `cd-mainnet.yaml`          | cd     | workflow_call only          | Build, deploy, verify, publish npm on IC Mainnet |
| `ci.yaml`                  | ci     | pull_request, merge_group   | Gatekeeper entrypoint for CI                     |
| `ci-lint.yaml`             | ci     | workflow_call only          | Lint the project                                 |
| `ci-test.yaml`             | ci     | workflow_call only          | Test the project                                 |
| `ci-build-canisters.yaml`  | ci     | workflow_call only          | Build all IC canisters                           |
| `ci-build-typescript.yaml` | ci     | workflow_call only          | Build npm package                                |
| `ci-verify-candid.yaml`    | ci     | workflow_call only          | Verify Candid interfaces                         |
| `chore-devenv-update.yaml` | chore  | schedule (weekly), dispatch | Update devenv.lock, create PR                    |
| `ci-pr-title.yaml`         | ci     | workflow_call only          | Validate conventional commit PR titles           |
| `sec-codeql.yaml`          | sec    | workflow_call only          | CodeQL security analysis                         |
| `ci-trivy.yaml`            | ci     | workflow_call only          | Trivy vulnerability scanning                     |

## Composite Actions

| Action               | Purpose                                                |
| -------------------- | ------------------------------------------------------ |
| `setup-devenv`       | Nix + Cachix + devenv + secretspec (checkout separate) |
| `report-status`      | Report CI status to GitHub commit status API           |
| `read-config`        | Read environment-specific YAML config                  |
| `build-canister`     | Build Rust WASM or Assets canister                     |
| `deploy-canister`    | Deploy canister to IC mainnet                          |
| `verify-canister`    | Run smoke tests against deployed canister              |
| `setup-canister-ids` | Copy env-specific `canister_ids.json`                  |
| `build-npm`          | Build ic-siwa TypeScript library                       |
| `publish-npm`        | Publish to npm (supports dry run)                      |
| `register-domain`    | Register custom domain with IC boundary nodes          |

## Naming Conventions

| Element           | Convention                                 | Example                    |
| ----------------- | ------------------------------------------ | -------------------------- |
| Workflow filename | `{ci\|cd\|chore}-{component}[-{env}].yaml` | `chore-devenv-update.yaml` |
| Workflow `name:`  | Title Case                                 | `Deploy (Testnet)`         |
| Job ID            | `kebab-case`                               | `deploy-provider`          |
| Job `name:`       | Title Case                                 | `Deploy Provider`          |
| Step ID           | `snake_case`                               | `setup_devenv`             |
| Step `name:`      | Title Case Verb-Noun                       | `Setup Devenv`             |
| Action inputs     | `kebab-case`                               | `github-token`             |
| File extension    | `.yaml`                                    | —                          |
