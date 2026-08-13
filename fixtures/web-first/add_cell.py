#!/usr/bin/env python3
"""Add a cell to the Web-first primitive suite.

Writes the fixture source (refusing to overwrite) and inserts its
`primitives.json` entry in sorted order (refusing a duplicate id or source).
The oracle is NOT created here — run `just bake` after, which creates the
missing oracle and verifies every existing one byte-for-byte (the
never-overwrite law lives in the baker, `bake_chromium.ts`).

Usage:
    python3 add_cell.py <id> --svg <path|-> [--width N] [--height N]

The id becomes `<id>.svg` / `chromium/<id>.png`. `--svg -` reads the source
from stdin. Entries default to the 64x64 `standalone-svg` shape every recent
cell uses; pass --width/--height for other dims. An `html-inline-svg` cell is
rare enough to author by hand.
"""

import argparse
import json
import re
import sys
from pathlib import Path

DIR = Path(__file__).resolve().parent
MANIFEST = DIR / "primitives.json"
ID_PATTERN = re.compile(r"^[a-z0-9]+(-[a-z0-9]+)*$")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("id", help="cell id, kebab-case (e.g. svg-stroke-cap-css-butt)")
    parser.add_argument("--svg", required=True, help="path to the SVG source, or - for stdin")
    parser.add_argument("--width", type=int, default=64)
    parser.add_argument("--height", type=int, default=64)
    args = parser.parse_args()

    if not ID_PATTERN.match(args.id):
        sys.exit(f"refused: id {args.id!r} is not kebab-case")

    body = sys.stdin.read() if args.svg == "-" else Path(args.svg).read_text()
    if "<svg" not in body:
        sys.exit("refused: source has no <svg> element")
    if not body.endswith("\n"):
        body += "\n"

    fixture_path = DIR / f"{args.id}.svg"
    if fixture_path.exists():
        sys.exit(f"refused: {fixture_path.name} already exists")

    manifest = json.loads(MANIFEST.read_text())
    fixtures = manifest["fixtures"]
    taken = {f["id"] for f in fixtures} | {f["source"] for f in fixtures}
    if args.id in taken or f"{args.id}.svg" in taken:
        sys.exit(f"refused: {args.id!r} already enumerated in primitives.json")

    fixtures.append(
        {
            "id": args.id,
            "source": f"{args.id}.svg",
            "entry": "standalone-svg",
            "oracle": f"chromium/{args.id}.png",
            "width": args.width,
            "height": args.height,
        }
    )
    fixtures.sort(key=lambda f: f["id"])

    fixture_path.write_text(body)
    MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"added {args.id} ({args.width}x{args.height}); run `just bake` to create its oracle")


if __name__ == "__main__":
    main()
