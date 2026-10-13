# web/site — the landing page at https://baylee.acevik.de/

A static page: what Baylee is, what it can do today, where the cards stand,
the closed beta, self-hosting, the licence, and a download button that picks
the right file for the visitor's system. English and German (`?lang=de|en`,
the choice remembered in `localStorage`, else the browser's language; the two
buttons switch by hand; without JavaScript both languages show). `?theme=dark|light`
forces a colour scheme (for screenshots); otherwise `prefers-color-scheme`
decides.

Nothing is copied from the repository into the page that GitHub can answer:
`site.js` asks `api.github.com` for the newest release (pre-releases
included) with its files, sizes and date, for the latest `ci.yml` run on
`main`, the last commit, stars and open issues, and keeps the answers in
`sessionStorage` for ten minutes (GitHub allows 60 requests an hour per
address without a token; there is no token in the page). When GitHub does not
answer, the page keeps its static text and links to the releases page. Where
the gateway serves the page, `site.js` also reads the gateway's own `/info`
and `/health` (same origin) for the beta section.

The download goes straight to the release file on GitHub
(`browser_download_url`), which GitHub serves as an attachment, so the page
stays. The system is read from `navigator.userAgentData` where a browser
offers it, else from the user agent; what cannot be known (an Intel Mac from
Safari, Windows on ARM from Firefox) is said beside the button instead of
guessed. Installers (`.exe`, `.dmg`, `.AppImage`, `.deb`) are offered first
when a release carries them, the archives otherwise.

## The card status (`data/`)

The "Sets and card status" list under the cards section reads
`data/sets.json` (every set in release order, its counts) and, when a visitor
opens a set, `data/sets/set-<code>.json` (the `set-` prefix keeps a code such as `con` from naming a reserved Windows file; that set's cards: name, oracle id,
status, the reason for a partial or never-built card, and a Scryfall printing
id). `scripts/site/card_status.py` writes both from one git revision of the
repository, never from the working tree:

```sh
python3 scripts/site/card_status.py --rev origin/main     # or any commit
python3 scripts/site/card_status.py --rev origin/main --fetch-bulk   # pictures for every card
```

It reads the CardIndex ledger (every card, the set it first appeared in, in
release order), every card file's `coverage` and `scryfall_id`, and
`data/unplayable.tsv`; it checks each set's count against the core index's
per-set file and prints the totals. Set names come from one request to
Scryfall's set list, cached under `target/site-cache/`. A card outside the
pool has no printing id in the repository; `--fetch-bulk` downloads Scryfall's
`default_cards` bulk file once into `target/site-cache/` (`--bulk` names one
already on disk), which gives every card the picture of its printing
in the set it is listed under (without it, the ids of the previous output are
kept). Regenerate and redeploy the page when cards land.

The pictures are loaded by the visitor's browser straight from
`cards.scryfall.io`, only for a set that is opened, lazily, unaltered, with
nothing drawn on them, and credited (`docs/legal.md` §3). The page never calls
Scryfall's API and nothing is re-hosted.

## Motion

The hero is the painting in layers (`.sky .layer`, each with a
`data-speed`, how far it follows the scroll, and a `data-tilt`, how far it
leans with a fine pointer); sections come up as they are read (only what
starts below the fold, so nothing on screen blinks), numbers count up,
the sets' bars grow, panels (`.glass`) carry a light that follows the
pointer, buttons sweep and leave a ring where they were pressed, and the
screenshots open large in a `<dialog>`. Everything moves with transform
and opacity from one `requestAnimationFrame` at most; under
`prefers-reduced-motion` nothing moves and nothing waits to be revealed.
Under 1100 px the sections are a menu (without JavaScript the links wrap
under the bar). Every control is at least 44 px.

## The contact form

`#contact` posts with `fetch` to the feedback service's
`POST /site/contact` (`docs/feedback.md` §"The landing page's contact
form"), whose address is the form's `data-endpoint`; a page served from
`localhost`/`127.0.0.1` uses `data-endpoint-local` instead, so a local try
reaches a service on this machine (`127.0.0.1:28780`, started with `FEEDBACK_CONTACT_ORIGINS=http://localhost:8000`).
It sends the name and address only if given, the message, the page's
language and an empty honeypot (`website`, hidden from people). Without
JavaScript the form is replaced by a note pointing to Discord and GitHub:
the page's CSP allows no form submission. The sentence beside it says what
is kept and for how long (`docs/privacy.md` §"Contact messages").

No request leaves the origin but the ones to GitHub, the feedback service
when a visitor sends the contact form, and, for an opened set, Scryfall's
image host: the fonts (Faustina,
Alegreya Sans; subsets of the client's own files, OFL 1.1, licences beside
them in `fonts/`), the painting and the logo (ours, `docs/legal.md` §10)
and the screenshots (`docs/images/readme/`, Baylee's text view, no card
image) are in this folder. No analytics, no cookies.

## The Impressum

`impressum.html` is the legal notice (§ 5 DDG; `docs/legal.md` §4 says what
it carries and why). The owner's name, address, e-mail address and
telephone number are in no file of this repository, and nothing a crawler
reads as text holds them:

- the data is a private JSON on the owner's machine (`name`, `street`,
  `postcode`, `city`, `country_de`, `country_en`, `email`, `phone`);
- `scripts/site/impressum_blob.py <private.json> [<out>]` (standard library
  only) writes from it a blob: base64url of the JSON XORed with a SHA-256
  keystream, its key split between `SCRIPT_KEY` in the script and in
  `impressum.js` and a random nonce in the blob. That is obfuscation, not
  security: there is no `@` and no `.` in it and none of the data as text, so a
  harvester grepping for addresses finds nothing, and anybody running the
  page reads it, which is what an Impressum is for. The output defaults to
  `~/.local/share/baylee/impressum.bin` and a path inside the repository is
  refused; `.gitignore` keeps `*.bin` and `private/` out of `web/`, and
  `scripts/site/test_impressum_blob.py` fails if a blob or the data appears
  under `web/`;
- the server keeps the blob in `/etc/baylee/site-private/impressum.bin`
  (outside `/opt/baylee/web`, which a page update replaces), and the
  feedback service hands it out (`FEEDBACK_IMPRESSUM_BLOB`, read on each
  request) for a solved proof of work: `GET /site/impressum/challenge`, then
  `POST /site/impressum` (`docs/feedback.md` §"The Impressum"). The site's
  Caddy passes `/site/impressum…` on to the service, so the page asks its
  own origin (`scripts/server/site.caddy`). A request that looks like a
  robot gets a harder puzzle, never a refusal; only a robot beyond doubt,
  or an address that followed the page's honeypot link (`.hp`, hidden from
  people, to `/site/impressum/kontakt`), gets invented details;
- `impressum.js` (not `site.js`, which would ask GitHub for nothing here)
  starts on *Impressum anzeigen*: it reports three facts about its own
  browser (`navigator.webdriver`, no languages, a software WebGL renderer),
  solves the puzzle in `impressum-worker.js` (plain SHA-256, a file because
  the CSP allows no inline worker), posts it, decodes the blob with
  `crypto.subtle` (a secure context: https, or `localhost`) and draws every
  character as its own span, in a shuffled DOM order among decoys that take
  no room (`.scr`, `.scr-x`, the order classes `.o0`–`.o63`), so the DOM's
  text is a jumble while the screen reads right. The drawn block is
  `aria-hidden` and not selectable; a sentence for screen readers, the
  *copy* buttons (the clipboard gets the right text) and *write*/*call*
  (a `mailto:`/`tel:` built in the click handler, never an `href` in the
  page) are the ways to use it. Without JavaScript the page says to turn it
  on or use the contact form; when the details do not come, it says they
  could not be loaded.

The page itself is served with `X-Robots-Tag: noindex, noarchive,
nosnippet` (and the same `<meta name="robots">`), `robots.txt` disallows it
and `/site/impressum/`, and no sitemap names it.

Deploying a changed address: edit the private JSON, run the script, copy the
blob to the server and install it as above; nothing in the page, the
service or Caddy changes. Trying it locally: write a blob from a made-up
JSON to a scratch directory (never the repository), start the feedback
service with `FEEDBACK_IMPRESSUM_KEY`, `FEEDBACK_IMPRESSUM_BLOB` (that file)
and `FEEDBACK_CONTACT_ORIGINS=http://localhost:8000` on `127.0.0.1:28780`
(the page's `data-endpoint-local`), and serve `web/` as below.

## Trying it locally

The page references its files as `/site/…`, so serve the parent folder:

```sh
cd web && python3 -m http.server 8000
```

and open <http://localhost:8000/site/>. The GitHub data loads from there; the
gateway card stays on its static text because there is no `/info` locally.

## Deploying it

`scripts/server/site.caddy` has the Caddy directives and the install steps:
`handle /` for the page, `handle /favicon.ico`, `handle_path /site/*` for the
files, everything else untouched. Its CSP names the feedback service in
`connect-src` for the contact form; `scripts/server/feedback-direct.caddy`
passes `/site/contact` on to the service. The page is written for that snippet's
policy, which allows no inline style or script (`style-src 'self';
script-src 'self'`): keep the CSS in `site.css` and the script in `site.js`.
