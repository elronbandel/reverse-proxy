# Proxy Flow Rules

**Status:** Active
**Date:** 2026-06-01

## Abstract

This document governs the end-to-end runtime flow of the proxy server — how a request enters, moves through the system, and produces a response.

## Terminology

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**,
**SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in this
document are to be interpreted as described in BCP 14, RFC 2119 and RFC 8174.

A *turn* is the full lifecycle of one request from arrival to final response.

## Requirements

1. **Single binary.** The proxy MUST run as a single Rust binary exposing both the OpenAI endpoint and the MCP server on configurable ports.

2. **Turn lifecycle.** Every turn MUST follow this sequence:
   1. Request arrives at `/v1/chat/completions` and is enqueued.
   2. When it reaches the front of the queue, the MCP tool list is updated.
   3. The agent calls `read_message` and receives the delta.
   4. The agent calls `write_message` or a dynamic tool.
   5. The proxy sends the HTTP response to the caller.
   6. If a tool was called: wait for the codebase to send the tool result, resolve the blocked MCP call with the result, go to step 4.
   7. If `write_message` was called: turn is complete; next queue entry becomes active.

3. **No state leakage.** Tool schemas, conversation snapshots, and pending response channels from a completed turn MUST NOT be visible in the next turn.

4. **Concurrent ingestion.** The proxy MUST accept new requests while a turn is in progress. Callers MUST NOT be blocked at the HTTP layer waiting for a prior turn to complete.

5. **Async runtime.** The implementation MUST use async I/O and MUST NOT block the async runtime.

## References

- `doctrine/RULES.md` — core principles 1, 3, 8.
- `doctrine/openai/RULES.md`, `doctrine/mcp/RULES.md`, `doctrine/queue/RULES.md`.

## Changelog

| Date       | Change            |
|------------|-------------------|
| 2026-06-01 | Initial version.  |
