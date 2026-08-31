#!/usr/bin/env -S pnpm --filter @grida/reftest exec tsx
/**
 * Bake the committed Chromium oracles for the `<text>` cell suite.
 *
 * The fixture is the document; the font is the environment. Each source is
 * captured with the pinned face declared inline as an `@font-face`, which is
 * the same identity the engine receives as a `textlayout::Environment` — so
 * neither side reads a font ambiently, and the committed `.svg` carries no
 * font bytes. See this directory's README.
 *
 * The font is verified against the digest `cases.json` declares before any
 * capture: a font that is not the pinned identity must never silently become
 * the baseline.
 *
 * Existing oracle pixels are verification-only: a differing image fails
 * instead of silently blessing a new baseline; missing oracles are created.
 *
 * Run: pnpm -C packages/grida-reftest exec tsx fixtures/web-first/text/bake_chromium.ts
 */

import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { exit } from "node:process";
import { fileURLToPath } from "node:url";

import { PNG } from "pngjs";

import {
  captureFirstSvg,
  declareFontInSvg,
  deterministicContext,
  launchDeterministicChromium,
} from "../chromium_capture";

interface Case {
  id: string;
  source: string;
  oracle: string;
  width: number;
  height: number;
}

interface Suite {
  schema_version: number;
  font: { family: string; path: string; sha256: string };
  cases: Case[];
}

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const DIR = dirname(SCRIPT_PATH);
const SUITE_PATH = join(DIR, "cases.json");
const OUT_MANIFEST = join(DIR, "oracle-bake.json");
const CAPTURE_MODULE = "../chromium_capture.ts";

function sha256(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

function assertSamePixels(existing: Buffer, fresh: Buffer, id: string): void {
  const a = PNG.sync.read(existing);
  const b = PNG.sync.read(fresh);
  if (a.width !== b.width || a.height !== b.height || !a.data.equals(b.data)) {
    throw new Error(`${id}: fresh Chromium pixels differ from the committed oracle`);
  }
}

async function main(): Promise<void> {
  const suiteBytes = await readFile(SUITE_PATH);
  const suite = JSON.parse(suiteBytes.toString("utf8")) as Suite;
  if (suite.schema_version !== 1 || suite.cases.length === 0) {
    throw new Error("unsupported or empty text suite");
  }
  const ids = new Set<string>();
  const sources = new Set<string>();
  const oracles = new Set<string>();
  let previousId = "";
  for (const fixture of suite.cases) {
    if (fixture.id <= previousId) {
      throw new Error("text cases must have unique ids in sorted order");
    }
    if (
      ids.has(fixture.id) ||
      sources.has(fixture.source) ||
      oracles.has(fixture.oracle)
    ) {
      throw new Error(`${fixture.id}: duplicate id, source, or oracle`);
    }
    if (fixture.width <= 0 || fixture.height <= 0) {
      throw new Error(`${fixture.id}: dimensions must be positive`);
    }
    ids.add(fixture.id);
    sources.add(fixture.source);
    oracles.add(fixture.oracle);
    previousId = fixture.id;
  }

  // The font is an identity, not a path: verify before any capture.
  const fontBytes = await readFile(join(DIR, suite.font.path));
  const fontDigest = sha256(fontBytes);
  if (fontDigest !== suite.font.sha256) {
    throw new Error(
      `declared font digest ${suite.font.sha256} does not match the bytes at ` +
        `${suite.font.path} (${fontDigest})`,
    );
  }
  const scriptBytes = await readFile(SCRIPT_PATH);
  const captureBytes = await readFile(join(DIR, CAPTURE_MODULE));
  const browser = await launchDeterministicChromium();
  const browserVersion = browser.version();
  const context = await deterministicContext(browser);

  const records: unknown[] = [];
  try {
    for (const fixture of suite.cases) {
      const sourcePath = join(DIR, fixture.source);
      const sourceBytes = await readFile(sourcePath);
      const declared = declareFontInSvg(
        sourceBytes.toString("utf8"),
        suite.font.family,
        fontBytes,
      );

      const page = await context.newPage();
      const capture = {
        media: "image/svg+xml" as const,
        source: Buffer.from(declared),
        width: fixture.width,
        height: fixture.height,
        label: fixture.id,
      };

      const first = await captureFirstSvg(page, capture);
      const second = await captureFirstSvg(page, capture);
      await page.close();
      if (!first.equals(second)) {
        throw new Error(`${fixture.id}: Chromium capture is not byte-deterministic`);
      }

      const decoded = PNG.sync.read(first);
      if (decoded.width !== fixture.width || decoded.height !== fixture.height) {
        throw new Error(
          `${fixture.id}: expected ${fixture.width}x${fixture.height}, got ${decoded.width}x${decoded.height}`,
        );
      }

      const oraclePath = join(DIR, fixture.oracle);
      if (existsSync(oraclePath)) {
        assertSamePixels(await readFile(oraclePath), first, fixture.id);
      } else {
        await mkdir(dirname(oraclePath), { recursive: true });
        await writeFile(oraclePath, first, { flag: "wx" });
        console.log(`created ${fixture.oracle}`);
      }

      records.push({
        id: fixture.id,
        source: fixture.source,
        source_sha256: sha256(sourceBytes),
        oracle: fixture.oracle,
        oracle_sha256: sha256(await readFile(oraclePath)),
        width: fixture.width,
        height: fixture.height,
      });
    }
  } finally {
    await context.close();
    await browser.close();
  }

  const manifest = {
    schema_version: 1,
    kind: "chromium-svg-text-oracle",
    browser_version: browserVersion,
    suite: "cases.json",
    suite_sha256: sha256(suiteBytes),
    bake_script: "bake_chromium.ts",
    bake_script_sha256: sha256(scriptBytes),
    capture_module: CAPTURE_MODULE,
    capture_module_sha256: sha256(captureBytes),
    font: {
      family: suite.font.family,
      sha256: suite.font.sha256,
      declaration: "inline @font-face injected as the root svg's first child",
    },
    capture_policy: {
      viewport: "the fixture's declared size as the initial viewport",
      device_scale_factor: 1,
      javascript: "disabled",
      network: "aborted",
      raster_posture: "-webkit-font-smoothing: none, carried by each fixture",
      comparison: "full RGBA byte-exact, no tolerance",
      fresh_captures_per_case: 2,
    },
    records,
  };
  await writeFile(OUT_MANIFEST, `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(`baked ${records.length} text cells with ${browserVersion}`);
}

main().catch((error) => {
  console.error(error);
  exit(1);
});
