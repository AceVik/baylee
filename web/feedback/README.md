# The feedback service's web UI (#311)

The page an admin triages player reports on, at
`https://feedback.baylee.app/`. It is a React app built to static
files that `baylee-feedback` serves itself from `FEEDBACK_WEB_DIR`, and it
talks only to that service's `/ui/api/…` routes (`docs/feedback.md` §"The
web UI"). Besides the reports it has a *Contact* page (`/contact`) for the
messages from the landing page's contact form: read, answer by mail,
mark done, delete (`docs/feedback.md` §"The landing page's contact
form"). `cargo build` never needs node: without a build, or with
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
`e2e/admin.spec.ts` drives the reports list, a report and the console at
320×640 (dark), 360×740 and 740×360 (touch, the first two in German) and
1280×800, and fails on
a page or a card wider than its screen, on a control under 44 px and on
visible text under 12 px; with `SCREENSHOTS_DIR` it saves each.

## Its look

One design system, in `src/styles/`, each selector defined once:

- `tokens.css`: every colour (light, and dark as a blue hour), the type
  scale (12 px at the least), the spacing grid of 4 px, radii, shadows,
  durations, the layout's sizes and stacking. No other file names a raw
  colour or size, so the look can move with the console into baylee.app
  (M5) by carrying this file and the next.
- `base.css`, `components.css`: type, links and focus; buttons, fields,
  panels, headline tiles, tables, badges, segmented controls, dialogs,
  toasts, and the states a page can be in (empty, failed, loading).
- `shell.css`: the bar, the console's sections (a sidebar on a wide screen,
  a tab bar at the bottom of a tablet or phone, its icons alone under
  480 px) and a page's column.
- `admin.css`, `reports.css`, `contact.css`, `models.css`: each page's own
  parts, built from the above.

The rules it keeps: text holds 4.5:1 and a control's edge, a chart's mark
and the focus ring 3:1 against every surface they are drawn on, in both
schemes (`src/styles/contrast.test.ts` measures `tokens.css`); a control is
44 px high in the console, on the bar and under a finger; motion is short
(120–180 ms) and stops when the system asks for reduced motion; the root is
87.5 %, so the reader's own font size scales the page. Every page has a skip
link to its content, names itself in the window's title and tells the
browser its language. The mark (`src/Brand.tsx`, and `public/favicon.svg`,
served by the service like the rest) is drawn, not fetched.

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
web font (system fonts only), no analytics or telemetry. The stylesheets'
few icons (a select's chevron, the search glass, the empty and failed
states) are `data:` SVG in the CSS, which the policy's `img-src data:`
allows. The one thing the
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
installs the same tree. What ships to the browser is React and Fluent:

| package | version | licence | why |
| --- | --- | --- | --- |
| react, react-dom | 19.3.1 | MIT | the UI (with `scheduler`, MIT) |
| @fluent/bundle | 0.19.1 | Apache-2.0 | the UI's words in each language (§"Its words"); no dependencies |

Build and test only, never shipped:

| package | version | licence | why |
| --- | --- | --- | --- |
| typescript | 7.0.2 | Apache-2.0 | type checking |
| vite | 8.3.4 | MIT | the build (rolldown, oxc, lightningcss MPL-2.0) |
| vitest | 5.0.4 | MIT | unit and component tests |
| jsdom | 29.1.1 | MIT | the DOM for those tests |
| @testing-library/react | 16.3.3 | MIT | component tests |
| @testing-library/user-event | 14.6.8 | MIT | typing and clicking in them |
| oxlint | 1.89.0 | MIT | lint |
| @playwright/test | 1.64.1 | Apache-2.0 | end-to-end tests |
| postgres | 3.4.9 | Unlicense | the e2e harness's schema, nothing else |
| @types/react, @types/react-dom | 19.3.0 | MIT | types |
| @types/node | 24.19.2 | MIT | types for the e2e harness |

`jsdom` stays on 29.1.1 so the build and tests support Node 22.22.1.
On the Node 22 line, jsdom 30 and some of its dependencies require at
least 22.22.2. The complete lockfile, including optional platform packages,
was checked against 22.22.1, and `npm ci --engine-strict` succeeds on it.

The whole locked tree (20.10.2026): 116 MIT, 29 Apache-2.0, 12 MPL-2.0
(lightningcss and its platform builds), 2 each of ISC, BSD-2-Clause,
BSD-3-Clause and MIT-0, and one each of BlueOak-1.0.0, CC0-1.0 and
Unlicense. All are compatible with AGPL-3.0; of the non-MIT ones only
`@fluent/bundle` (Apache-2.0) reaches the built page.

## Using it

- The list: newest first, 50 a page (25 to 200, with page numbers). Views
  along the top (Inbox = new, Open, Mine, Unassigned, Done, All, Trash, the
  first ones with their counts), saved views (kept on the service, per
  admin), a search of the text, a crash's message and report ids, and
  under "Filters" several statuses, kinds and priorities at once, the
  assignee, a label, gateway, way in, pseudonym, a range of days (UTC),
  whether a game record is attached, only unread, and the order. Each
  filter shows its count under the rest, and each active one is a pill
  that removes it. Column headers sort. The filters live in the URL, so a
  filtered view is a link. Times are local, as "5 minutes ago" with the
  exact time as the title. A report in `new` this admin never opened is
  marked unread (kept by the service, per admin). Crash groups show the
  crashes the filter matches by signature, with a way to their reports.
  The list is asked again every 30 seconds while the page is visible:
  changes to the rows shown are applied, and new reports are announced
  ("3 new reports since you looked · Show") rather than slid under the
  cursor; Refresh shows a spinner and "Updated hh:mm", and a refresh that
  fails is a toast.
  `j`/`k` (or the arrows) move the cursor, `Enter` opens the report in a
  drawer beside the list (over it on a narrow screen), `o` opens its page,
  `x` selects, `a` selects the page, `1`–`6` set the status of the
  selection (or of the report under the cursor), `m` assigns it to you
  (or to nobody again), `u` marks it read or unread, `#` or `Del` moves it
  to the trash, `[`/`]` turn the page, `r` refreshes, `c` copies the link,
  `e` exports the selection (or the page) as CSV, `/` goes to the search,
  `Esc` closes the drawer or clears the selection, `?` lists them. A
  selection shows a bar that changes all of them in one request (all or
  none): status, priority, assignee, a label added or removed, the others
  marked as duplicates of one, the trash (with Undo in the toast; in the
  Trash view: restore, or delete for good), read or unread, and copies or
  exports them. "Export all n matching" is the service's CSV of the whole
  filter (at most 5000). A reporter is
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
  game record's size and completeness and a download, and the history in
  words. Beside it: status, priority, assignee ("Assign to me"), labels,
  the report it duplicates (by id or link), the GitHub issue, internal
  notes (`Ctrl`+`Enter` adds one; only its author removes it), the reports
  of the same game or crash and its duplicates, and "Move to trash" (with
  Undo; a report in the trash says since when and offers Restore and
  Delete for good). Every change names the revision it was made against: a
  report someone else changed meanwhile is read again and says so. The
  page steps to the previous and next report of the list it came from
  (`j`, `k`), and the screenshot opens at full size.
  Copy the link, the id or a plain-text summary. `Escape` goes back to the
  list.
- Triage: the status; the GitHub issue, linked by number or URL; a button
  that opens GitHub's new-issue page with a neutral technical summary the
  admin writes in the field above it (empty until they do: no personal
  data, no quotes from the report), a neutral category and the build, and
  nothing else: not the player's text, not the pseudonym, not the client's
  details, and no link back to the report (owner, 08.10.2026). The server
  holds no GitHub token. The trash needs no question (Undo is in its
  toast); deleting for good, from the trash only, asks once more. Every
  change is in the report's history, under the admin's name.
- Admin (`/admin/…`, shown when the service has a gateway's console,
  `docs/feedback.md` §"The admin console"), a sidebar on a wide screen and
  a tab bar under the thumb on a phone (its icons alone under 480 px):
  - Overview (`/admin`): headline tiles (online, games running, accounts,
    new today, games today, new reports, and "Offline now": the anonymous
    count of offline games this service hears beat, `/ui/api/admin/offline`), four 30-day bar charts (reports,
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
    A running table has End game (`src/admin/Moderation.tsx`, beta.8): a
    reason the players are shown, asked in a dialog.
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
    latest games, and the acts on it (beta.8, `src/admin/Moderation.tsx`,
    `docs/feedback.md` §"The admin console"): reset its password (shown
    once, with Copy), ban it (a reason and an end, or permanently) or lift
    the ban, delete it (its handle typed to confirm). Each asks first; a
    banned account is marked in the list and on its page.
  - Blocked cards (`/admin/cards`, `src/admin/Cards.tsx`): a card of the
    gateway's pool searched by name or index and blocked, without an end,
    for a day, a week, a month or until a moment picked, with a reason the
    players see, after a second question; the blocks in force, each lifted
    after a second question; the earlier ones, folded; the latest card
    changes (`docs/protocol.md` §"Blocked cards").
  - Beta keys (`/admin/keys`): made (each shown once, with Copy, Copy all
    and, where the browser can, Share), listed and filtered by state
    (never the key, which is kept only as a hash), revoked after a second
    question; the latest console changes.
  - Models (`/admin/models`, `src/admin/Models.tsx`, beta.7): the hosted
    language models (`docs/feedback.md` §"Hosted models"): the connected
    seat agents, each profile with its state, games, caps and spend, per
    agent; a profile added, edited or removed on a seat agent, switched on
    or off, probed, and its key set or forgotten (write-only: typed, sent,
    cleared, never shown again). Each change is passed on to the gateway
    and answered as it answers.

  The admin area and the reports (since beta.7) are in German when the
  browser asks for German, else English; the sign-in form is English only.

## Its words

The UI's words are Project Fluent files, `locales/<lang>/feedback.ftl`, one
per language, held to the rules and the glossary of `docs/translations.md`.
English decides which messages exist and which arguments each takes; another
language only translates. `src/i18n.ts` parses each file once into a
`FluentBundle` (without bidi isolation marks, which would reach the DOM) and
keeps the call sites' API: `t(lang, "contact.summary", { open: 3 })` names
the message `contact-summary`, because a Fluent id takes no dot, and a
message a language lacks reads in English, a key English lacks as itself.
`src/i18n-keys.ts` lists every id as call sites spell it, so a misspelt key
does not compile. A number passed as an argument is written as the language
writes it (1,234, 1.234) and picks the plural form by CLDR's rules
(`{ $n -> [one] … *[other] … }`), so a count is passed as a number and an
identifier (an issue number, a version) as a string.

`src/i18n.test.ts` refuses a file the parser skipped a line of, an id written
twice, a language without a file (or a file without a language), a message
missing from a language or not in English, a message reading other arguments
than its English one, a plural form the language does not have, and a
message that fails to format; it also holds `src/i18n-keys.ts` to the English
file. A language is added in `src/i18n.ts` (an import of its file and an
entry in `LANGS` with its locale) and nowhere else in the code.
