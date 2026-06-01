# MCP Server Rules

**Status:** Active
**Date:** 2026-06-01

## Abstract

This document governs the MCP server exposed by the reverse proxy — the interface through which an agent acts as the LLM for queued OpenAI requests.

## Terminology

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**,
**SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in this
document are to be interpreted as described in BCP 14, RFC 2119 and RFC 8174.

## Requirements

1. **Tool list composition.** The MCP server MUST expose exactly: `read_message`, `write_message`, and one tool per function in the current pending request's `tools` array.

2. **`tools/list_changed` on new conversation only.** When the next queued request is a new conversation (see `doctrine/queue/RULES.md:5`), the server MUST emit `notifications/tools/list_changed`. It MUST NOT emit it for continuations of the same conversation.

3. **`read_message`.** Returns `{ conversation_id, messages }` where `messages` is the delta for this conversation (see `doctrine/queue/RULES.md:4`).

4. **`write_message`.** Accepts a `content` string. Resolves the blocked `/v1/chat/completions` call with `finish_reason: stop` and advances the queue.

5. **Dynamic tools.** Each dynamic tool MUST mirror the name, description, and input schema of its corresponding OpenAI function, converted to MCP `input_schema` format. Calling it MUST resolve the blocked `/v1/chat/completions` call with `finish_reason: tool_calls`. The call MUST block until the codebase returns the tool result as the next request in the same conversation, then resolve with the tool result content.

6. **MCP compliance.** The server MUST conform to the MCP protocol specification.

## References

- MCP protocol specification.
- `doctrine/RULES.md` — core principles 4 (native tool exposure), 5 (delta delivery), 6 (synchronous tool round-trips).
- `doctrine/openai/RULES.md` — response shapes.
- `doctrine/queue/RULES.md` — conversation tracking and delta.

## Changelog

| Date       | Change            |
|------------|-------------------|
| 2026-06-01 | Initial version.  |
