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
web font (system fonts only), no analytics or telemetry. The one thing the
page loads from anywhere else is the picture of a card: a card a report
names, or a card on the set progress page, is fetched by the admin's
browser straight from Scryfall's image host (`src/reports/scryfall.ts`,
`docs/legal.md` §3), never through the service, which neither proxies nor
caches it. The service's CSP (`default-src 'self'`, no inline script,
images also as `data:` for a report's screenshot and from
`cards.scryfall.io`, `connect-src 'self'` so nothing calls Scryfall's API)
holds the page to that.

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

- The list: newest first, 50 a page. Views along the top (Inbox = new,
  Triaged, In progress, Resolved, All, each with its count), a search of
  the text, and under "Filters" the kind, status, gateway, pseudonym, a
  range of days (UTC) and whether a game record is attached. The filters
  live in the URL, so a filtered view is a link. Times are local, as "5
  minutes ago" with the exact time as the title. A report in `new` that was
  never opened on this device is marked unread (kept in `localStorage`,
  per device; the service keeps no such state). The list is asked again
  every 30 seconds while the page is visible: changes to the rows shown are
  applied, and new reports are announced ("3 new reports since you looked
  · Show") rather than slid under the cursor; Refresh shows a spinner and
  "Updated hh:mm", and a refresh that fails is a toast.
  `j`/`k` (or the arrows) move the cursor, `Enter` opens the report in a
  drawer beside the list (over it on a narrow screen), `o` opens its page,
  `x` selects, `a` selects the page, `1`–`6` set the status of the
  selection (or of the report under the cursor), `r` refreshes, `c` copies
  the link, `e` exports the page (or the selection) as CSV, `/` goes to
  the search, `Esc` closes the drawer or clears the selection, `?` lists
  them. A selection shows a bar: set a status on all of them (one audited
  change each), copy their ids, export them, mark them read. A reporter is
  shown as an alias derived from the pseudonym (`src/reports/alias.ts`:
  two words, a hue and the pseudonym's first four characters), the same in
  every row; clicking it filters by them. The alias is derived in the
  browser, never resolved to an account: the pseudonym exists so that this
  service does not know who wrote a report (`docs/feedback.md` §"The
  gateway"), and the UI keeps it that way.
- A report (the drawer and the page `/r/{id}` show the same): what the
  player wrote with each card it names as a chip (its Scryfall page, its
  picture on hover, "Image: Scryfall") and each player as a seat, where in
  the game it was written (turn, phase and step, seat, the question the
  engine was asking, what the client held unsent, the last refusal), a
  crash, the screenshot, the player's game log, the table as they saw it,
  system and settings, the whole `client` object as a folding tree, the
  game record's size and completeness and a download, and the history.
  Copy the link, the id or a plain-text summary. `Escape` goes back to the
  list.
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
    new today, games today, new reports), four 30-day bar charts (reports,
    with bugs and crashes apart; games; players; new accounts and guests;
    hover, touch or arrow keys for a day, and a table view), the open
    tables, every count the gateway keeps, and a Services panel: whether
    the gateway answered and how fast, its build, and this service's
    build (`/health`); asked again every 15 seconds while the page is
    visible and not while it is hidden.
  - Live (`/admin/live`): who is online and where, every waiting and
    running table with its chairs, decks and readiness, the agents, and
    the server: CPU (with the load averages), memory, disk, network in and
    out, requests a second and open sockets, how long the gateway and the
    host have been up, each with the last hour as a line
    (`src/admin/Server.tsx`), and every engine process on that machine
    with its cores and memory; a value the host could not say (anything
    off `/proc` on a Mac) is a dash. Every 5 seconds. Under it the pool's
    progress in short, with the sets being worked on.
  - Sets (`/admin/sets`, `src/admin/Sets.tsx`): how far this build is
    through each set, from the gateway's ledger and compiled pool, in
    release order (sortable by size, by completeness, by what is left;
    unfinished only; a set code to find one), each as a bar of
    implemented, partial, stub and not started; a set counts the cards
    first printed in it. One set (`/admin/sets/{code}`): its cards as
    chips, each linking to its Scryfall page with its picture on hover,
    filtered by how far each is and by name.
  - Accounts (`/admin/accounts`): search by name, username or `#tag`,
    registered or guests, online only, five orders, 50 a page, all in the
    query string; a table on a desktop, cards on a phone. One account
    (`/admin/accounts/{id}`): its facts, where it is now, its decks and
    latest games.
  - Beta keys (`/admin/keys`): made (each shown once, with Copy, Copy all
    and, where the browser can, Share), listed and filtered by state
    (never the key, which is kept only as a hash), revoked after a second
    question; the latest console changes.

  The admin area and the reports (since beta.7) are in German when the
  browser asks for German, else English; the sign-in form is English only.
