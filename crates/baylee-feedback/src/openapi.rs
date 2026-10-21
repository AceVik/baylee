//! The feedback service's API document: its root, its security schemes and
//! the refusals many routes share. Nothing here runs; `cargo dev api`
//! (`tools/api-docs`) reads these blocks with every route's own and writes
//! `docs/api/feedback.openapi.yaml`. `docs/feedback.md` stays the normative
//! description; the document is what a client author reads first.
//!
//! @openapi
//! @scope feedback
//! ```yaml
//! openapi: 3.1.0
//! info:
//!   title: Baylee feedback service
//!   version: '{{CARGO_PKG_VERSION}}'
//!   description: >-
//!     The service that keeps what players report: gateways pass reports on
//!     with their intake tokens, clients signed in nowhere send their own, and
//!     admins read and triage them, with tokens or in the service's web UI. The
//!     UI's admin pages also reach one gateway's admin console through it, and
//!     the landing page posts its contact form and fetches its Impressum here.
//!     Every JSON answer is `Cache-Control: no-store`; a refusal is an
//!     `ErrorBody`.
//!   license:
//!     name: AGPL-3.0-only
//!     identifier: AGPL-3.0-only
//! servers:
//!   - url: http://127.0.0.1:28780
//!     description: A feedback service on this machine (`FEEDBACK_BIND`)
//! tags:
//!   - { name: Service, description: Whether the service is well }
//!   - { name: Intake, description: 'Reports a gateway passes on, with its intake token' }
//!   - { name: Clients, description: 'What a client sends itself, with no credential: a report from a client signed in nowhere, and the anonymous offline count' }
//!   - { name: Site, description: 'The landing page: its contact form and its Impressum' }
//!   - { name: Reports, description: Reading and changing reports with the read or admin token }
//!   - { name: Sessions, description: "Signing in to the web UI and out again" }
//!   - { name: Triage, description: "The web UI's reports: the list, one report, its notes, the trash, saved views" }
//!   - { name: Contact, description: "The web UI's contact messages from the landing page" }
//!   - { name: Console, description: "The web UI's admin pages, passed on to one gateway's admin console" }
//!   - { name: Moderation, description: 'Acting on a gateway''s account or game, through its admin console' }
//!   - { name: Models, description: "Hosted language models and who may seat them, through the gateway's admin console" }
//! components:
//!   securitySchemes:
//!     gatewayToken:
//!       type: http
//!       scheme: bearer
//!       description: 'A gateway''s intake token, one of `FEEDBACK_GATEWAY_TOKENS`. The report is filed under the name that token is configured with.'
//!     readToken:
//!       type: http
//!       scheme: bearer
//!       description: '`FEEDBACK_READ_TOKEN`: reads reports.'
//!     adminToken:
//!       type: http
//!       scheme: bearer
//!       description: '`FEEDBACK_ADMIN_TOKEN`: reads reports, changes their status and deletes them.'
//!     session:
//!       type: apiKey
//!       in: cookie
//!       name: __Host-baylee-feedback
//!       description: 'The web UI''s session, set by `POST /ui/api/login` (`HttpOnly; Secure; SameSite=Strict; Path=/`). It ends 12 hours unused or 7 days after sign-in. The tokens open none of the `/ui/api` routes, and a session none of the token routes.'
//!     csrf:
//!       type: apiKey
//!       in: header
//!       name: X-Baylee-CSRF
//!       description: 'Must be `1` on every `/ui/api` request that changes something, sign-in and sign-out included. Such a request must also carry an `Origin` naming the request''s `Host`, and must not be marked `Sec-Fetch-Site` other than `same-origin`.'
//! ```
//!
//! @openapi-fragment `ReadRefusals`
//! @return 401: `$ErrorBody` "Neither the read token nor the admin token"
//! @return 503: `$ErrorBody` "The database is unavailable"
//!
//! @openapi-fragment `AdminTokenRefusals`
//! @return 401: `$ErrorBody` "Not the admin token"
//! @return 503: `$ErrorBody` "The database is unavailable"
//!
//! @openapi-fragment `SessionRefusals`
//! @return 401: `$ErrorBody` "No live session: signed out, or the session lapsed"
//! @return 503: `$ErrorBody` "The database is unavailable"
//!
//! @openapi-fragment `SessionChangeRefusals`
//! @return 401: `$ErrorBody` "No live session: signed out, or the session lapsed"
//! @return 403: `$ErrorBody` "A cross-site request: no `X-Baylee-CSRF: 1`, an `Origin` that is not this host, or `Sec-Fetch-Site` other than `same-origin`"
//! @return 503: `$ErrorBody` "The database is unavailable"
//!
//! @openapi-fragment `ConsoleRefusals`
//! @return 401: `$ErrorBody` "No live session: signed out, or the session lapsed"
//! @return 404: `$ErrorBody` "No gateway console is configured for this service (`FEEDBACK_GATEWAY_ADMIN_URL`)"
//! @return 502: `$ErrorBody` "The gateway refused this service's console token, did not answer within 10 seconds, or answered with a status or body this service does not pass on"
//! @return 503: `$ErrorBody` "The database is unavailable, the gateway's console is shut after too many wrong tokens, or the gateway's own `503`"
//!
//! @openapi-fragment `ConsoleChangeRefusals`
//! @return 401: `$ErrorBody` "No live session: signed out, or the session lapsed"
//! @return 403: `$ErrorBody` "A cross-site request: no `X-Baylee-CSRF: 1`, an `Origin` that is not this host, or `Sec-Fetch-Site` other than `same-origin`"
//! @return 404: `$ErrorBody` "No gateway console is configured for this service (`FEEDBACK_GATEWAY_ADMIN_URL`)"
//! @return 502: `$ErrorBody` "The gateway refused this service's console token, did not answer within 10 seconds, or answered with a status or body this service does not pass on"
//! @return 503: `$ErrorBody` "The database is unavailable, the gateway's console is shut after too many wrong tokens, or the gateway's own `503`"
