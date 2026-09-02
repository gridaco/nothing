#!/usr/bin/env python3
"""Register one real-font SVG text geometry witness without overwriting.

Usage:
    python3 add_case.py <id> --source <svg> --facts <json>

The source must use the suite's canonical text shape. It may contain direct
text only, or direct text plus flat paint/position `<tspan>` children.
`facts` records the font-derived evidence Chromium cannot expose:
units-per-em, complete source-run/cluster/glyph placement mappings, and outline
ink bounds. Flat paint-only tspans require v3 facts with explicit run tags;
positioned tspans require those source-run facts plus v4 shaping chunks. New
direct text cases retain the v2 placed-glyph shape. The manifest's immutable
pre-T3c cases retain their legacy direct-scalar shape; this command neither
creates nor migrates that historical form.
"""

import argparse
import fcntl
import json
import math
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Optional

DIR = Path(__file__).resolve().parent
MANIFEST = DIR / "cases.json"
SVG_NS = "http://www.w3.org/2000/svg"
ID_PATTERN = re.compile(r"^svg-text-[a-z0-9]+(?:-[a-z0-9]+)*$")
OPAQUE_HEX_COLOR_PATTERN = re.compile(r"^#[0-9a-fA-F]{6}$")
POSITION_LIST_PATTERN = re.compile(
    r"^-?(?:0|[1-9][0-9]*)(?: -?(?:0|[1-9][0-9]*))*$"
)


def direct_scalar(character: str) -> bool:
    codepoint = ord(character)
    return " " <= character <= "~" or any(
        start <= codepoint <= end
        for start, end in (
            (0x00C0, 0x00C5),
            (0x00C7, 0x00CF),
            (0x00D1, 0x00D6),
            (0x00D9, 0x00DD),
            (0x00E0, 0x00E5),
            (0x00E7, 0x00EF),
            (0x00F1, 0x00F6),
            (0x00F9, 0x00FD),
            (0x00FF, 0x00FF),
        )
    )


def admitted_mark(character: str) -> bool:
    return character in {"\u0301", "\u030b"}


def admitted_source(scalars: list[str]) -> bool:
    previous = None
    for character in scalars:
        if direct_scalar(character):
            previous = character
        elif admitted_mark(character):
            if previous is None or not previous.isascii() or not previous.isalpha():
                return False
            previous = character
        else:
            return False
    return True


def canonical(case: dict) -> str:
    if case.get("source_runs"):
        content_bytes = case["text"].encode("utf-8")
        fragments = []
        for run in case["source_runs"]:
            start, end = run["source_utf8"]
            fragment = content_bytes[start:end].decode("utf-8")
            if run["owner"] == "text":
                fragments.append(fragment)
            else:
                positioning = "".join(
                    f' {name}="{run[name]}"'
                    for name in ("x", "y", "dx", "dy")
                    if name in run
                )
                fragments.append(
                    f'<tspan{positioning} fill="{run["fill"]}">{fragment}</tspan>'
                )
        markup = "".join(fragments)
    else:
        markup = case["text"]
    return (
        f'<svg xmlns="{SVG_NS}" width="{case["width"]}" height="{case["height"]}">\n'
        f'  <text x="{case["x"]}" y="{case["y"]}" '
        f'text-anchor="{case["text_anchor"]}" '
        f'font-family="{case["font_family"]}" '
        f'font-size="{case["font_size"]}" fill="{case["fill"]}">'
        f'{markup}</text>\n'
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
    if suite.get("schema_version") != 4:
        sys.exit("refused: unsupported geometry suite schema")
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
    if set(text.attrib) != required:
        sys.exit("refused: the text element has only the canonical attributes")
    if not OPAQUE_HEX_COLOR_PATTERN.fullmatch(text.attrib["fill"]):
        sys.exit("refused: geometry text fill must be an opaque #RRGGBB solid")

    fragments = []
    has_tspan = False
    has_positioning = False

    def append_fragment(
        value: Optional[str],
        owner: str,
        fill: Optional[str] = None,
        positioning: Optional[dict[str, str]] = None,
    ) -> None:
        if value:
            fragment = {"text": value, "owner": owner, "fill": fill}
            if positioning:
                fragment.update(positioning)
            fragments.append(fragment)

    append_fragment(text.text, "text")
    for child in list(text):
        if (
            child.tag != f"{{{SVG_NS}}}tspan"
            or "fill" not in child.attrib
            or not set(child.attrib) <= {"fill", "x", "y", "dx", "dy"}
            or list(child)
        ):
            sys.exit(
                "refused: text children are flat <tspan> elements carrying fill and optional x/y/dx/dy"
            )
        if not OPAQUE_HEX_COLOR_PATTERN.fullmatch(child.attrib["fill"]):
            sys.exit("refused: geometry tspan fill must be an opaque #RRGGBB solid")
        positioning = {
            name: child.attrib[name]
            for name in ("x", "y", "dx", "dy")
            if name in child.attrib
        }
        for name, value in positioning.items():
            if not POSITION_LIST_PATTERN.fullmatch(value):
                sys.exit(
                    f"refused: geometry tspan {name} must be a canonical space-separated integral number list"
                )
        has_tspan = True
        has_positioning = has_positioning or bool(positioning)
        append_fragment(child.text, "tspan", child.attrib["fill"], positioning)
        append_fragment(child.tail, "text")

    content = "".join(fragment["text"] for fragment in fragments)
    scalars = list(content)
    if not scalars or not admitted_source(scalars):
        sys.exit("refused: the witness must stay inside textlayout-v4's exact repertoire")
    if " ".join(content.split()) != content:
        sys.exit("refused: source text must already be in canonical collapsed-space form")
    families = {font["family"] for font in suite["fonts"]}
    if text.attrib["font-family"] not in families:
        sys.exit("refused: the source must request one exact suite family")
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
    required_fact_fields = {
        "schema_version",
        "units_per_em",
        "clusters",
        "glyphs",
        "ink_bounds",
    }
    if has_tspan:
        required_fact_fields.add("source_runs")
    if has_positioning:
        required_fact_fields.add("shaping_chunks")
    if not isinstance(facts, dict) or set(facts) != required_fact_fields:
        sys.exit(
            f"refused: facts must name exactly {sorted(required_fact_fields)}"
        )
    expected_fact_schema = 4 if has_positioning else 3 if has_tspan else 2
    if facts["schema_version"] != expected_fact_schema:
        sys.exit(f"refused: this geometry source requires fact schema {expected_fact_schema}")
    units_per_em = facts["units_per_em"]
    if (
        not isinstance(units_per_em, int)
        or isinstance(units_per_em, bool)
        or not 1 <= units_per_em <= 65535
    ):
        sys.exit("refused: units_per_em must be a positive 16-bit integer")
    clusters = facts["clusters"]
    glyphs = facts["glyphs"]
    if not isinstance(clusters, list) or not clusters:
        sys.exit("refused: facts require at least one shaping cluster")
    if not isinstance(glyphs, list) or not glyphs:
        sys.exit("refused: facts require at least one placed glyph")

    utf8_offsets = [0]
    utf16_offsets = [0]
    for character in scalars:
        utf8_offsets.append(utf8_offsets[-1] + len(character.encode("utf-8")))
        utf16_offsets.append(
            utf16_offsets[-1] + len(character.encode("utf-16-le")) // 2
        )

    parsed_source_runs = []
    source_byte = 0
    for fragment in fragments:
        end = source_byte + len(fragment["text"].encode("utf-8"))
        parsed_source_runs.append(
            {
                "source_utf8": [source_byte, end],
                "owner": fragment["owner"],
                "fill": fragment["fill"],
                **{
                    name: fragment[name]
                    for name in ("x", "y", "dx", "dy")
                    if name in fragment
                },
            }
        )
        source_byte = end
    source_runs = []
    if has_tspan:
        supplied_runs = facts["source_runs"]
        if not isinstance(supplied_runs, list) or len(supplied_runs) != len(parsed_source_runs):
            sys.exit(
                "refused: source-run facts must match every non-empty source fragment"
            )
        for run_index, (parsed, supplied) in enumerate(zip(parsed_source_runs, supplied_runs)):
            if (
                not isinstance(supplied, dict)
                or set(supplied) != {"source_utf8", "tag"}
                or supplied["source_utf8"] != parsed["source_utf8"]
                or not isinstance(supplied["tag"], int)
                or isinstance(supplied["tag"], bool)
                or not 0 <= supplied["tag"] <= 0xFFFFFFFF
            ):
                sys.exit(f"refused: source-run fact {run_index} does not match the source")
            source_runs.append({**parsed, "tag": supplied["tag"]})

    def tag_at(byte_index: int) -> int:
        for run in source_runs:
            if run["source_utf8"][0] <= byte_index < run["source_utf8"][1]:
                return run["tag"]
        raise AssertionError("validated source runs cover every source byte")

    expected_scalar = 0
    expected_glyph = 0
    glyph_clusters = [None] * len(glyphs)
    for cluster_index, cluster in enumerate(clusters):
        cluster_fields = {
            "source_utf8",
            "source_utf16",
            "source_scalars",
            "glyphs",
        }
        if has_tspan:
            cluster_fields.add("source_run_tag")
        if not isinstance(cluster, dict) or set(cluster) != cluster_fields:
            sys.exit(f"refused: cluster fact {cluster_index} has the wrong fields")
        ranges = [
            cluster["source_utf8"],
            cluster["source_utf16"],
            cluster["source_scalars"],
            cluster["glyphs"],
        ]
        if any(
            not isinstance(span, list)
            or len(span) != 2
            or any(not isinstance(value, int) or isinstance(value, bool) for value in span)
            or span[0] >= span[1]
            for span in ranges
        ):
            sys.exit(f"refused: cluster fact {cluster_index} has an invalid range")
        scalar_start, scalar_end = cluster["source_scalars"]
        glyph_start, glyph_end = cluster["glyphs"]
        if (
            scalar_start != expected_scalar
            or glyph_start != expected_glyph
            or scalar_end > len(scalars)
            or glyph_end > len(glyphs)
            or cluster["source_utf8"] != [utf8_offsets[scalar_start], utf8_offsets[scalar_end]]
            or cluster["source_utf16"]
            != [utf16_offsets[scalar_start], utf16_offsets[scalar_end]]
        ):
            sys.exit(f"refused: cluster fact {cluster_index} does not cover source contiguously")
        cluster_scalars = scalars[scalar_start:scalar_end]
        glyph_count = glyph_end - glyph_start
        direct = (
            len(cluster_scalars) == 1
            and direct_scalar(cluster_scalars[0])
            and glyph_count == 1
        )
        one_mark = (
            len(cluster_scalars) == 2
            and cluster_scalars[0].isascii()
            and cluster_scalars[0].isalpha()
            and admitted_mark(cluster_scalars[1])
            and glyph_count in {1, 2}
        )
        if not direct and not one_mark:
            sys.exit(
                f"refused: cluster fact {cluster_index} is outside the v4 cardinality"
            )
        if has_tspan and cluster["source_run_tag"] != tag_at(cluster["source_utf8"][0]):
            sys.exit(
                f"refused: cluster fact {cluster_index} does not carry its first scalar's source-run tag"
            )
        for glyph_index in range(glyph_start, glyph_end):
            glyph_clusters[glyph_index] = cluster_index
        expected_scalar = scalar_end
        expected_glyph = glyph_end
    if expected_scalar != len(scalars) or expected_glyph != len(glyphs):
        sys.exit("refused: cluster facts do not cover the whole source and glyph stream")

    pen_x = 0
    for glyph_index, glyph in enumerate(glyphs):
        glyph_fields = {
            "glyph_id",
            "cluster_index",
            "x",
            "offset_x",
            "offset_y",
            "advance",
        }
        if has_tspan:
            glyph_fields.add("source_run_tag")
        if not isinstance(glyph, dict) or set(glyph) != glyph_fields:
            sys.exit(f"refused: glyph fact {glyph_index} has the wrong fields")
        glyph_id = glyph["glyph_id"]
        cluster_index = glyph["cluster_index"]
        placement = [glyph["x"], glyph["offset_x"], glyph["offset_y"], glyph["advance"]]
        if (
            not isinstance(glyph_id, int)
            or isinstance(glyph_id, bool)
            or not 1 <= glyph_id <= 65535
            or not isinstance(cluster_index, int)
            or isinstance(cluster_index, bool)
            or cluster_index != glyph_clusters[glyph_index]
            or any(
                not isinstance(value, (int, float))
                or isinstance(value, bool)
                or not math.isfinite(value)
                for value in placement
            )
            or glyph["advance"] < 0
            or glyph["x"] != pen_x
        ):
            sys.exit(f"refused: glyph fact {glyph_index} has invalid identity or placement")
        cluster_glyphs = clusters[cluster_index]["glyphs"]
        glyph_in_cluster = glyph_index - cluster_glyphs[0]
        if cluster_glyphs[1] - cluster_glyphs[0] == 1:
            valid_offset = glyph["offset_x"] == 0 and glyph["offset_y"] == 0
        elif glyph_in_cluster == 0:
            valid_offset = glyph["offset_x"] == 0 and glyph["offset_y"] == 0
        else:
            valid_offset = glyph["advance"] == 0
        if not valid_offset:
            sys.exit(f"refused: glyph fact {glyph_index} is outside v4 mark placement")
        if has_tspan and glyph["source_run_tag"] != clusters[cluster_index]["source_run_tag"]:
            sys.exit(f"refused: glyph fact {glyph_index} disagrees with its cluster's source-run tag")
        pen_x += glyph["advance"]

    if has_positioning:
        boundaries = {0, len(content.encode("utf-8"))}
        for fragment in parsed_source_runs:
            fragment_bytes = content.encode("utf-8")[
                fragment["source_utf8"][0] : fragment["source_utf8"][1]
            ]
            fragment_text = fragment_bytes.decode("utf-8")
            fragment_offsets = []
            offset = 0
            for character in fragment_text:
                fragment_offsets.append(fragment["source_utf8"][0] + offset)
                offset += len(character.encode("utf-8"))
            for name in ("x", "y"):
                if name in fragment:
                    boundaries.update(
                        fragment_offsets[: len(fragment[name].split(" "))]
                    )
        expected_source_chunks = [
            [start, end]
            for start, end in zip(
                sorted(boundaries), sorted(boundaries)[1:], strict=False
            )
            if start < end
        ]
        shaping_chunks = facts["shaping_chunks"]
        if (
            not isinstance(shaping_chunks, list)
            or len(shaping_chunks) != len(expected_source_chunks)
        ):
            sys.exit(
                "refused: v4 shaping chunks must match every consumed x/y boundary"
            )
        cluster_starts = [cluster["source_utf8"][0] for cluster in clusters]
        cluster_starts.append(len(content.encode("utf-8")))
        canonical_origin = 0
        for chunk_index, (chunk, expected_source) in enumerate(
            zip(shaping_chunks, expected_source_chunks, strict=True)
        ):
            fields = {
                "source_utf8",
                "source_utf16",
                "source_scalars",
                "clusters",
                "glyphs",
                "origin_x",
                "advance",
            }
            if not isinstance(chunk, dict) or set(chunk) != fields:
                sys.exit(f"refused: shaping chunk fact {chunk_index} has the wrong fields")
            if chunk["source_utf8"] != expected_source:
                sys.exit(
                    f"refused: shaping chunk fact {chunk_index} disagrees with authored x/y boundaries"
                )
            try:
                scalar_start = utf8_offsets.index(expected_source[0])
                scalar_end = utf8_offsets.index(expected_source[1])
                cluster_start = cluster_starts.index(expected_source[0])
                cluster_end = cluster_starts.index(expected_source[1])
            except ValueError:
                sys.exit(
                    f"refused: shaping chunk fact {chunk_index} splits a scalar or shaping cluster"
                )
            glyph_start = clusters[cluster_start]["glyphs"][0]
            glyph_end = clusters[cluster_end - 1]["glyphs"][1]
            advance = sum(
                glyph["advance"] for glyph in glyphs[glyph_start:glyph_end]
            )
            expected = {
                "source_utf8": expected_source,
                "source_utf16": [utf16_offsets[scalar_start], utf16_offsets[scalar_end]],
                "source_scalars": [scalar_start, scalar_end],
                "clusters": [cluster_start, cluster_end],
                "glyphs": [glyph_start, glyph_end],
                "origin_x": canonical_origin,
                "advance": advance,
            }
            if chunk != expected:
                sys.exit(
                    f"refused: shaping chunk fact {chunk_index} does not match the pinned artifact partition"
                )
            canonical_origin += advance
    elif "shaping_chunks" in facts:
        sys.exit("refused: only positioned v4 facts carry shaping_chunks")

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
    if has_tspan:
        case["source_runs"] = source_runs
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
