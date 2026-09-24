# The game server is written in Rust, the client in TypeScript/React

The Narrator runs on an authoritative server written in Rust (axum + tokio, with the official `livekit-api` crate to mint tokens and drive call permissions), while the phone client is TypeScript + React. Rust was chosen over an all-TypeScript stack because its strictness suits agent-written code; we checked that the Rust LiveKit SDK covers everything the Node one does (tokens, participant permissions, subscriptions, track muting, webhooks) before committing.

## Consequences

- Two languages in the repo; client/server message types must be kept in sync across the boundary (worth generating from one source).
- `livekit-api` is pre-1.0 with thin docs: pin the version and use LiveKit's Node/Go docs as reference.
