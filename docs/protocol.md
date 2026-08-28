# Protocol

Binary WebSocket protocol (protobuf, `baylee-protocol`, wasm-safe).
Schema: `crates/baylee-protocol/proto/baylee/v1/transport.proto`.

## v0 (M0)
Transport handshake + preset transfer:
`Hello{protocol_version, card_pool_hash}` / `HelloAck`, `JoinGame`,
`ResumeGame{last_seq}`, `GamePresetMsg`, `Heartbeat`, `Error`, wrapped in
an `Envelope` oneof. Card references are `{card_index, print_ref}`;
`card_pool_hash` invalidates client caches.

## v1 (M1–M3, planned)
`FullView` / `Delta{seq, events, changed_views}` (per-player
hidden-information filtered), `ChoiceRequestMsg` / `PlayerActionMsg`
mirroring the engine choice taxonomy (Priority, ChooseObjects, ChooseCards
[server-side filtered], ChooseModes, ChooseTargets, PayMana, PayCostParts,
OrderObjects, ChooseNumber, ChooseColor/Type/Subtype, Mulligan,
CombatDamageAssignment, YesNo), `SetAutomationRules`, timer messages
(`TimeExtensionRequest/Vote/Result`), dev-mode `DevCommand`, `GameOver`.
Spectator/replay streams reuse `Delta`.
