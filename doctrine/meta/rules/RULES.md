# Meta Rules

**Status:** Active
**Date:** 2026-06-01

## Abstract

Reverse Proxy governs itself through `doctrine/`: a self-contained body of
**rules** (what a result must be) and **skills** (how to produce it). This
document specifies what a rule is, the form every `RULES.md` under `doctrine/`
must take, where rules live, and how they change. Skills are governed by
`doctrine/meta/skills/RULES.md`.

## Terminology

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**,
**SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in every
`RULES.md` under `doctrine/` are to be interpreted as described in BCP 14,
RFC 2119 and RFC 8174.

A *rule* constrains an outcome — a property observable in a finished artifact.
A *skill* describes a method — a procedure for producing an artifact.
A *topic* is a directory under `doctrine/` grouping the rules and skills for one area.

## Principles

1. **Rules are normative.** All contributions MUST comply with active rules; code that violates a rule MUST NOT be merged.

2. **Rules govern outcomes, not methods.** A rule MUST describe what must be true of a finished artifact, never the procedure to achieve it; a procedure is a skill.

3. **One home per rule.** Each rule MUST appear in exactly one `RULES.md`; a rule that applies repo-wide lives in the most general topic and MUST NOT be mirrored into specific ones.

4. **Topics may nest.** A topic's rules MUST live in `doctrine/<topic>/RULES.md`; where a nested topic's rule conflicts with an ancestor's, the nested (more specific) rule MUST govern.

5. **Format.** Every `RULES.md` MUST contain, in order: a title, a Status, a Date, an Abstract, a Terminology section, numbered normative requirements, a References section, and a Changelog.

6. **Requirements are addressable.** Each numbered requirement MUST be citable from anywhere in the tree as `doctrine/<topic>/RULES.md:<n>`.

7. **Status lifecycle.** Each `RULES.md` MUST declare a status of Draft (proposed), Active (enforced), or Superseded (replaced).

8. **Changelog required.** Every change to an active `RULES.md` MUST be recorded in its Changelog with a date and a summary.

9. **Revision, not silent drift.** A published requirement MUST NOT be silently removed or renumbered; it MUST be deprecated in place with a replacement reference where one applies.

## References

- RFC 2119, RFC 8174 (BCP 14).
- `doctrine/meta/skills/RULES.md` — the companion meta for skills.

## Changelog

| Date       | Change                                                        |
|------------|---------------------------------------------------------------|
| 2026-06-01 | Initial version.                                              |
