# Test Rules

**Status:** Active
**Date:** 2026-06-01

## Abstract

This document governs how the proxy is tested. Tests are the executable specification of the proxy's contract. They are the primary way to verify that the rules in `doctrine/` are upheld.

## Terminology

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**,
**SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in this
document are to be interpreted as described in BCP 14, RFC 2119 and RFC 8174.

A *spec test* is a test whose input is a raw OpenAI API request body or MCP tool call, and whose expected output is the corresponding MCP tool response or OpenAI response. No implementation detail appears in the test body.

## Requirements

1. **Tests are the spec.** Every behavioral rule in `doctrine/` MUST have at least one corresponding spec test. A rule with no test is unverified.

2. **API-level inputs and outputs only.** Test bodies MUST express inputs and outputs exclusively as OpenAI API payloads or MCP API payloads — raw JSON. Internal structs, queue state, and implementation types MUST NOT appear in test assertions.

3. **Self-contained.** Each test MUST be readable in isolation. A reader MUST be able to understand what is being tested without reading any other test or any source file.

4. **Named for the rule.** Each test function MUST be named after the behavior it verifies, not the implementation mechanism (e.g., `simple_message_is_exposed_to_agent`, not `test_queue_push`).

5. **No mocks.** Tests MUST spin up the real proxy server in-process. The OpenAI endpoint and MCP server MUST both be live during the test. No component MAY be stubbed.

6. **Blocking tool calls are tested end-to-end.** Tests covering tool call flows MUST verify the full round-trip: agent calls tool → OpenAI caller receives tool_call response → codebase sends tool result → MCP tool call resolves with result.

7. **Queue ordering is tested with concurrent requests.** Tests covering queue behavior MUST send at least two requests concurrently and assert that the MCP agent sees them in arrival order.

8. **Conversation delta is tested explicitly.** Tests covering conversation continuity MUST assert that `read_message` returns only the new messages, not the full history, on subsequent turns of the same conversation.

9. **Each test covers exactly one rule.** A test MUST NOT assert multiple independent behaviors. If two rules need verification, write two tests.

## References

- `doctrine/proxy/RULES.md` — the flow being tested.
- `doctrine/openai/RULES.md` — OpenAI response shapes.
- `doctrine/mcp/RULES.md` — MCP tool shapes.
- `doctrine/queue/RULES.md` — queue and delta behavior.

## Changelog

| Date       | Change            |
|------------|-------------------|
| 2026-06-01 | Initial version.  |
