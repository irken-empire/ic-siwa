# Claude Code Instructions

## Required Reading

Before starting any task, read the following files for project context:

- `AGENTS.md` - Detailed agent instructions, technology stack, coding guidelines, and workflows
- `docs/spec.md` - Functional specifications for SIWA authentication

## Quick Reference

- **Package Manager**: Use `bun` exclusively (not npm/npx)
- **Environment**: All commands run in `devenv shell`
- **Helper Script**: Use `ic-siwa` (or `./scripts/ic-siwa.sh`) for common tasks

## Key Commands

```bash
ic-siwa build         # Build all canisters and libraries
ic-siwa deploy        # Deploy to local dfx replica
ic-siwa deploy --network juno  # Deploy to Juno emulator
ic-siwa test          # Run tests
ic-siwa logs          # Tail canister logs in real-time
ic-siwa loop          # Full dev loop: fmt, lint, build, test, deploy
ic-siwa agent-docs    # Download LLM documentation to docs/agents/
ic-siwa help          # Show all available commands
```

## AI Agent Documentation

Run `ic-siwa agent-docs` to download LLM-optimized documentation for the technologies used in this project. Files are saved to `docs/agents/` and include:

- DaisyUI (UI components)
- Astro (frontend framework)
- Juno (deployment platform)
- Oisy (IC wallet integration)

Reference these files for detailed context about specific technologies.
