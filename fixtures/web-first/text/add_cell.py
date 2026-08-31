#!/usr/bin/env python3
"""Add one cell to the dedicated Web-first text suite.

The document is copied into this directory and a sorted `cases.json` row is
added. Existing fixtures and rows are never overwritten. Chromium pixels are
created separately by `just text-bake`, whose baker verifies every existing
oracle before writing any missing one.

Usage:
    python3 add_cell.py <id> --source <path|-> [--width N] [--height N]
"""

import argparse
import json
import re
import sys
from pathlib import Path

DIR = Path(__file__).resolve().parent
MANIFEST = DIR / "cases.json"
ID_PATTERN = re.compile(r"^svg-text-[a-z0-9]+(?:-[a-z0-9]+)*$")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("id", help="text cell id, kebab-case and prefixed svg-text-")
    parser.add_argument("--source", required=True, help="path to the source, or - for stdin")
    parser.add_argument("--width", type=int, default=100)
    parser.add_argument("--height", type=int, default=100)
    args = parser.parse_args()

    if not ID_PATTERN.match(args.id):
        sys.exit(f"refused: id {args.id!r} is not an svg-text-* kebab-case id")
    if args.width <= 0 or args.height <= 0:
        sys.exit("refused: dimensions must be positive")

    body = sys.stdin.read() if args.source == "-" else Path(args.source).read_text()
    if "<svg" not in body or "<text" not in body:
        sys.exit("refused: a text cell needs both <svg> and <text>")
    if "@font-face" in body or "data:font" in body:
        sys.exit("refused: the font is the declared bake environment, not fixture bytes")
    if not body.endswith("\n"):
        body += "\n"

    fixture = DIR / f"{args.id}.svg"
    if fixture.exists():
        sys.exit(f"refused: {fixture.name} already exists")

    manifest = json.loads(MANIFEST.read_text())
    cases = manifest["cases"]
    source_name = fixture.name
    if any(case["id"] == args.id or case["source"] == source_name for case in cases):
        sys.exit(f"refused: {args.id!r} is already enumerated in cases.json")

    cases.append(
        {
            "id": args.id,
            "source": source_name,
            "oracle": f"chromium/{args.id}.png",
            "width": args.width,
            "height": args.height,
        }
    )
    cases.sort(key=lambda case: case["id"])

    fixture.write_text(body)
    MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n")
    print(
        f"added {args.id} ({args.width}x{args.height}); "
        "run `just text-bake` to create its oracle"
    )


if __name__ == "__main__":
    main()
