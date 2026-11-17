#!/usr/bin/env python3
"""Builds the QuickSnap site: one page per language from one template.

    python3 _src/build.py

Reads  _src/template.html and _src/i18n/<code>.json
Writes index.html (English, the x-default), <prefix>/index.html for every
other language, and sitemap.xml.

Template syntax
  {{key}}    text from the language file, as HTML (may contain <b>, links...)
  {{@key}}   the same text as a plain, escaped attribute value
  {{$name}}  a value computed here (paths, hreflang tags, JSON-LD...)
  [[token]]  expanded inside texts and template: [[cmd]], [[keys:5]],
             [[hotkey_heading]], [[releases_link]], [[egui]], [[tinyskia]],
             [[claude]], and [[mac:name]]: the exact words macOS shows in
             that language (button and pane names, from apple-strings.json)

Pages: the landing page (template.html) and the installation guide
(install.html, at <prefix>install/) in every language, and the privacy
policy (privacy.html) in English.

The build fails if a language file is missing a key, has an extra one, or if
anything is left unresolved, so no page silently falls back to English.
"""
import datetime
import html
import json
import math
import re
import sys
from pathlib import Path

SRC = Path(__file__).resolve().parent
OUT = SRC.parent

BASE = "https://donbuche.github.io/quicksnap/"
VERSION = "0.5.2"
GA_ID = "G-XSWM6BDFHG"  # Google Analytics 4, property "QuickSnap (GitHub Pages)"
REPO = "https://github.com/donbuche/quicksnap"

# Order of the app's language menu: English, the languages of Spain, the
# rest of Europe, then Asia. `prefix` is the folder ("" = site root).
LANGS = [
    # code     prefix    hreflang   og_locale  name                  flag  browser tags            base
    ("en",    "",       "en",      "en_US",   "English",            "gb", ["en"],                "en"),
    ("es",    "es/",    "es",      "es_ES",   "Español",            "es", ["es"],                "es"),
    ("ca",    "ca/",    "ca",      "ca_ES",   "Català",             "ca", ["ca"],                "ca"),
    ("gl",    "gl/",    "gl",      "gl_ES",   "Galego",             "gl", ["gl"],                "gl"),
    ("eu",    "eu/",    "eu",      "eu_ES",   "Euskara",            "eu", ["eu"],                "eu"),
    ("fr",    "fr/",    "fr",      "fr_FR",   "Français",           "fr", ["fr"],                "fr"),
    ("de",    "de/",    "de",      "de_DE",   "Deutsch",            "de", ["de"],                "de"),
    ("it",    "it/",    "it",      "it_IT",   "Italiano",           "it", ["it"],                "it"),
    ("pt",    "pt/",    "pt-PT",   "pt_PT",   "Português",          "pt", ["pt-pt", "pt"],       "pt"),
    ("pt-br", "pt-br/", "pt-BR",   "pt_BR",   "Português (Brasil)", "br", ["pt-br"],             None),
    ("nl",    "nl/",    "nl",      "nl_NL",   "Nederlands",         "nl", ["nl"],                "nl"),
    ("pl",    "pl/",    "pl",      "pl_PL",   "Polski",             "pl", ["pl"],                "pl"),
    ("ja",    "ja/",    "ja",      "ja_JP",   "日本語",              "jp", ["ja"],                "ja"),
    ("zh",    "zh/",    "zh-Hans", "zh_CN",   "简体中文",             "cn", ["zh-cn", "zh-sg", "zh-hans", "zh-hans-cn", "zh"], "zh"),
    ("ko",    "ko/",    "ko",      "ko_KR",   "한국어",              "kr", ["ko"],                "ko"),
]
# What macOS itself calls things, per site language, taken from the system's
# own translation tables (macOS has no Galician or Basque: those pages use
# the Spanish names). "%@" is the app's name.
APPLE = json.loads((SRC / "apple-strings.json").read_text("utf-8"))

LANGS = [dict(zip(("code", "prefix", "hreflang", "og_locale", "name", "flag", "match", "base"), l)) for l in LANGS]


# ---------------------------------------------------------------- flags --
# Small decorative flags (30 x 20), always shown next to the language name.

def star(cx, cy, r, rot=-90):
    pts = []
    for i in range(10):
        rr = r if i % 2 == 0 else r * 0.382
        a = math.radians(rot + i * 36)
        pts.append(f"{cx + rr * math.cos(a):.2f},{cy + rr * math.sin(a):.2f}")
    return " ".join(pts)


def stripes(colours, vertical=False):
    n = len(colours)
    out = []
    for i, c in enumerate(colours):
        if vertical:
            out.append(f'<rect x="{30 * i / n:.3f}" width="{30 / n + 0.05:.3f}" height="20" fill="{c}"/>')
        else:
            out.append(f'<rect y="{20 * i / n:.3f}" width="30" height="{20 / n + 0.05:.3f}" fill="{c}"/>')
    return "".join(out)


def trigram(x, y, angle, pattern):
    # pattern: three rows, True = solid bar, False = broken bar
    parts = []
    for i, solid in enumerate(pattern):
        yy = -1.6 + i * 1.6
        if solid:
            parts.append(f'<rect x="-2.2" y="{yy - 0.5:.2f}" width="4.4" height="1" />')
        else:
            parts.append(f'<rect x="-2.2" y="{yy - 0.5:.2f}" width="1.9" height="1" /><rect x="0.3" y="{yy - 0.5:.2f}" width="1.9" height="1" />')
    return f'<g transform="translate({x} {y}) rotate({angle})" fill="#000">{"".join(parts)}</g>'


FLAGS = {
    "gb": '<rect width="30" height="20" fill="#012169"/>'
          '<path d="M0 0l30 20M30 0L0 20" stroke="#fff" stroke-width="4"/>'
          '<path d="M0 0l30 20M30 0L0 20" stroke="#C8102E" stroke-width="1.5"/>'
          '<path d="M15 0v20M0 10h30" stroke="#fff" stroke-width="6"/>'
          '<path d="M15 0v20M0 10h30" stroke="#C8102E" stroke-width="3.4"/>',
    "es": '<rect width="30" height="20" fill="#AA151B"/><rect y="5" width="30" height="10" fill="#F1BF00"/>',
    "ca": '<rect width="30" height="20" fill="#FCDD09"/>' + "".join(
        f'<rect y="{20 / 9 * k:.3f}" width="30" height="{20 / 9:.3f}" fill="#DA121A"/>' for k in (1, 3, 5, 7)),
    "gl": '<rect width="30" height="20" fill="#fff"/><path d="M0 0L30 20" stroke="#0099CC" stroke-width="5"/>',
    "eu": '<rect width="30" height="20" fill="#D52B1E"/>'
          '<path d="M0 0l30 20M30 0L0 20" stroke="#009B48" stroke-width="3"/>'
          '<path d="M15 0v20M0 10h30" stroke="#fff" stroke-width="3"/>',
    "fr": stripes(["#002654", "#fff", "#CE1126"], vertical=True),
    "de": stripes(["#000", "#DD0000", "#FFCE00"]),
    "it": stripes(["#009246", "#fff", "#CE2B37"], vertical=True),
    "pt": '<rect width="30" height="20" fill="#DA291C"/><rect width="12" height="20" fill="#046A38"/>'
          '<circle cx="12" cy="10" r="4.2" fill="#FFE900"/><rect x="10.4" y="7.8" width="3.2" height="4" rx="1" fill="#fff" stroke="#DA291C" stroke-width=".8"/>',
    "br": '<rect width="30" height="20" fill="#009C3B"/><path d="M15 2.4 27.6 10 15 17.6 2.4 10z" fill="#FFDF00"/>'
          '<circle cx="15" cy="10" r="4.6" fill="#002776"/><path d="M10.6 9.2c3-.9 6.4-.5 8.9 1.2" stroke="#fff" stroke-width=".7" fill="none"/>',
    "nl": stripes(["#AE1C28", "#fff", "#21468B"]),
    "pl": '<rect width="30" height="20" fill="#fff"/><rect y="10" width="30" height="10" fill="#DC143C"/>',
    "jp": '<rect width="30" height="20" fill="#fff"/><circle cx="15" cy="10" r="6" fill="#BC002D"/>',
    "cn": '<rect width="30" height="20" fill="#EE1C25"/>'
          f'<polygon points="{star(5, 5, 3)}" fill="#FFFF00"/>'
          + "".join(f'<polygon points="{star(x, y, 1, rot)}" fill="#FFFF00"/>'
                    for x, y, rot in ((10, 2, -60), (12, 4, -80), (12, 7, -100), (10, 9, -120))),
    "kr": '<rect width="30" height="20" fill="#fff"/>'
          '<g transform="rotate(33.7 15 10)">'
          '<path d="M10 10a5 5 0 0 1 10 0z" fill="#CD2E3A"/><path d="M10 10a5 5 0 0 0 10 0z" fill="#0047A0"/>'
          '<circle cx="12.5" cy="10" r="2.5" fill="#CD2E3A"/><circle cx="17.5" cy="10" r="2.5" fill="#0047A0"/></g>'
          + trigram(6, 4.5, -56.3, (True, True, True))
          + trigram(24, 15.5, -56.3, (False, False, False))
          + trigram(24, 4.5, 56.3, (False, True, False))
          + trigram(6, 15.5, 56.3, (True, False, True)),
}


def flag_sprite():
    return "\n".join(
        f'    <symbol id="flag-{k}" viewBox="0 0 30 20">{v}</symbol>' for k, v in FLAGS.items())


# --------------------------------------------------------------- helpers --

def esc(s):
    return html.escape(s, quote=True)


def plain(s):
    """HTML text -> plain text (for attributes, JSON-LD, the banner)."""
    return html.unescape(re.sub(r"<[^>]+>", "", s))


def fix_amp(s):
    return re.sub(r"&(?!#?\w+;)", "&amp;", s)


def rel(from_lang, to_lang, sub=""):
    """Link from a page of from_lang to the same page (sub, e.g. "install/")
    of to_lang."""
    root = "../" * (from_lang["prefix"] + sub).count("/")
    return (root + to_lang["prefix"] + sub) or "./"


def expand_tokens(text, t, plain_mode=False):
    opt, cmd = t["key_option"], t["key_command"]
    mac = APPLE[t["_code"]]

    def keys(n):
        label = f"{opt} + {cmd} + {n}"
        if plain_mode:
            return label
        return (f'<span class="keys"><span class="sr-only">{esc(label)}</span>'
                f'<kbd aria-hidden="true">⌥</kbd><kbd aria-hidden="true">⌘</kbd><kbd aria-hidden="true">{n}</kbd></span>')

    def link(href, label):
        return label if plain_mode else f'<a href="{href}">{label}</a>'

    table = {
        "cmd": cmd if plain_mode else f'<span class="sr-only">{esc(cmd)}</span><kbd aria-hidden="true">⌘</kbd>',
        "hotkey_heading": (f"⌥⌘5" if plain_mode else
                           f'<span class="grad" aria-hidden="true">⌥⌘5</span><span class="sr-only">{esc(opt)} + {esc(cmd)} + 5</span>'),
        "releases_link": link(REPO + "/releases/latest", t["step1_link"]),
        "egui": link("https://github.com/emilk/egui", "egui"),
        "tinyskia": link("https://github.com/linebender/tiny-skia", "tiny-skia"),
        "claude": link("https://claude.com/claude-code", "Claude Code"),
    }

    def sub(m):
        tok = m.group(1)
        if tok.startswith("keys:"):
            return keys(tok[5:])
        if tok.startswith("mac:"):
            if tok[4:] not in mac:
                sys.exit(f"Unknown macOS string [[{tok}]]")
            # without its final full stop: it is quoted inside our own sentence
            words = mac[tok[4:]].replace("%@", "QuickSnap").rstrip(".。")
            return words if plain_mode else esc(words)
        if tok not in table:
            sys.exit(f"Unknown token [[{tok}]]")
        return table[tok]

    return re.sub(r"\[\[([\w:]+)\]\]", sub, text)


# ------------------------------------------------------------------ build --

def load_texts():
    texts = {}
    reference = json.loads((SRC / "i18n" / "en.json").read_text("utf-8"))
    problems = []
    for lang in LANGS:
        path = SRC / "i18n" / f"{lang['code']}.json"
        if not path.exists():
            problems.append(f"missing file {path.name}")
            continue
        data = json.loads(path.read_text("utf-8"))
        missing = [k for k in reference if k not in data]
        extra = [k for k in data if k not in reference]
        empty = [k for k, v in data.items() if not str(v).strip()]
        if missing or extra or empty:
            problems.append(f"{path.name}: missing {missing} extra {extra} empty {empty}")
        for k, v in data.items():
            tokens_ref = sorted(re.findall(r"\[\[[\w:]+\]\]", reference.get(k, "")))
            tokens = sorted(re.findall(r"\[\[[\w:]+\]\]", v))
            if tokens != tokens_ref:
                problems.append(f"{path.name}: {k} has tokens {tokens}, expected {tokens_ref}")
        data["_code"] = lang["code"]
        texts[lang["code"]] = data
    if problems:
        sys.exit("Translation problems:\n  " + "\n  ".join(problems))
    return texts


def jsonld(lang, t, canonical):
    shots = f"{BASE}assets/shots/{lang['code']}/"
    data = {
        "@context": "https://schema.org",
        "@type": "SoftwareApplication",
        "name": "QuickSnap",
        "description": plain(t["meta_description"]),
        "url": canonical,
        "inLanguage": lang["hreflang"],
        "applicationCategory": "MultimediaApplication",
        "applicationSubCategory": "Screenshot",
        "operatingSystem": "macOS 11+",
        "softwareVersion": VERSION,
        "fileSize": "5MB",
        "downloadUrl": REPO + "/releases/latest/download/QuickSnap.dmg",
        "installUrl": REPO + "/releases/latest",
        "image": BASE + "assets/icon-256.png",
        "screenshot": [shots + "home-dark.jpg", shots + "editor-dark.jpg"],
        "isAccessibleForFree": True,
        "offers": {"@type": "Offer", "price": "0", "priceCurrency": "USD"},
        "author": {"@type": "Person", "name": "Joan Galtés i Moreno"},
        "sameAs": [REPO],
    }
    return json.dumps(data, ensure_ascii=False, indent=2).replace("</", "<\\/")


def render(template, lang, texts, sub=""):
    """sub: "" for the landing page, "install/" for the guide."""
    t = texts[lang["code"]]
    canonical = BASE + lang["prefix"] + sub
    root = "../" * (lang["prefix"] + sub).count("/")

    alternates = [f'<link rel="alternate" hreflang="{l["hreflang"]}" href="{BASE}{l["prefix"]}{sub}">' for l in LANGS]
    alternates.append(f'<link rel="alternate" hreflang="x-default" href="{BASE}{sub}">')
    og_alt = [f'<meta property="og:locale:alternate" content="{l["og_locale"]}">' for l in LANGS if l is not lang]

    def lang_link(l, cls_flag=True, indent="        ", via="menu"):
        cur = ' aria-current="page"' if l is lang else ""
        flag = (f'<svg class="flag" aria-hidden="true" focusable="false"><use href="#flag-{l["flag"]}"/></svg>'
                if cls_flag else "")
        return (f'{indent}<li><a href="{rel(lang, l, sub)}" hreflang="{l["hreflang"]}" lang="{l["hreflang"]}" '
                f'data-lang="{l["code"]}" data-via="{via}"{cur}>{flag}{esc(l["name"])}</a></li>')

    banner = []
    for l in LANGS:
        tl = texts[l["code"]]
        banner.append({
            "code": l["code"], "lang": l["hreflang"], "flag": l["flag"], "href": rel(lang, l, sub),
            "match": l["match"], "base": l["base"],
            "text": esc(plain(tl["banner_text"])), "go": esc(plain(tl["banner_go"])),
            "dismiss": esc(plain(tl["banner_dismiss"])),
        })

    computed = {
        "html_lang": lang["hreflang"],
        "code": lang["code"],
        "canonical": canonical,
        "alternates": "\n".join(alternates),
        "og_locale": lang["og_locale"],
        "og_alternates": "\n".join(og_alt),
        "og_image": f"{BASE}assets/shots/{lang['code']}/home-dark.jpg",
        "jsonld": jsonld(lang, t, canonical),
        "root": root,
        "home": "../" if sub else "./",
        "guide": root + lang["prefix"] + "install/",
        "shots": f"{root}assets/shots/{lang['code']}/",
        "version": VERSION,
        "ga": GA_ID,
        "ga_suffix": GA_ID.split("-", 1)[1],
        "base": BASE,
        "today": datetime.date.today().isoformat(),
        "today_long": datetime.date.today().strftime("%-d %B %Y"),
        "host": BASE.split("/")[2],
        "flag": lang["flag"],
        "lang_name": esc(lang["name"]),
        "flag_sprite": flag_sprite(),
        "lang_menu": "\n".join(lang_link(l) for l in LANGS),
        "lang_pills": "\n".join(lang_link(l, indent="      ", via="pills") for l in LANGS),
        "footer_langs": "\n".join(lang_link(l, cls_flag=False, indent="          ", via="footer") for l in LANGS),
        "langs_json": json.dumps(banner, ensure_ascii=False).replace("</", "<\\/"),
        "js_pause": json.dumps(plain(t["motion_pause"]), ensure_ascii=False),
        "js_play": json.dumps(plain(t["motion_play"]), ensure_ascii=False),
    }

    def sub(m):
        kind, key = m.group(1), m.group(2)
        if kind == "$":
            if key not in computed:
                sys.exit(f"Unknown computed value {{{{${key}}}}}")
            return computed[key]
        if key not in t:
            sys.exit(f"Unknown text key {{{{{key}}}}}")
        if kind == "@":
            return esc(plain(expand_tokens(t[key], t, plain_mode=True)))
        return expand_tokens(fix_amp(t[key]), t)

    page = re.sub(r"\{\{([@$]?)(\w+)\}\}", sub, template)
    page = expand_tokens(page, t)
    leftover = re.findall(r"\{\{[^}]*\}\}|\[\[[^\]]*\]\]", page)
    if leftover:
        sys.exit(f"{lang['code']}: unresolved {leftover[:5]}")
    return page


def sitemap():
    today = datetime.date.today().isoformat()
    links = "".join(f'\n    <xhtml:link rel="alternate" hreflang="{l["hreflang"]}" href="{BASE}{l["prefix"]}"/>' for l in LANGS)
    links += f'\n    <xhtml:link rel="alternate" hreflang="x-default" href="{BASE}"/>'
    urls = "".join(
        f'\n  <url>\n    <loc>{BASE}{l["prefix"]}</loc>\n    <lastmod>{today}</lastmod>{links}\n  </url>' for l in LANGS)
    links = "".join(f'\n    <xhtml:link rel="alternate" hreflang="{l["hreflang"]}" href="{BASE}{l["prefix"]}install/"/>' for l in LANGS)
    links += f'\n    <xhtml:link rel="alternate" hreflang="x-default" href="{BASE}install/"/>'
    urls += "".join(
        f'\n  <url>\n    <loc>{BASE}{l["prefix"]}install/</loc>\n    <lastmod>{today}</lastmod>{links}\n  </url>' for l in LANGS)
    urls += f'\n  <url>\n    <loc>{BASE}privacy/</loc>\n    <lastmod>{today}</lastmod>\n  </url>'
    return ('<?xml version="1.0" encoding="UTF-8"?>\n'
            '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" '
            'xmlns:xhtml="http://www.w3.org/1999/xhtml">' + urls + "\n</urlset>\n")


def main():
    template = (SRC / "template.html").read_text("utf-8")
    texts = load_texts()
    for lang in LANGS:
        shots = OUT / "assets" / "shots" / lang["code"]
        if not (shots / "home-dark.jpg").exists():
            sys.exit(f"Missing screenshots for {lang['code']}: run _src/shots.sh")
        target = OUT / lang["prefix"] / "index.html"
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(render(template, lang, texts), "utf-8")
        print(f"  {lang['code']:6} -> {target.relative_to(OUT)}")
    style = re.search(r"<style>.*?</style>", template, re.S).group(0)
    headscript = re.search(r"<script>\n  // Before first paint.*?</script>", template, re.S).group(0)
    script = re.search(r"<script>\n\(function \(\) \{.*?</script>", template, re.S).group(0)
    # Installation guide: same header, footer and script as the landing page
    guide = ((SRC / "install.html").read_text("utf-8").replace("{{$style}}", style)
             .replace("{{$headscript}}", headscript).replace("{{$script}}", script))
    for lang in LANGS:
        target = OUT / lang["prefix"] / "install" / "index.html"
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(render(guide, lang, texts, "install/"), "utf-8")
        print(f"  {lang['code']:6} -> {target.relative_to(OUT)}")
    # Privacy policy: English only, same look as the landing pages
    privacy = (SRC / "privacy.html").read_text("utf-8").replace("{{$style}}", style).replace("{{$headscript}}", headscript)
    en = dict(LANGS[0], prefix="privacy/")
    target = OUT / "privacy" / "index.html"
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(render(privacy, en, texts), "utf-8")
    print("  privacy -> privacy/index.html")
    (OUT / "sitemap.xml").write_text(sitemap(), "utf-8")
    print("  sitemap.xml")


if __name__ == "__main__":
    main()
