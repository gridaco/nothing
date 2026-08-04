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

import { chromium, type Page } from "@playwright/test";
import { PNG } from "pngjs";

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

function sha256(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

/**
 * Declare the pinned face to the document, exactly as the host declares it
 * to the engine. The rule is injected as the root `<svg>`'s first child so
 * the committed source stays font-free.
 */
function withFontDeclared(source: string, family: string, fontDataUrl: string): string {
  const rule = `<style>@font-face{font-family:"${family}";src:url(${fontDataUrl});}</style>`;
  const rootEnd = source.indexOf(">");
  if (rootEnd < 0 || !source.slice(0, rootEnd).includes("<svg")) {
    throw new Error("fixture must open with an <svg> root element");
  }
  return source.slice(0, rootEnd + 1) + rule + source.slice(rootEnd + 1);
}

async function capture(page: Page, fixture: Case, source: string): Promise<Buffer> {
  await page.setViewportSize({ width: fixture.width, height: fixture.height });
  const dataUrl = `data:image/svg+xml;base64,${Buffer.from(source).toString("base64")}`;
  await page.goto(dataUrl, { waitUntil: "load" });

  const svg = page.locator("svg").first();
  if ((await svg.count()) !== 1) {
    throw new Error(`${fixture.id}: expected a first <svg> element`);
  }
  const box = await svg.boundingBox();
  if (!box || box.width !== fixture.width || box.height !== fixture.height) {
    throw new Error(
      `${fixture.id}: unexpected SVG box ${JSON.stringify(box)}; expected ${fixture.width}x${fixture.height}`,
    );
  }
  return svg.screenshot({ omitBackground: true, type: "png" });
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

  // The font is an identity, not a path: verify before any capture.
  const fontBytes = await readFile(join(DIR, suite.font.path));
  const fontDigest = sha256(fontBytes);
  if (fontDigest !== suite.font.sha256) {
    throw new Error(
      `declared font digest ${suite.font.sha256} does not match the bytes at ` +
        `${suite.font.path} (${fontDigest})`,
    );
  }
  const fontDataUrl = `data:font/ttf;base64,${fontBytes.toString("base64")}`;

  const scriptBytes = await readFile(SCRIPT_PATH);
  const browser = await chromium.launch({
    args: ["--no-sandbox", "--disable-setuid-sandbox"],
  });
  const browserVersion = browser.version();
  const context = await browser.newContext({
    javaScriptEnabled: false,
    viewport: { width: 1280, height: 720 },
    deviceScaleFactor: 1,
    colorScheme: "light",
    locale: "en-US",
    timezoneId: "UTC",
  });
  await context.route("**/*", (route) => route.abort());
  const page = await context.newPage();

  const records: unknown[] = [];
  try {
    for (const fixture of suite.cases) {
      const sourcePath = join(DIR, fixture.source);
      const sourceBytes = await readFile(sourcePath);
      const declared = withFontDeclared(
        sourceBytes.toString("utf8"),
        suite.font.family,
        fontDataUrl,
      );

      const first = await capture(page, fixture, declared);
      const second = await capture(page, fixture, declared);
      assertSamePixels(first, second, `${fixture.id} (repeat capture)`);

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
        await writeFile(oraclePath, first);
        console.log(`created ${fixture.oracle}`);
      }

      records.push({
        id: fixture.id,
        source: fixture.source,
        source_sha256: sha256(sourceBytes),
        oracle: fixture.oracle,
        oracle_sha256: sha256(await readFile(oraclePath)),
      });
    }
  } finally {
    await browser.close();
  }

  const manifest = {
    kind: "chromium-svg-text-oracle",
    browser_version: browserVersion,
    suite: "cases.json",
    suite_sha256: sha256(suiteBytes),
    bake_script: "bake_chromium.ts",
    bake_script_sha256: sha256(scriptBytes),
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
