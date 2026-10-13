#!/usr/bin/env python3
"""Checks on the landing page's own files (web/site): what can be read from
the HTML and the CSS without a browser.

- nothing inline the CSP would refuse (style attributes, inline scripts,
  on* handlers);
- every picture has an alt text, every form control a label, every button a
  type, every dialog a name, every German block `lang="de"`;
- per language one h1 and no heading level skipped on the way down;
- the content stamps are current (scripts/site/stamp.py);
- no filter, opacity or blend on a card picture (docs/legal.md §3).

Run from the repository root:

    python3 -m unittest discover -s scripts/site -p 'test_*.py'

Standard library only.
"""

from __future__ import annotations

import re
import subprocess
import sys
import unittest
from html.parser import HTMLParser
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SITE = ROOT / "web" / "site"
PAGES = ("index.html", "impressum.html", "datenschutz.html")
VOID = {"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"}


class Node:
    def __init__(self, tag: str, attrs: dict[str, str | None], parent: "Node | None"):
        self.tag, self.attrs, self.parent = tag, attrs, parent
        self.children: list[Node] = []
        self.text = ""

    def classes(self) -> set[str]:
        return set((self.attrs.get("class") or "").split())

    def ancestors(self):
        n = self.parent
        while n is not None:
            yield n
            n = n.parent

    def walk(self):
        yield self
        for c in self.children:
            yield from c.walk()


class Tree(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.root = Node("#root", {}, None)
        self.cur = self.root
        self.inline_scripts: list[str] = []

    def handle_starttag(self, tag, attrs):
        node = Node(tag, dict(attrs), self.cur)
        self.cur.children.append(node)
        if tag not in VOID:
            self.cur = node

    def handle_startendtag(self, tag, attrs):
        self.cur.children.append(Node(tag, dict(attrs), self.cur))

    def handle_endtag(self, tag):
        n = self.cur
        while n is not None and n.tag != tag:
            n = n.parent
        if n is not None and n.parent is not None:
            self.cur = n.parent

    def handle_data(self, data):
        self.cur.text += data
        if self.cur.tag == "script" and data.strip():
            self.inline_scripts.append(data.strip()[:60])


def parse(name: str) -> Tree:
    tree = Tree()
    tree.feed((SITE / name).read_text(encoding="utf-8"))
    return tree


def text_of(node: Node) -> str:
    return (node.text + "".join(text_of(c) for c in node.children)).strip()


class PageTests(unittest.TestCase):
    def test_nothing_inline_that_the_csp_refuses(self):
        for name in PAGES:
            tree = parse(name)
            for n in tree.root.walk():
                self.assertNotIn("style", n.attrs, f"{name}: <{n.tag}> has a style attribute")
                handlers = [a for a in n.attrs if a.startswith("on")]
                self.assertFalse(handlers, f"{name}: <{n.tag}> has {handlers}")
                if n.tag == "style":
                    self.fail(f"{name}: an inline <style>")
            self.assertEqual(tree.inline_scripts, [], f"{name}: inline script")

    def test_every_picture_has_an_alt_text(self):
        for name in PAGES:
            for n in parse(name).root.walk():
                if n.tag == "img":
                    self.assertIn("alt", n.attrs, f"{name}: {n.attrs.get('src') or n.attrs.get('id')} has no alt")

    def test_every_form_control_has_a_label(self):
        for name in PAGES:
            tree = parse(name)
            labelled = {n.attrs.get("for") for n in tree.root.walk() if n.tag == "label"}
            for n in tree.root.walk():
                if n.tag not in ("input", "textarea", "select") or n.attrs.get("type") == "hidden":
                    continue
                wrapped = any(a.tag == "label" for a in n.ancestors())
                named = n.attrs.get("aria-label") or n.attrs.get("aria-labelledby")
                self.assertTrue(wrapped or named or n.attrs.get("id") in labelled, f"{name}: {n.attrs} has no label")

    def test_buttons_have_a_type_and_dialogs_a_name(self):
        for name in PAGES:
            for n in parse(name).root.walk():
                if n.tag == "button":
                    self.assertIn(n.attrs.get("type"), ("button", "submit"), f"{name}: a button without a type: {n.attrs}")
                    self.assertTrue(text_of(n) or n.attrs.get("aria-label"), f"{name}: a button without a name: {n.attrs}")
                if n.tag == "dialog":
                    self.assertTrue(n.attrs.get("aria-label") or n.attrs.get("aria-labelledby"), f"{name}: an unnamed dialog")

    def test_german_blocks_say_they_are_german(self):
        for name in PAGES:
            for n in parse(name).root.walk():
                if "de" in n.classes():
                    self.assertEqual(n.attrs.get("lang"), "de", f"{name}: <{n.tag} class=de> without lang=de: {text_of(n)[:50]}")

    def test_headings_go_down_one_level_at_a_time_in_each_language(self):
        for name in PAGES:
            tree = parse(name)
            for lang, other in (("en", "de"), ("de", "en")):
                levels = []
                for n in tree.root.walk():
                    if not re.fullmatch(r"h[1-6]", n.tag):
                        continue
                    if other in n.classes() or any(other in a.classes() for a in n.ancestors()):
                        continue
                    levels.append(int(n.tag[1]))
                self.assertEqual(levels.count(1), 1, f"{name} ({lang}): {levels.count(1)} h1")
                self.assertEqual(levels[0], 1, f"{name} ({lang}): the first heading is h{levels[0]}")
                for a, b in zip(levels, levels[1:]):
                    self.assertLessEqual(b, a + 1, f"{name} ({lang}): h{a} followed by h{b}")

    def test_links_that_open_a_tab_say_noopener(self):
        for name in PAGES:
            for n in parse(name).root.walk():
                if n.tag == "a" and n.attrs.get("target") == "_blank":
                    self.assertIn("noopener", n.attrs.get("rel") or "", f"{name}: {n.attrs.get('href')}")

    def test_the_content_stamps_are_current(self):
        run = subprocess.run([sys.executable, str(ROOT / "scripts" / "site" / "stamp.py"), "--check"], capture_output=True, text=True)
        self.assertEqual(run.returncode, 0, "stale stamps; run python3 scripts/site/stamp.py\n" + run.stdout)

    def test_no_card_picture_is_filtered_faded_or_blended(self):
        css = (SITE / "site.css").read_text(encoding="utf-8")
        css = re.sub(r"/\*.*?\*/", "", css, flags=re.S)
        for selector, body in re.findall(r"([^{}]+)\{([^{}]*)\}", css):
            if not re.search(r"\.(ctile-pic|cp-pic)\b", selector):
                continue
            for prop in ("filter", "opacity", "mix-blend-mode", "backdrop-filter"):
                self.assertNotRegex(body, rf"(^|;|\s){prop}\s*:", f"{selector.strip()} sets {prop}")
        js = (SITE / "site.js").read_text(encoding="utf-8")
        self.assertIn("object-fit: contain", css.split(".ctile-pic img", 1)[1].split("}", 1)[0])
        self.assertNotRegex(js, r"\.style\.(filter|opacity)\s*=.*(ctile|cp-pic)")


class PrivacyNoticeTests(unittest.TestCase):
    """web/site/datenschutz.html says what the site does; these hold it to the code."""

    def test_every_page_links_the_privacy_notice_and_the_legal_notice(self):
        """Each page's footer names both legal pages."""
        for name in PAGES:
            tree = parse(name)
            foot = [n for n in tree.root.walk() if n.tag == "footer"]
            self.assertEqual(len(foot), 1, name)
            hrefs = {n.attrs.get("href") for n in foot[0].walk() if n.tag == "a"}
            self.assertIn("/site/datenschutz.html", hrefs, name)
            self.assertIn("/site/impressum.html", hrefs, name)

    def test_every_foreign_host_the_page_may_reach_is_named_in_the_notice(self):
        """A host in the CSP that is not ours is one the visitor's browser talks to."""
        caddy = (ROOT / "scripts" / "server" / "site.caddy").read_text(encoding="utf-8")
        csp = re.search(r'Content-Security-Policy "([^"]+)"', caddy).group(1)
        hosts = set(re.findall(r"https://([a-z0-9.-]+)", csp))
        notice = (SITE / "datenschutz.html").read_text(encoding="utf-8")
        for host in sorted(hosts):
            if host.endswith("baylee.acevik.de"):
                continue  # our own services, on the same server
            self.assertTrue(host in notice, f"{host} is in the CSP and not in datenschutz.html")

    def test_no_script_sets_a_cookie(self):
        """The notice says the site sets no cookie."""
        for js in SITE.glob("*.js"):
            self.assertNotIn("document.cookie", js.read_text(encoding="utf-8"), js.name)

    def test_the_site_keeps_no_access_log(self):
        """The notice says no access log is written; the snippet must agree."""
        caddy = (ROOT / "scripts" / "server" / "site.caddy").read_text(encoding="utf-8")
        directives = [l.strip() for l in caddy.splitlines() if l.strip() and not l.strip().startswith("#")]
        self.assertFalse([l for l in directives if re.match(r"(log|log_skip|skip_log)\b", l)], "site.caddy writes a log")

    def test_the_notice_names_the_controller_only_through_the_impressum(self):
        """No address, mail or phone on the page: the Impressum names the controller."""
        tree = parse("datenschutz.html")
        section = next(n for n in tree.root.walk() if n.attrs.get("id") == "verantwortlich")
        self.assertIn("/site/impressum.html", {n.attrs.get("href") for n in section.walk() if n.tag == "a"})
        text = text_of(tree.root)
        self.assertNotRegex(text, r"[\w.+-]+@[\w-]+\.[\w.]+", "an e-mail address on the privacy page")
        self.assertNotRegex(text, r"\+\d[\d ]{6,}", "a telephone number on the privacy page")


if __name__ == "__main__":
    unittest.main()
