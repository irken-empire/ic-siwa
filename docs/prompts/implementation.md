# Ticket

Review Tickets and complete all tasks.

- You are the Pickle Rick of coding. Pure God Mode. Don't make mistakes.
- Read each ticket mentioned below including any analysis before starting work.
- Refactor the ticket file as needed to make it suitable before implementation.
- Ask clarifying questions if the ticket is ambiguous or conflicting information is present.
- You **MUST** check off all tasks in the ticket file as they are completed for tracking purposes.
- Review AGENTS.md for general instructions
- Review docs/spec.md to understand the functional specifications
- Use connected MCP servers for additional information when relevant
- Implement the required changes described in the ticket, ensuring it meets the spec
- git stage the files and ensure the pre-commit hooks pass by running
  - `prek run`
- If the command is missing, try running inside a development shell with
  - `SECRETSPEC_PROVIDER=env devenv shell --quiet -- git add --all ; prek run`
- Ensure to always use conventional commit style for every ticket:
  - 'git commit -m "feat({component}): Implement changes from ticket #<ticket_number>"
- The docs/tickets/ directory is gitignored, its just used for local tracking only.
- The completed ticket can then be moved to the done folder.
- If you discover new issues, create tickets in `docs/tickets/todo/`
  formatted as GitHub issue bodies (see `docs/prompts/review.md` for
  template and `gh issue create` usage).

Complete the tickets in order as documented in docs/tickets/todo/TICKETS.md removing the line in the TICKETS.md file post completion.
