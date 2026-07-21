#!/usr/bin/env -S pnpm --filter @grida/reftest exec tsx
/**
 * Bake the committed Chromium oracle for the Web-first prototype fixture.
 *
 * Renders `svg-currentcolor-rect.svg` in headless Chromium at
 * deviceScaleFactor=1 (so SVG user units map 1:1 to device pixels), captures a
 * byte-deterministic 64x64 PNG with `omitBackground`, and records provenance
 * (browser version, source sha256, oracle sha256, this script's sha256).
 *
 * This reuses the technique of `crates/grida_dev/scripts/scoreboard_bake_chromium.ts`
 * (DSF=1, JS/network disabled, deterministic capture) WITHOUT the scoreboard's
 * corpus contract or closed-shape boundary — this fixture uses `style="color"`
 * + `fill="currentColor"` deliberately, which the scoreboard boundary forbids.
 *
 * Create-new only: it refuses to overwrite an existing oracle or manifest.
 *
 * Run:  pnpm -C packages/grida-reftest exec tsx fixtures/web-first/bake_chromium.ts
 */

import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { exit } from "node:process";
import { fileURLToPath } from "node:url";

import { chromium } from "@playwright/test";

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const DIR = dirname(SCRIPT_PATH);
const SOURCE = join(DIR, "svg-currentcolor-rect.svg");
const OUT_PNG = join(DIR, "chromium", "svg-currentcolor-rect.png");
const OUT_MANIFEST = join(DIR, "oracle-bake.json");
const WIDTH = 64;
const HEIGHT = 64;

function sha256(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

async function main(): Promise<void> {
  if (existsSync(OUT_PNG)) {
    throw new Error(`oracle already exists: ${OUT_PNG} (bake is create-new)`);
  }

  const source = await readFile(SOURCE);
  const scriptBytes = await readFile(SCRIPT_PATH);

  const browser = await chromium.launch({
    args: ["--no-sandbox", "--disable-setuid-sandbox"],
  });
  const browserVersion = browser.version();
  const context = await browser.newContext({
    javaScriptEnabled: false,
    viewport: { width: WIDTH, height: HEIGHT },
    deviceScaleFactor: 1,
    colorScheme: "light",
    locale: "en-US",
    timezoneId: "UTC",
  });
  await context.route("**/*", (route) => route.abort());

  const page = await context.newPage();
  const dataUrl = `data:image/svg+xml;base64,${source.toString("base64")}`;
  await page.goto(dataUrl, { waitUntil: "load" });

  const observed = await page.evaluate(() => {
    const root = document.documentElement as unknown as SVGSVGElement;
    return {
      namespace: root.namespaceURI,
      width: root.width.baseVal.value,
      height: root.height.baseVal.value,
    };
  });
  if (
    observed.namespace !== "http://www.w3.org/2000/svg" ||
    observed.width !== WIDTH ||
    observed.height !== HEIGHT
  ) {
    throw new Error(`unexpected SVG root: ${JSON.stringify(observed)}`);
  }

  const capture = () =>
    page.screenshot({
      clip: { x: 0, y: 0, width: WIDTH, height: HEIGHT },
      omitBackground: true,
      type: "png",
    });
  const first = await capture();
  const second = await capture();
  if (!first.equals(second)) {
    throw new Error("Chromium bake is not byte-deterministic");
  }

  await context.close();
  await browser.close();

  await mkdir(dirname(OUT_PNG), { recursive: true });
  await writeFile(OUT_PNG, first, { flag: "wx" });

  const manifest = {
    schema_version: 0,
    fixture: "svg-currentcolor-rect",
    kind: "chromium",
    browser_version: browserVersion,
    bake_script_sha256: sha256(scriptBytes),
    source: "svg-currentcolor-rect.svg",
    source_sha256: sha256(source),
    oracle: "chromium/svg-currentcolor-rect.png",
    oracle_sha256: sha256(first),
    capture: {
      width: WIDTH,
      height: HEIGHT,
      device_scale_factor: 1,
      omit_background: true,
      source_transport: "data-url-from-file-bytes",
      javascript_enabled: false,
      network_enabled: false,
    },
  };
  await writeFile(OUT_MANIFEST, `${JSON.stringify(manifest, null, 2)}\n`, {
    flag: "wx",
  });

  console.log(`Chromium ${browserVersion}: baked svg-currentcolor-rect`);
  console.log(`  ${OUT_PNG}`);
  console.log(`  ${OUT_MANIFEST}`);
}

main().catch((error: unknown) => {
  console.error(error instanceof Error ? error.message : String(error));
  exit(1);
});
