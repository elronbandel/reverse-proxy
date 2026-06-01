# Reverse Proxy Rules

**Status:** Active
**Date:** 2026-06-01

## Abstract

Reverse Proxy is a Rust server that intercepts OpenAI-compatible LLM calls and queues them for an MCP-connected agent to answer, making the agent act as the LLM for any OpenAI-compatible codebase. This document defines the project's core principles and is the root of the doctrine tree.

## Terminology

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**,
**SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in this
document are to be interpreted as described in BCP 14, RFC 2119 and RFC 8174.

## Core Principles

1. **The agent is the LLM.** The proxy MUST NOT forward requests to any upstream model. An MCP-connected agent is the sole source of responses.

2. **Drop-in compatibility.** Any codebase using the OpenAI SDK MUST work against this proxy by changing only the `base_url`. No other code change is required.

3. **FIFO queue.** Requests MUST be served to the MCP agent in arrival order, one at a time.

4. **Native tool exposure.** Tools declared in an OpenAI request MUST appear as first-class MCP tools, not wrapped in a generic call. The agent calls them the same way it calls any MCP tool.

5. **Delta delivery.** The agent MUST only receive messages it has not yet seen. The proxy tracks conversation history and sends only the new messages each turn.

6. **Synchronous tool round-trips.** When the agent calls a tool, that MCP call MUST block until the codebase sends back the tool result. The agent does not need to poll or re-read.

7. **Self-contained repository.** The repository MUST be the sole source of information about itself. Every rule, convention, and assumption MUST be documented in `doctrine/`.

8. **Clean code.** All code MUST be the simplest, most minimal implementation that satisfies the rules. No dead code, no premature abstractions, no unnecessary dependencies.

## Doctrine Map

| Topic | Document |
|-------|----------|
| Meta — rules form | [doctrine/meta/rules/RULES.md](meta/rules/RULES.md) |
| Meta — skills form | [doctrine/meta/skills/RULES.md](meta/skills/RULES.md) |
| OpenAI endpoint | [doctrine/openai/RULES.md](openai/RULES.md) |
| MCP server | [doctrine/mcp/RULES.md](mcp/RULES.md) |
| Queue and conversations | [doctrine/queue/RULES.md](queue/RULES.md) |
| Proxy flow | [doctrine/proxy/RULES.md](proxy/RULES.md) |
| Tests | [doctrine/tests/RULES.md](tests/RULES.md) |
| Code quality | [doctrine/code/RULES.md](code/RULES.md) |

## References

- RFC 2119, RFC 8174 (BCP 14).
- OpenAI Chat Completions API specification.
- MCP protocol specification.

## Changelog

| Date       | Change            |
|------------|-------------------|
| 2026-06-01 | Initial version.  |
| 2026-06-02 | Added doctrine/code/RULES.md for code quality rules. |
