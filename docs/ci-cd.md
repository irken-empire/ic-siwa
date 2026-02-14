# CI/CD Pipeline

## Overview

The ic-siwa project uses GitHub Actions for CI/CD with a release-driven deployment model.
Workflows are chained using `workflow_call` for reliable, explicit orchestration.

## Release Flow

```mermaid
flowchart TD
    A["Push to trunk"] --> B["cd-release.yml"]
    C["Manual dispatch<br/>(Testnet)"] --> B
    D["Manual dispatch<br/>(Mainnet)"] --> B

    B --> E{"Environment?"}

    E -->|"Testnet<br/>(auto or manual)"| F["pre-release job"]
    E -->|"Mainnet<br/>(manual only)"| G["promote job"]

    F --> F1["Read version<br/>from Cargo.toml"]
    F1 --> F2["Create GitHub<br/>Pre-Release"]
    F2 --> H["deploy-testnet<br/>(workflow_call)"]

    G --> G1["Validate release tag"]
    G1 --> G2["Promote to<br/>full release"]
    G2 --> I["deploy-mainnet<br/>(workflow_call)"]

    H --> T1["Lint + Test"]
    T1 --> T2["Build Canisters"]
    T2 --> T3["Build npm"]
    T3 --> T4["Deploy Provider"]
    T4 --> T5["Verify Deployment"]
    T5 --> T6["Deploy Test Canisters<br/>(if debug)"]
    T6 --> T7["Publish npm<br/>(dry run)"]

    I --> M1["Lint + Test"]
    M1 --> M2["Build Canisters"]
    M2 --> M3["Build npm"]
    M3 --> M4["Deploy Provider<br/>(requires approval)"]
    M4 --> M5["Verify Deployment"]
    M5 --> M6["Publish npm"]
    M6 --> M7["Update Release Notes"]
```

## CI Pipelines

```mermaid
flowchart LR
    subgraph "CI (Pull Requests)"
        CI1["ci-devenv.yaml<br/>Devenv health check"]
        CI2["ci.yml<br/>Lint + Test + Build"]
    end

    subgraph "CD (Releases)"
        CD1["cd-release.yml<br/>Version + tag"]
        CD2["cd-testnet.yaml<br/>Full testnet deploy"]
        CD3["cd-mainnet.yaml<br/>Full mainnet deploy"]
    end

    CD1 -->|workflow_call| CD2
    CD1 -->|workflow_call| CD3
```

## Workflows

| Workflow          | Trigger                      | Description                                |
| ----------------- | ---------------------------- | ------------------------------------------ |
| `ci-devenv.yaml`  | PR, push                     | Validates devenv builds correctly          |
| `ci.yml`          | PR, push                     | Lint, test, and build canisters + npm      |
| `cd-release.yml`  | Push to trunk, manual        | Creates pre-release or promotes to release |
| `cd-testnet.yaml` | Called by cd-release, manual | Full testnet deployment pipeline           |
| `cd-mainnet.yaml` | Called by cd-release, manual | Full mainnet deployment pipeline           |

## Environments

| Environment | Purpose                        | Secrets                                           |
| ----------- | ------------------------------ | ------------------------------------------------- |
| **Testnet** | Staging deployments            | `DFX_IDENTITY_PEM`, `IC_SIWA_SALT`, Cachix tokens |
| **Mainnet** | Production (requires approval) | `DFX_IDENTITY_PEM`, `IC_SIWA_SALT`, Cachix tokens |

## Design Decisions

### workflow_call over event chaining

Deploy workflows are called directly via `workflow_call` rather than triggered by `release` events
(`prereleased`/`released`). This avoids [GitHub's `GITHUB_TOKEN` limitation](https://docs.github.com/en/actions/writing-workflows/choosing-when-your-workflow-runs/triggering-a-workflow#triggering-a-workflow-from-a-workflow)
where events created by `GITHUB_TOKEN` do not trigger other workflows.

Deploy workflows also support `workflow_dispatch` for manual re-runs independent of the release flow.

### Canister build matrix

Canisters are built in parallel using a matrix strategy. Artifacts are uploaded and shared
between the build and deploy jobs to avoid rebuilding.
