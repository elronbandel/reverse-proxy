# Meta Skills Rules

**Status:** Active
**Date:** 2026-06-01

## Abstract

This document specifies what a skill is, the form every `SKILL.md` under `doctrine/` must take, and how skills change.

## Terminology

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**,
**SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in this
document are to be interpreted as described in BCP 14, RFC 2119 and RFC 8174.

A *skill* is a procedure — a sequence of steps an agent or human follows to produce a result. It does not constrain what the result must be; that is a rule.

## Principles

1. **Skills describe method, not outcome.** A skill MUST describe how to do something. What the result must be is a rule.

2. **Format.** Every `SKILL.md` MUST contain, in order: a title, a Status, a Date, an Abstract, numbered steps, a References section, and a Changelog.

3. **One skill per file.** Each skill MUST live in its own `SKILL.md` under `doctrine/<topic>/<skill-name>/SKILL.md`.

4. **Steps are executable.** Each step MUST be concrete enough for an agent to execute without ambiguity.

5. **Changelog required.** Every change to an active `SKILL.md` MUST be recorded in its Changelog.

## References

- `doctrine/meta/rules/RULES.md` — the companion meta for rules.

## Changelog

| Date       | Change            |
|------------|-------------------|
| 2026-06-01 | Initial version.  |
