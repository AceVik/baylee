# The feedback service's web UI (#311)

The page an admin triages player reports on, at
`https://feedback.baylee.acevik.de/`. It is a React app built to static
files that `baylee-feedback` serves itself from `FEEDBACK_WEB_DIR`, and it
talks only to that service's `/ui/api/…` routes (`docs/feedback.md` §"The
web UI"). `cargo build` never needs node: without a build, or with
`FEEDBACK_WEB_DIR` unset, the service simply serves no page.

```sh
npm ci
npm run typecheck    # tsc --noEmit, app and e2e
npm run lint         # oxlint
npm test             # vitest + Testing Library, jsdom
npm run build        # dist/, what FEEDBACK_WEB_DIR points at
npm run check        # all four, as CI runs them

# End to end, against a real service, a real gateway and PostgreSQL (not in CI):
cargo build -p baylee-feedback -p baylee-gateway
npm run build
npx playwright install chromium          # once
DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee npm run e2e
SCREENSHOTS_DIR=/tmp/shots DATABASE_URL=… npm run e2e -- -g screenshots
DATABASE_URL=… npm run serve:e2e         # the seeded service, to look at by hand
```

The end-to-end harness (`e2e/harness.ts`) makes a schema of its own, adds an
admin through the binary's `admin add`, starts a `baylee-gateway` in a
second schema as a closed beta with its admin console on a loopback port
(two players let in with keys its `invite create` made), starts the service
with `FEEDBACK_WEB_DIR=dist` and that console, hands reports in through
`/intake/reports` as a gateway does, and drops both schemas afterwards.
`e2e/admin.spec.ts` drives the console at 360×740 and 740×360 (touch, the
first in German) and 1280×800, and fails on a page or a card wider than its
screen and on a control under 44 px; with `SCREENSHOTS_DIR` it saves each.

## TypeScript

`typescript` is pinned to **7.0.2** (the native compiler) in `package.json`,
the one place its version is written. The owner asked for TypeScript 8; it is
not released yet, and moving to it is that one line once it is. The settings
are the strictest there are (`strict`, `noUncheckedIndexedAccess`,
`exactOptionalPropertyTypes`, `noImplicitOverride`,
`noPropertyAccessFromIndexSignature`, `verbatimModuleSyntax`, …).
`typescript-eslint` does not accept TypeScript 7 (its peer range ends below
6.1), so linting is `oxlint`, which needs no compiler.

## What it may load

Everything is bundled and served from the service's own origin: no CDN, no
web font (system fonts only), no analytics or telemetry. The service's CSP
(`default-src 'self'`, no inline script, images also as `data:` for a
report's screenshot) holds the page to that.

## Dependencies and their licences

Every version is exact, and `package-lock.json` is committed so `npm ci`
installs the same tree. What ships to the browser is only React:

| package | version | licence | why |
| --- | --- | --- | --- |
| react, react-dom | 19.3.0 | MIT | the UI (with `scheduler`, MIT) |

Build and test only, never shipped:

| package | version | licence | why |
| --- | --- | --- | --- |
| typescript | 7.0.2 | Apache-2.0 | type checking |
| vite | 8.3.1 | MIT | the build (rolldown, oxc, lightningcss MPL-2.0) |
| vitest | 5.0.2 | MIT | unit and component tests |
| jsdom | 30.1.1 | MIT | the DOM for those tests |
| @testing-library/react | 16.3.3 | MIT | component tests |
| @testing-library/user-event | 14.6.7 | MIT | typing and clicking in them |
| oxlint | 1.85.0 | MIT | lint |
| @playwright/test | 1.63.0 | Apache-2.0 | end-to-end tests |
| postgres | 3.4.9 | Unlicense | the e2e harness's schema, nothing else |
| @types/react, @types/react-dom | 19.3.0 | MIT | types |
| @types/node | 24.19.0 | MIT | types for the e2e harness |

The whole installed tree (27.09.2026): 114 MIT, 28 Apache-2.0, 12 MPL-2.0
(lightningcss and its platform builds), 2 each of ISC, BSD-2-Clause,
BSD-3-Clause and MIT-0, and one each of BlueOak-1.0.0, CC0-1.0 and
Unlicense. All are compatible with AGPL-3.0; none of the non-MIT ones
reaches the built page.

## Using it

- The list: newest first, 50 a page, filtered by kind, status, gateway,
  pseudonym, text, a range of days (UTC) and whether a game record is
  attached. The filters live in the URL, so a filtered view is a link.
  `j`/`k` (or the arrows) move, `Enter` opens, `/` goes to the search.
  A pseudonym in a row, or in "Reporters by number of reports", filters by
  it.
- A report: what the player wrote, a crash, the screenshot, the player's
  game log, the table as they saw it, system and settings, and the whole
  `client` object as a folding tree; the game record's size and
  completeness and a download. `Escape` goes back to the list.
- Triage: the status; the GitHub issue, linked by number or URL; a button
  that opens GitHub's new-issue page with a neutral technical summary the
  admin writes in the field above it (empty until they do: no personal
  data, no quotes from the report), a neutral category and the build, and
  nothing else: not the player's text, not the pseudonym, not the client's
  details, and no link back to the report (owner, 08.10.2026). The server
  holds no GitHub token. Deleting asks once more. Every change is in the
  report's history, under the admin's name.
- Admin (`/admin/…`, shown when the service has a gateway's console,
  `docs/feedback.md` §"The admin console"), a sidebar on a wide screen and
  a tab bar under the thumb on a phone:
  - Overview (`/admin`): headline tiles (online, games running, accounts,
    new today, games today, new reports), three 30-day bar charts (games,
    players, new accounts and guests; hover, touch or arrow keys for a
    day, and a table view), the open tables, and every count the gateway
    keeps; asked again every 15 seconds while the page is visible and not
    while it is hidden.
  - Live (`/admin/live`): who is online and where, every waiting and
    running table with its chairs, decks and readiness, and the agents;
    every 5 seconds.
  - Accounts (`/admin/accounts`): search by name, username or `#tag`,
    registered or guests, online only, five orders, 50 a page, all in the
    query string; a table on a desktop, cards on a phone. One account
    (`/admin/accounts/{id}`): its facts, where it is now, its decks and
    latest games.
  - Beta keys (`/admin/keys`): made (each shown once, with Copy, Copy all
    and, where the browser can, Share), listed and filtered by state
    (never the key, which is kept only as a hash), revoked after a second
    question; the latest console changes.

  In German when the browser asks for German, else English; the rest of
  the UI is English only.
