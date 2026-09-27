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

# End to end, against a real service and PostgreSQL (not in CI):
cargo build -p baylee-feedback
npm run build
npx playwright install chromium          # once
DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee npm run e2e
SCREENSHOTS_DIR=/tmp/shots DATABASE_URL=… npm run e2e -- -g screenshots
DATABASE_URL=… npm run serve:e2e         # the seeded service, to look at by hand
```

The end-to-end harness (`e2e/harness.ts`) makes a schema of its own, adds an
admin through the binary's `admin add`, starts the binary with
`FEEDBACK_WEB_DIR=dist`, hands reports in through `/intake/reports` as a
gateway does, and drops the schema afterwards.

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
  that opens GitHub's new-issue page prefilled with the report's text, kind
  and build (not the pseudonym, not the client's details). The server holds
  no GitHub token. Deleting asks once more. Every change is in the report's
  history, under the admin's name.
