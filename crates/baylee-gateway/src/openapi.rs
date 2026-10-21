//! The gateway's API document: its root, its security scheme and the
//! refusals many routes share. Nothing here runs; `cargo dev api`
//! (`crates/baylee-api-docs`) reads these blocks with every route's own and writes
//! `docs/api/gateway.openapi.yaml`. `docs/protocol.md` stays the normative
//! description; the document is what a client author reads first.
//!
//! @openapi
//! @scope public
//! ```yaml
//! openapi: 3.1.0
//! info:
//!   title: Baylee gateway
//!   version: '{{CARGO_PKG_VERSION}}'
//!   description: >-
//!     A Baylee gateway: accounts, decks, the lobby, and the games its agents
//!     host. The gateway runs no rules; a game's engine speaks to its seats
//!     over the WebSocket routes, which this document lists but does not
//!     describe frame by frame (`docs/protocol.md`).
//!   license:
//!     name: AGPL-3.0-only
//!     identifier: AGPL-3.0-only
//! servers:
//!   - url: http://127.0.0.1:28766
//!     description: A gateway on this machine (`PORT`, default 28766). The same routes are served on `BAYLEE_UNIX_SOCKET` when it is set.
//! tags:
//!   - name: Gateway
//!     description: 'What the gateway is and whether it is well'
//!   - name: Accounts
//!     description: 'Signing in and out, guests, the account and its settings'
//!   - name: Terms
//!     description: 'The terms of use and their acceptance'
//!   - name: Lobby
//!     description: 'Rooms, seats and the games the lobby lists'
//!   - name: Chairs
//!     description: 'A host''s chair for a seat bridge, and hosted model seats'
//!   - name: Decks
//!     description: 'A player''s decks and their pictures'
//!   - name: Catalog
//!     description: 'Card text, search, printings and card images'
//!   - name: Replays
//!     description: 'Replays of hosted games'
//!   - name: Social
//!     description: 'Friends, chat devices and sealed messages'
//!   - name: Central
//!     description: 'Baylee Central accounts at this gateway'
//!   - name: Reports
//!     description: 'Bug reports a client sends on'
//! components:
//!   securitySchemes:
//!     bearer:
//!       type: http
//!       scheme: bearer
//!       description: The session token a sign-in answers. Sockets take a single-use ticket bought with it instead (`POST /ws-ticket`).
//!     seatToken:
//!       type: http
//!       scheme: bearer
//!       description: 'A seat token: the `seat_token` that creating, joining or taking a seat, a rematch or a chair redemption answers. It proves one chair of one game.'
//!     chairTicket:
//!       type: http
//!       scheme: bearer
//!       description: 'A host''s single-use chair ticket (`POST /lobby/games/{id}/chairs/{seat}/ticket`), which a seat bridge redeems.'
//! ```
//!
//! @openapi
//! @scope admin
//! ```yaml
//! openapi: 3.1.0
//! info:
//!   title: Baylee gateway admin console
//!   version: '{{CARGO_PKG_VERSION}}'
//!   description: >-
//!     A gateway's admin console: counts, accounts, beta keys, card blocks,
//!     hosted models and the terms. It listens on its own loopback address
//!     only, never on the public port or the unix socket, and refuses a
//!     browser's request before it reads the token (`docs/protocol.md`
//!     §"The admin console").
//!   license:
//!     name: AGPL-3.0-only
//!     identifier: AGPL-3.0-only
//! servers:
//!   - url: http://127.0.0.1:28767
//!     description: The admin listener (`BAYLEE_ADMIN_BIND`, loopback only).
//! tags:
//!   - name: Admin
//!     description: 'The admin console'
//! components:
//!   securitySchemes:
//!     adminToken:
//!       type: http
//!       scheme: bearer
//!       description: '`BAYLEE_ADMIN_TOKEN`, on the admin listener only. A change also names its admin in `X-Baylee-Admin`.'
//! ```
//!
//! @openapi-fragment `SessionRefusals`
//! @return 401: `$ErrorBody` "No bearer token, or one that is invalid or expired"
//! @return 403: `$ErrorBody` "The account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
//! @return 503: `$ErrorBody` "The account store or Baylee Central is unavailable"
//!
//! @openapi-fragment `SessionRefusalsAnyTerms`
//! @return 401: `$ErrorBody` "No bearer token, or one that is invalid or expired"
//! @return 403: `$ErrorBody` "The account is banned (`ban`)"
//! @return 503: `$ErrorBody` "The account store or Baylee Central is unavailable"
//!
//! @asyncapi
//! @scope public
//! ```yaml
//! asyncapi: 3.0.0
//! info:
//!   title: Baylee gateway sockets
//!   version: '{{CARGO_PKG_VERSION}}'
//!   description: >-
//!     The gateway's WebSockets. A seat's socket carries protobuf `Envelope`s
//!     the gateway forwards to the game's engine and back without decoding
//!     them; the lobby feed, the social socket and the seat agents' socket
//!     speak JSON. A client buys a single-use ticket for a socket over
//!     `POST /ws-ticket` and opens it with `?ticket=`; agents and engines
//!     name their secret in their first frame instead (`docs/protocol.md`).
//!   license:
//!     name: AGPL-3.0-only
//!     identifier: AGPL-3.0-only
//! servers:
//!   local:
//!     host: 127.0.0.1:28766
//!     protocol: ws
//!     description: A gateway on this machine (`PORT`, default 28766).
//! ```
