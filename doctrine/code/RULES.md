# Code Rules

**Status:** Active
**Date:** 2026-06-02

## Abstract

This document governs the quality of all code in the repository — production and test. The goal is the simplest, most readable implementation that satisfies the behavioral rules.

## Terminology

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**,
**SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in this
document are to be interpreted as described in BCP 14, RFC 2119 and RFC 8174.

## Requirements

1. **Minimal implementation.** Every function, type, and module MUST be the simplest possible implementation that satisfies the rules. No code MAY exist for hypothetical future needs.

2. **No dead code.** Unused functions, types, variables, and imports MUST NOT exist.

3. **No premature abstraction.** An abstraction MUST NOT be introduced until it is needed by at least two concrete callsites. Three similar lines are better than a premature helper.

4. **No unnecessary comments.** Comments MUST NOT restate what the code already says. A comment is only justified when the WHY is non-obvious — a hidden constraint, a subtle invariant, or a known workaround.

5. **Meaningful names.** Every identifier MUST describe what it is or does. Single-letter names are only permitted for loop indices and closures where the type is immediately obvious from context.

6. **No nested complexity.** Functions MUST NOT contain deeply nested control flow. Extract a helper when nesting exceeds three levels.

7. **Tests are spec, not scaffolding.** Every test MUST assert at least one behavioral property. A test that always passes regardless of implementation is not a test.

8. **Test inputs and outputs are plain values.** Test bodies MUST express inputs and expected outputs as inline literals (`json!({...})`, strings, slices). Builder functions and computed expected values MUST NOT appear in test bodies — only in test helpers.

9. **One assertion per test.** Each test MUST cover exactly one behavioral rule. Tests that assert multiple independent behaviors MUST be split.

10. **Named variables for all test values.** Every input, target, or expected value in a test MUST be assigned to a named variable before being passed to an assertion helper. Inline literals in function call arguments are not permitted.

## References

- `doctrine/RULES.md` — principle 8 (Clean code).
- `doctrine/tests/RULES.md` — test-specific rules.

## Changelog

| Date       | Change            |
|------------|-------------------|
| 2026-06-02 | Initial version.  |
