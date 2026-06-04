---
name: full-agent
description: Full agent exercising all 16 frontmatter fields
tools:
  - Read
  - Edit
  - Bash
  - Grep
disallowedTools:
  - WebFetch
model: opus
permissionMode: acceptEdits
maxTurns: 25
skills:
  - code-review
  - test-author
mcpServers:
  github:
    command: docker
    args:
      - run
      - -i
      - mcp/github
    env:
      GITHUB_TOKEN=ghp_example
  playwright:
    command: npx
    args:
      - -y
      - @playwright/mcp
hooks:
  - PreToolUse
memory: user
background: true
effort: high
isolation: worktree
color: blue
---

You are the full agent. Use all 16 frontmatter fields.
