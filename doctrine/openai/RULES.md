# OpenAI Endpoint Rules

**Status:** Active
**Date:** 2026-06-01

## Abstract

This document governs the OpenAI-compatible HTTP endpoint exposed by the reverse proxy.

## Terminology

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**,
**SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in this
document are to be interpreted as described in BCP 14, RFC 2119 and RFC 8174.

## Requirements

1. **Accepted fields.** The endpoint MUST expose `POST /v1/chat/completions` and accept `messages`, `model`, and `tools`. Unknown fields MAY be accepted and ignored.

2. **Blocking response.** The endpoint MUST NOT return a response until the MCP-connected agent resolves the request. The HTTP connection is held open.

3. **Text response shape.** When the agent calls `write_message`, the proxy MUST return:
   ```json
   {
     "choices": [{
       "message": { "role": "assistant", "content": "<text>" },
       "finish_reason": "stop"
     }]
   }
   ```

4. **Tool call response shape.** When the agent calls a dynamic tool, the proxy MUST return:
   ```json
   {
     "choices": [{
       "message": {
         "role": "assistant",
         "content": null,
         "tool_calls": [{
           "type": "function",
           "function": { "name": "<name>", "arguments": "<json-string>" }
         }]
       },
       "finish_reason": "tool_calls"
     }]
   }
   ```

## References

- OpenAI Chat Completions API specification.
- `doctrine/RULES.md` — core principle 1 (agent is the LLM, no forwarding).
- `doctrine/mcp/RULES.md` — how the endpoint feeds the MCP server.
- `doctrine/queue/RULES.md` — how requests are queued.

## Changelog

| Date       | Change            |
|------------|-------------------|
| 2026-06-01 | Initial version.  |
