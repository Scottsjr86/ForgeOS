# FORGEOS-V1-AGENT-100 User Guide Source

Status: `ACTIVE`
Validation: `OPERATOR_VALIDATION_PENDING`

ForgeOS can submit an exact remote-agent request to Nyx and display the returned
Nyx-owned task record. The record includes provider, model, source revision,
worktree, allowed paths, budget, current state, output or failure, execution counts,
recorded provider cost, and the immutable task/run identities.

ForgeOS can cancel or continue only the exact queued record it received. A changed
task ID, run ID, request hash, source binding, scope, budget, or route fails closed.
Terminal records cannot be presented as resumable.

Nyx_Server remains independently usable for chat, development, tools, and other API
clients. ForgeOS does not own Nyx's run database and does not call providers around
Nyx.
