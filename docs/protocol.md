# Protocol

Binary WebSocket protocol (protobuf, `baylee-protocol`, wasm-safe).
Schema: `crates/baylee-protocol/proto/baylee/v1/transport.proto`.

## v0 (M0)
Transport handshake + preset transfer:
`Hello{protocol_version, card_pool_hash}` / `HelloAck`, `JoinGame`,
`ResumeGame{last_seq}`, `GamePresetMsg`, `Heartbeat`, `Error`, wrapped in
an `Envelope` oneof. Card references are `{card_index, print_ref}`;
`card_pool_hash` invalidates client caches.

## v1 (M3, shipped 2026-08-29)
`CreateGame` / `GameCreated`, `ChoiceRequest{game_id, seq,
pending_json}`, `PlayerActionMsg{game_id, seat_token, action_json}`,
`StateDelta` (reserved). **Design decision (documented):** the full
engine choice taxonomy (`Pending`, `PlayerAction`) travels as
**serde_json payloads inside the protobuf frames** — the wire stays
binary protobuf, but the taxonomy evolves without proto churn. A typed
protobuf mapping of the taxonomy is a protocol v2 item, together with
per-player hidden-information filtering (`FullView`/`Delta`), timers
(`TimeExtensionRequest/Vote/Result`), `SetAutomationRules`, dev-mode
`DevCommand`, and spectator streams.
Server: `baylee-engine-server` (tokio + tokio-tungstenite), one process,
games as `Session`s — engine + human seat + auto-driven AI seats
(baylee-ai). E2E smoke: real binary over a real socket
(`crates/baylee-engine-server/tests/e2e.rs`).
