# Cross-gateway ranked duels

## Admission and transport

Players keep both home lobbies. Central chooses the game hosts, shows their operator names and current terms, and waits for both players to approve the recipients. The game client uses Central for the deck registration, seat grant, game socket and reconnect. A replacement operator requires consent before receiving private data. An authenticated participant can view their opponent's published handle through Central. Every host receives separate match-scoped seat aliases and the playable deck registrations. The operators see the cards they have to simulate. Timing and publicly visible play can still identify a player.

The relayed ranked socket is separate from ordinary lobby and casual sockets. Its fixed Central path accepts a single-use capability in the first binary frame. The outgoing connection originates at Central. Hosts receive fresh requests assembled from the match assignment. They receive neither the player's IP forwarding headers nor their ordinary Central or home-gateway session. [Native transport security](../native-transport-security.md)

## Replication and failover

Central can reserve two verified hosts with matching engine artifacts, independent registered owners and spare capacity. Each runs its own game-engine session. Protected matches use a Central-generated seed, a frozen playable opening, a journal of independently authenticated seat decisions and two engine votes. The primary releases the agreed seat projection. A disagreement pauses progress and prevents settlement. Single-host play is possible at initial assignment when there is no compatible pair, and the client identifies that mode. A protected game keeps its verification requirement throughout recovery.

A command UUID and an expected journal position make a player's retry identifiable across disconnects. Central owns decision and reconnect deadlines. A caught-up standby can take over under a new generation. The new pair needs current leases, compatible execution and operator approvals before releasing more play. The game clock pauses during recovery. A recovered former primary cannot score, publish moves or void the new generation. Central settles one outcome and issues one receipt per participant. [Ranked protocol](../protocol.md#gateway-hosted-ranked-duels)

## Reproducing the checks

The engine regression replays the same archived Commander game in two sessions, including a delayed session that catches up. Network coverage in the Central integration harness exercises independently running gateway and engine processes, consent, private seat views, replay and interrupted play. See the integration command for its local PostgreSQL prerequisites. [Twin-engine regression](../../crates/baylee-gamehost/src/session/tests/twins.rs) · [Central integration harness](../../crates/baylee-gateway/tests/e2e_central/ranked/replication.rs) · [Development commands](../dev/commands.md)

Redundancy is between gateways. Central connectivity is required for mirrored progress. Two agreeing engines can share the same game-rules defect. Distinct registered operators can also share infrastructure. Operators should measure inter-region latency and relay headroom at peak load before a public rollout.
