# FORGEOS-V1-NYX-200 Closure and Specification

Status: `CLOSED`
Validation: `OPERATOR_AND_REAL_SERVER_PROVED`
Cross-repo gate input: `NYX_GATE_INPUT.json`

## Bounded capability

ForgeOS consumes Nyx_Server's public model, session, conversation, message, and
ordered response-event APIs through a thin authenticated client. Nyx owns model
catalog truth, routing, execution, identities, persistence, and event production.

ForgeOS may construct requests, display returned state, retain transient selected
identity for the UI, and independently reject malformed or contradictory responses.
It does not keep a second canonical conversation ledger.

## Source slice

- optional validated bearer authentication in the shared Nyx HTTP transport;
- typed model, session, conversation, message, native response-event, and OpenAI-compatible stream DTOs;
- thin native lifecycle client for create, list, read, select, close, restore, and persistent message send;
- separate validated SSE consumer for Nyx's OpenAI-compatible streaming surface;
- strict identity, ordering, attribution, schema, content-type, stream-schema, and terminal-event validation;
- fixture-backed behavior tests and one ignored real-Nyx witness;
- exact Nyx gate receipts and ownership evidence.

## Required proof

```bash
python3 scripts/run_ci.py
```

The operator must also run the ignored witness against an independently running
Nyx_Server using the bounded deterministic mock backend. The witness must discover
the selected model, create a session, use its server-created conversation, consume
an ordered OpenAI-compatible SSE response through `[DONE]`, send a persistent native
message, read history, close the session, and restore the same identity.

## Explicit non-claims

This slice does not call a model runtime directly, choose a route around Nyx,
persist canonical conversation state, synthesize native or SSE events, own server
identities, or implement Nyx chat behavior inside ForgeOS.

## Closure evidence

- Forge behavior CI: 82 suites, 419 passed, 0 failed, 5 ignored.
- Real Nyx witness: passed against the independent Nyx process.
- Stream proof: contiguous sequences 0, 1, 2 ending in `response.completed` and `[DONE]`.
- Persistence proof: exact user and assistant messages remained bound to the Nyx-owned session and conversation.
- Restart proof: the same session identity restored successfully.
