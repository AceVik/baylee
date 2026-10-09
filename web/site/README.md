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

No request leaves the origin but the ones to GitHub: the fonts (Faustina,
Alegreya Sans; subsets of the client's own files, OFL 1.1, licences beside
them in `fonts/`), the painting and the logo (ours, `docs/legal.md` §10)
and the screenshots (`docs/images/readme/`, Baylee's text view, no card
image) are in this folder. No analytics, no cookies.

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
files, everything else untouched. The page is written for that snippet's
policy, which allows no inline style or script (`style-src 'self';
script-src 'self'`): keep the CSS in `site.css` and the script in `site.js`.
