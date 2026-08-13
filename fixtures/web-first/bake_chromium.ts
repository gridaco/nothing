#!/usr/bin/env -S pnpm --filter @grida/reftest exec tsx
/**
 * Bake the committed Chromium oracles for the Web-first primitive suite.
 *
 * Every root-level HTML/SVG fixture is enumerated by `primitives.json`.
 * Standalone SVG and the first inline SVG in HTML are captured as SVG-local
 * rasters at deviceScaleFactor=1. Each capture is repeated byte-for-byte.
 * Existing oracle pixels are verification-only: a differing image fails
 * instead of silently blessing a new baseline; missing oracles are created.
 *
 * This reuses the scoreboard bake's deterministic browser posture WITHOUT
 * invoking the sealed scoreboard or producing a similarity score.
 *
 * Run: pnpm -C packages/grida-reftest exec tsx fixtures/web-first/bake_chromium.ts
 */

import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { exit } from "node:process";
import { fileURLToPath } from "node:url";

import { type Page } from "@playwright/test";
import { PNG } from "pngjs";

import {
  captureFirstSvg,
  deterministicContext,
  launchDeterministicChromium,
} from "./chromium_capture";

type Entry = "standalone-svg" | "html-inline-svg";

interface Primitive {
  id: string;
  source: string;
  entry: Entry;
  oracle: string;
  width: number;
  height: number;
  /**
   * Comparison-side gate policy, carried in the suite but never read here:
   * capture is always the browser's own pixels. See
   * `crates/websem/tests/reftest_oracle.rs` for its meaning.
   */
  tolerance?: unknown;
}

interface PrimitiveSuite {
  schema_version: number;
  fixtures: Primitive[];
}

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const DIR = dirname(SCRIPT_PATH);
const SUITE_PATH = join(DIR, "primitives.json");
const OUT_MANIFEST = join(DIR, "oracle-bake.json");

function sha256(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

function assertDimensions(png: Buffer, fixture: Primitive): void {
  const decoded = PNG.sync.read(png);
  if (decoded.width !== fixture.width || decoded.height !== fixture.height) {
    throw new Error(
      `${fixture.id}: expected ${fixture.width}x${fixture.height}, got ${decoded.width}x${decoded.height}`,
    );
  }
}

function assertSamePixels(existing: Buffer, fresh: Buffer, id: string): void {
  const a = PNG.sync.read(existing);
  const b = PNG.sync.read(fresh);
  if (a.width !== b.width || a.height !== b.height || !a.data.equals(b.data)) {
    throw new Error(`${id}: fresh Chromium pixels differ from the committed oracle`);
  }
}

async function captureSvg(page: Page, fixture: Primitive, source: Buffer): Promise<Buffer> {
  // The posture lives in `chromium_capture.ts` — the one module probes share
  // — and its hash is recorded in the manifest below.
  return captureFirstSvg(page, {
    media: fixture.entry === "standalone-svg" ? "image/svg+xml" : "text/html",
    source,
    width: fixture.width,
    height: fixture.height,
    label: fixture.id,
  });
}

async function main(): Promise<void> {
  const suiteBytes = await readFile(SUITE_PATH);
  const suite = JSON.parse(suiteBytes.toString("utf8")) as PrimitiveSuite;
  if (suite.schema_version !== 1 || suite.fixtures.length === 0) {
    throw new Error("unsupported or empty primitive suite");
  }

  const scriptBytes = await readFile(SCRIPT_PATH);
  const captureBytes = await readFile(join(DIR, "chromium_capture.ts"));
  const browser = await launchDeterministicChromium();
  const browserVersion = browser.version();
  const context = await deterministicContext(browser);

  const records: Array<Record<string, unknown>> = [];
  for (const fixture of suite.fixtures) {
    const sourcePath = join(DIR, fixture.source);
    const oraclePath = join(DIR, fixture.oracle);
    const source = await readFile(sourcePath);
    const page = await context.newPage();
    const first = await captureSvg(page, fixture, source);
    const second = await captureSvg(page, fixture, source);
    await page.close();

    if (!first.equals(second)) {
      throw new Error(`${fixture.id}: Chromium capture is not byte-deterministic`);
    }
    assertDimensions(first, fixture);

    await mkdir(dirname(oraclePath), { recursive: true });
    let oracle = first;
    if (existsSync(oraclePath)) {
      oracle = await readFile(oraclePath);
      assertSamePixels(oracle, first, fixture.id);
    } else {
      await writeFile(oraclePath, first, { flag: "wx" });
    }

    records.push({
      id: fixture.id,
      source: fixture.source,
      source_sha256: sha256(source),
      oracle: fixture.oracle,
      oracle_sha256: sha256(oracle),
      width: fixture.width,
      height: fixture.height,
    });
    console.log(`verified ${fixture.id} (${fixture.width}x${fixture.height})`);
  }

  await context.close();
  await browser.close();

  const manifest = {
    schema_version: 1,
    kind: "chromium-primitive-suite",
    browser_version: browserVersion,
    bake_script_sha256: sha256(scriptBytes),
    capture_module_sha256: sha256(captureBytes),
    suite: "primitives.json",
    suite_sha256: sha256(suiteBytes),
    capture: {
      device_scale_factor: 1,
      omit_background: true,
      source_transport: "data-url-from-file-bytes",
      javascript_enabled: false,
      network_enabled: false,
      target: "first-svg-element",
      viewport: {
        "standalone-svg": "fixture-declared-dims (the initial viewport)",
        "html-inline-svg": "1280x720",
      },
    },
    fixtures: records,
  };
  await writeFile(OUT_MANIFEST, `${JSON.stringify(manifest, null, 2)}\n`, {
    flag: "w",
  });
  console.log(`Chromium ${browserVersion}: verified ${records.length} primitive oracles`);
}

main().catch((error: unknown) => {
  console.error(error instanceof Error ? error.message : String(error));
  exit(1);
});
