#!/usr/bin/env python3
"""Register one real-font SVG text geometry witness without overwriting.

Usage:
    python3 add_case.py <id> --source <svg> --facts <json>

The source must use the suite's canonical one-run shape. `facts` records the
font-derived evidence Chromium cannot expose: units-per-em, direct cmap glyph
ids/source mappings, and outline ink bounds.
"""

import argparse
import fcntl
import json
import math
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

DIR = Path(__file__).resolve().parent
MANIFEST = DIR / "cases.json"
SVG_NS = "http://www.w3.org/2000/svg"
ID_PATTERN = re.compile(r"^svg-text-[a-z0-9]+(?:-[a-z0-9]+)*$")


def canonical(case: dict) -> str:
    return (
        f'<svg xmlns="{SVG_NS}" width="{case["width"]}" height="{case["height"]}">\n'
        f'  <text x="{case["x"]}" y="{case["y"]}" '
        f'text-anchor="{case["text_anchor"]}" '
        f'font-family="{case["font_family"]}" '
        f'font-size="{case["font_size"]}" fill="{case["fill"]}">'
        f'{case["text"]}</text>\n'
        "</svg>\n"
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("id")
    parser.add_argument("--source", required=True)
    parser.add_argument("--facts", required=True)
    args = parser.parse_args()

    if not ID_PATTERN.fullmatch(args.id):
        sys.exit(f"refused: {args.id!r} is not an svg-text-* kebab-case id")

    manifest_file = MANIFEST.open("r+", encoding="utf-8")
    fcntl.flock(manifest_file.fileno(), fcntl.LOCK_EX)
    suite = json.load(manifest_file)
    source = Path(args.source).read_text()
    if "<!DOCTYPE" in source or "<script" in source or "@font-face" in source:
        sys.exit("refused: geometry sources contain no doctype, script, or font bytes")
    try:
        root = ET.fromstring(source)
    except ET.ParseError as error:
        sys.exit(f"refused: source is not XML: {error}")
    if root.tag != f"{{{SVG_NS}}}svg" or set(root.attrib) != {"width", "height"}:
        sys.exit("refused: expected one width/height SVG root")
    children = list(root)
    if len(children) != 1 or children[0].tag != f"{{{SVG_NS}}}text":
        sys.exit("refused: expected exactly one direct <text> child")
    text = children[0]
    required = {"x", "y", "text-anchor", "font-family", "font-size", "fill"}
    if set(text.attrib) != required or list(text):
        sys.exit("refused: the one text run has only the canonical attributes and no children")
    content = text.text or ""
    if not content or any(not (" " <= character <= "~") for character in content):
        sys.exit("refused: the rung-B witness is non-empty printable ASCII")
    if " ".join(content.split()) != content:
        sys.exit("refused: source text must already be in canonical collapsed-space form")
    if text.attrib["font-family"] != suite["font"]["family"]:
        sys.exit("refused: the source must request the suite's exact family")
    if text.attrib["text-anchor"] not in {"start", "middle", "end"}:
        sys.exit("refused: text-anchor must be start, middle, or end")

    try:
        width = int(root.attrib["width"])
        height = int(root.attrib["height"])
        numbers = {
            name: float(text.attrib[name]) for name in ("x", "y", "font-size")
        }
    except ValueError:
        sys.exit("refused: dimensions and geometry must be plain finite numbers")
    if (
        width <= 0
        or height <= 0
        or not all(math.isfinite(value) for value in numbers.values())
        or numbers["font-size"] <= 0
    ):
        sys.exit("refused: dimensions and finite font size must be positive")

    facts = json.loads(Path(args.facts).read_text())
    if not isinstance(facts, dict) or set(facts) != {
        "units_per_em",
        "glyphs",
        "ink_bounds",
    }:
        sys.exit("refused: facts must name units_per_em, glyphs, and ink_bounds")
    units_per_em = facts["units_per_em"]
    if (
        not isinstance(units_per_em, int)
        or isinstance(units_per_em, bool)
        or not 1 <= units_per_em <= 65535
    ):
        sys.exit("refused: units_per_em must be a positive 16-bit integer")
    if not isinstance(facts["glyphs"], list) or len(facts["glyphs"]) != len(content):
        sys.exit("refused: rung B requires one direct cmap glyph fact per ASCII byte")
    for index, (character, glyph) in enumerate(zip(content, facts["glyphs"])):
        glyph_id = glyph.get("glyph_id") if isinstance(glyph, dict) else None
        expected = {
            "source_utf8_byte": index,
            "source_utf16_index": index,
            "scalar": character,
            "glyph_id": glyph_id,
            "cluster": index,
        }
        if (
            glyph != expected
            or not isinstance(glyph_id, int)
            or isinstance(glyph_id, bool)
            or not 1 <= glyph_id <= 65535
        ):
            sys.exit(f"refused: glyph fact {index} is not the direct source mapping")
    ink_bounds = facts["ink_bounds"]
    if not isinstance(ink_bounds, dict) or set(ink_bounds) != {
        "x",
        "y",
        "width",
        "height",
    }:
        sys.exit("refused: ink_bounds must be one x/y/width/height rectangle")
    if any(
        not isinstance(value, (int, float))
        or isinstance(value, bool)
        or not math.isfinite(value)
        for value in ink_bounds.values()
    ) or ink_bounds["width"] <= 0 or ink_bounds["height"] <= 0:
        sys.exit("refused: ink_bounds must contain finite positive extents")

    case = {
        "id": args.id,
        "source": f"{args.id}.svg",
        "oracle": f"chromium/{args.id}.json",
        "width": width,
        "height": height,
        "text": content,
        "x": text.attrib["x"],
        "y": text.attrib["y"],
        "text_anchor": text.attrib["text-anchor"],
        "font_family": text.attrib["font-family"],
        "font_size": text.attrib["font-size"],
        "fill": text.attrib["fill"],
        "font_facts": facts,
    }
    if source != canonical(case):
        sys.exit("refused: source bytes are not in the canonical geometry-fixture shape")

    fixture = DIR / case["source"]
    oracle = DIR / case["oracle"]
    if fixture.exists() or oracle.exists():
        sys.exit("refused: an existing geometry source or oracle is immutable")
    if any(
        row["id"] == args.id
        or row["source"] == case["source"]
        or row["oracle"] == case["oracle"]
        for row in suite["cases"]
    ):
        sys.exit("refused: id, source, or oracle is already registered")

    suite["cases"].append(case)
    suite["cases"].sort(key=lambda row: row["id"])
    with fixture.open("x", encoding="utf-8") as fixture_file:
        fixture_file.write(source)
    manifest_file.seek(0)
    json.dump(suite, manifest_file, indent=2)
    manifest_file.write("\n")
    manifest_file.truncate()
    manifest_file.flush()
    fcntl.flock(manifest_file.fileno(), fcntl.LOCK_UN)
    manifest_file.close()
    print(f"added {args.id}; run `just text-geometry-bake`")


if __name__ == "__main__":
    main()
