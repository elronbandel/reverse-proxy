# AGENTS.md

Reverse Proxy is governed by **`doctrine/`** — a self-contained body of
**rules** (what a result must be) and **skills** (how to produce it). Before
you change anything here, read the doctrine for the area you're touching and
treat its rules as binding: code that violates an active rule must not merge.

## How the doctrine works

- A **rule** states an *outcome* a finished artifact must have. Rules are normative.
- A **skill** is a *procedure* you can follow to produce a conforming result.
- Form, placement, and lifecycle of rules and skills are fixed by [`doctrine/meta/`](doctrine/meta/).

## Map

**Meta (the constitution)**
- [`doctrine/meta/rules/RULES.md`](doctrine/meta/rules/RULES.md) — what a rule is and how rules are written.
- [`doctrine/meta/skills/RULES.md`](doctrine/meta/skills/RULES.md) — what a skill is and how skills are written.

**Rules — the *what***
- [`doctrine/RULES.md`](doctrine/RULES.md) — core principles; read this first for any task.
- [`doctrine/openai/RULES.md`](doctrine/openai/RULES.md) — OpenAI-compatible endpoint contract.
- [`doctrine/mcp/RULES.md`](doctrine/mcp/RULES.md) — MCP server tool exposure and behavior.
- [`doctrine/queue/RULES.md`](doctrine/queue/RULES.md) — queue ordering, conversation continuity, delta delivery.
- [`doctrine/proxy/RULES.md`](doctrine/proxy/RULES.md) — end-to-end request flow.
- [`doctrine/tests/RULES.md`](doctrine/tests/RULES.md) — test structure and coverage requirements.

## Working in this repo

1. **Find the topic(s)** your change touches and read their `RULES.md` plus every ancestor up to `doctrine/RULES.md`.
2. **If a skill exists** for what you're doing, follow it.
3. **Before opening a PR**, verify your change against the relevant rules.
4. **Changing a rule** means editing it in its one home, adding a Changelog entry, and citing it in your PR. Never encode standards anywhere but `doctrine/`.
