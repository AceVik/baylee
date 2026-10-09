# web/play-soon — the page at /play/ while the browser version rests

A static page (owner, 08.10.2026) that stands in for the browser client at
<https://baylee.acevik.de/play/>: Baylee's name and logo, a short note that the
web version is paused while the desktop app is polished, the download button
(GitHub Releases), the project links and the Fan Content notice, in English
and German (`?lang=de|en`, the browser's language, or the two buttons; without
JavaScript both languages show). `?theme=dark|light` forces a colour scheme
(screenshots); otherwise `prefers-color-scheme` decides.

Nothing leaves the origin: the fonts (Faustina, Alegreya Sans; subsets of the
client's own files, OFL 1.1, licences beside them in `fonts/`), the painting
(`sanctuary.webp`, our `assets/scenes/sanctuary-world.png` scaled down) and the
logo are served from this folder. No analytics, no cookies, no storage. The
only `https://` in these files are the outbound links (GitHub).

## Deploying it

Caddy already serves `/play/` from `/opt/baylee/web/play`
(`scripts/server/play.caddy`): copy this folder there and nothing in Caddy
changes. The page is written for that site block's policy, which allows no
inline style or script (`style-src 'self'; script-src 'self'`): keep the CSS in
`style.css` and the script in `lang.js`, never inline. Install it as
`baylee-deploy` installs the client, a copy beside the old directory and one
rename:

```sh
sudo cp -R web/play-soon /opt/baylee/web/play.new
sudo rm -rf /opt/baylee/web/play
sudo mv /opt/baylee/web/play.new /opt/baylee/web/play
```

`baylee-deploy stage` rebuilds the browser client whenever `trunk` and the
wasm target are on the server (`stage` clears `state/staged/` first, so no
old build lingers there), and `finish` would then put that build over this
page. To keep the page in place through deploys, make `trunk` unfindable for
the script. The script reads `TRUNK` from its environment
(`trunk=${TRUNK:-$HOME/.cargo/bin/trunk}`) *before* it sources
`/etc/baylee/deploy.env`, so a line in that file does nothing; set it where
the script is started instead:

- the timer's unit (`baylee-deploy.service`, `ExecStart=… baylee-deploy watch`):
  a drop-in, `sudo systemctl edit baylee-deploy.service`, with
  `[Service]` / `Environment=TRUNK=/nonexistent`, then `daemon-reload`;
- a deploy run by hand: `TRUNK=/nonexistent baylee-deploy <rev>`.

`play_build` then logs "no trunk here: the browser client is not built, the
one installed stays". Remove the override and deploy to bring the browser
client back. (A one-line reorder in the script, sourcing `deploy.env` before
the `trunk=` line, would let the file carry it; not done here.)

Screenshots: `.claude/ux-b6/play-soon/render.py` (headless Chrome).
