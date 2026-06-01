# Test Rules

**Status:** Active
**Date:** 2026-06-01

## Abstract

This document governs how the proxy is tested. Tests are the executable specification of the proxy's contract.

## Terminology

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**,
**SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in this
document are to be interpreted as described in BCP 14, RFC 2119 and RFC 8174.

A *spec test* is a test whose input is a raw OpenAI API request body or MCP tool call, and whose expected output is the corresponding MCP tool response or OpenAI response.

## Requirements

1. **Tests are the spec.** Every behavioral rule in `doctrine/` MUST have at least one corresponding spec test. A rule with no test is unverified.

2. **API-level inputs and outputs only.** Test bodies MUST express inputs and outputs exclusively as OpenAI API payloads or MCP API payloads — raw JSON. Internal structs, queue state, and implementation types MUST NOT appear in test assertions.

3. **Self-contained.** Each test MUST be readable in isolation. A reader MUST be able to understand what is being tested without reading any other test or any source file.

4. **Named for the behavior.** Each test function MUST be named after the behavior it verifies, not the implementation mechanism.

5. **No stubs.** Tests MUST exercise the real proxy behavior. No component MAY be stubbed or mocked.

6. **Each test covers exactly one rule.** A test MUST NOT assert multiple independent behaviors. If two rules need verification, write two tests.

7. **Named variables for all test values.** Every input, target, or expected value MUST be assigned to a named variable before being passed to an assertion helper.

## References

- `doctrine/proxy/RULES.md` — the flow being tested.
- `doctrine/openai/RULES.md` — OpenAI response shapes.
- `doctrine/mcp/RULES.md` — MCP tool shapes.
- `doctrine/queue/RULES.md` — queue and delta behavior.

## Changelog

| Date       | Change                                                              |
|------------|---------------------------------------------------------------------|
| 2026-06-01 | Initial version.                                                    |
| 2026-06-02 | Removed method-prescriptive rules (concurrent requests, round-trip coverage, in-process requirement) — these describe how to test, not what a test must be. |
