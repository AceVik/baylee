#!/usr/bin/env python3
"""Names the landing page's files by their content (web/site/README.md
§"Caching").

Every reference to a file of the page in web/site/index.html (href, src,
srcset, imagesrcset) and every url() in web/site/site.css gets `?v=` and the
first 10 hex digits of the file's SHA-256. Caddy serves a URL with `v` as
immutable for a year (scripts/server/site.caddy), so a visitor's browser
keeps the fonts, the pictures, the stylesheet and the scripts until one of
them changes, and then asks for the new name at once.

site.css is stamped first, so its own hash covers the names inside it.
The data (data/*.json), the HTML pages and absolute URLs are left alone.

    python3 scripts/site/stamp.py           # rewrite the stamps
    python3 scripts/site/stamp.py --check   # exit 1 if any stamp is stale

Run it after changing any file of the page; scripts/site/test_site_page.py
fails while a stamp is stale. Standard library only.
"""

from __future__ import annotations

import hashlib
import re
import sys
from pathlib import Path

SITE = Path(__file__).resolve().parents[2] / "web" / "site"

# url("fonts/x.woff2") or url("img/x.avif"), stamped or not
CSS_REF = re.compile(r'url\("((?:fonts|img)/[^"?]+)(?:\?v=[0-9a-f]+)?"\)')
# /site/<file> after a quote, a space or a comma (srcset), stamped or not;
# never data/ (regenerated with the cards) and never an .html page
HTML_REF = re.compile(r'(?<=["\s,])/site/((?!data/)[^"\s?,]+\.(?:css|js|woff2|webp|avif|png|jpg|ico))(?:\?v=[0-9a-f]+)?')


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()[:10]


def stamp_css(text: str) -> str:
    def one(m: re.Match[str]) -> str:
        return f'url("{m.group(1)}?v={digest(SITE / m.group(1))}")'

    return CSS_REF.sub(one, text)


def stamp_html(text: str) -> str:
    def one(m: re.Match[str]) -> str:
        return f"/site/{m.group(1)}?v={digest(SITE / m.group(1))}"

    return HTML_REF.sub(one, text)


def run(check: bool) -> int:
    stale = []
    css = SITE / "site.css"
    new_css = stamp_css(css.read_text(encoding="utf-8"))
    if new_css != css.read_text(encoding="utf-8"):
        stale.append(css)
        if not check:
            css.write_text(new_css, encoding="utf-8")
    # the page's names use site.css as it is (or would be) after stamping
    page = SITE / "index.html"
    text = page.read_text(encoding="utf-8")
    if check and stale:
        # what index.html would need is unknowable until site.css is
        # rewritten; it is stale either way
        stale.append(page)
    else:
        new_page = stamp_html(text)
        if new_page != text:
            stale.append(page)
            if not check:
                page.write_text(new_page, encoding="utf-8")
    for path in stale:
        print(("stale: " if check else "stamped: ") + str(path.relative_to(SITE.parents[1])))
    return 1 if check and stale else 0


if __name__ == "__main__":
    sys.exit(run("--check" in sys.argv[1:]))
