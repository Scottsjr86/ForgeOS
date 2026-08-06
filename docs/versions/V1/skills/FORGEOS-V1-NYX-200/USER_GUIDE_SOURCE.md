# FORGEOS-V1-NYX-200 User Guide Source

Status: `ACTIVE`
Validation: `OPERATOR_VALIDATION_PENDING`

ForgeOS can authenticate to Nyx, list Nyx-provided models, create and restore Nyx
sessions, select or close conversations, send persistent native messages, consume
Nyx's validated OpenAI-compatible SSE stream, and display the exact ordered response
events and model attribution returned by Nyx.

A missing model remains a clear Nyx server error. Foreign session or conversation
identity, duplicate or reordered lists, malformed schemas, contradictory model
attribution, invalid native terminal-event sequences, malformed SSE frames, missing
stream-schema headers, sequence gaps, and missing `[DONE]` termination fail closed.

Nyx_Server remains independently usable for chat, development, tools, agents, and
other API clients. ForgeOS does not own Nyx's model registry or conversation store.
