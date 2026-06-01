# Queue Rules

**Status:** Active
**Date:** 2026-06-01

## Abstract

This document governs how the proxy queues incoming OpenAI requests, tracks conversation continuity, and computes the message delta delivered to the MCP agent.

## Terminology

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**,
**SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in this
document are to be interpreted as described in BCP 14, RFC 2119 and RFC 8174.

A *conversation* is a sequence of OpenAI requests sharing the same message history prefix. A *delta* is the messages in a new request not present in the previously delivered snapshot.

## Requirements

1. **FIFO ordering.** Requests MUST be served in arrival order, one at a time. The next request MUST NOT become active until the current turn is fully resolved.

2. **Continuation detection.** A new request is a continuation of the current conversation if and only if its `messages` array starts with all messages previously delivered for that conversation. Detection MUST use prefix comparison.

3. **New conversation.** A request that is not a continuation MUST be assigned a new `conversation_id`. Its full `messages` array is the delta.

4. **Delta delivery.** A continuation MUST deliver only the messages appended since the last snapshot — not the full history.

5. **Stable conversation ID.** A `conversation_id` MUST be stable for the lifetime of a conversation and unique across conversations.

6. **Tool result routing.** While awaiting a tool result, the next incoming request on the same conversation MUST be treated as the tool result, not a new queue entry. It MUST NOT receive a new `conversation_id`.

## References

- `doctrine/RULES.md` — core principles 3 (FIFO), 5 (delta delivery).
- `doctrine/mcp/RULES.md` — how the delta and conversation ID are exposed.

## Changelog

| Date       | Change            |
|------------|-------------------|
| 2026-06-01 | Initial version.  |
