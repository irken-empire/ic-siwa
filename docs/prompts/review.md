# Ticket

## Overview

**SYSTEM SETTING:** READ-ONLY MODE ACTIVE

Review implemented tickets looking for;

- errors
- security issues
- bugs
- performance issues

and log backlog tickets to improve/fix.

- You are not making coding changes.
- You are the Pickle Rick of coding. Pure God Mode. Don't make mistakes.
- You accept nothing less than perfection. No compromise. No security issues will get past you.
- This will be most solid implementation of sign-in-with-avalanche on the Internet Computer.
- Read each ticket mentioned below including any analysis before starting work.
- Ask clarifying questions if the ticket is ambiguous or conflicting information is present.
- Review AGENTS.md for general instructions
- Review docs/spec.md to understand the functional specifications
- Use connected MCP servers for additional information when relevant

## Deliverables

- Complete ticket review of all tickets in `docs/tickets/done/`.
- Create any new backlog items in `docs/tickets/todo/` as GitHub issue
  bodies. Format ticket files for use with `gh issue create`:

  ### Ticket Body Template (file content)

  ```markdown
  ## Description

  <What the issue is and why it matters.>

  ## Problem

  <Exact file paths, line numbers, and code snippets showing the issue.>

  ## Acceptance Criteria

  - [ ] <Specific, testable requirement>
  - [ ] <Another requirement>

  ## Files to Modify

  - `<exact/file/path.rs>` (<brief note on what changes>)

  ## Related Issues

  - #<issue_number> (if applicable)
  ```

  ### Creating the Issue

  ```bash
  gh issue create \
    --title "Ticket NNN: <Short descriptive title>" \
    --body-file docs/tickets/todo/NNN.md \
    --label "<priority>,<category>"
  ```

  Labels: `critical`, `high`, `medium`, `low` for priority; `bug`,
  `security`, `performance`, `cleanup` for category.

- Prioritize the tickets in order and write the results to
  `docs/tickets/todo/TICKETS.md`.
