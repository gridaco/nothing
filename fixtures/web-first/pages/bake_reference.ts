#!/usr/bin/env -S pnpm --filter @grida/reftest exec tsx
/**
 * Bake regenerable Chromium reference renders for the Web-first page harness.
 *
 * These are the *target* renders — the ground truth the Web-first engine grows
 * toward. The engine renders none of these pages today (it does solid-fill SVG
 * rects only); the references stand as the harness's ground truth.
 *
 * For every `fixtures/web-first/pages/*.html`, this renders the page in headless
 * Chromium at deviceScaleFactor=1 with a fixed 1200px-wide viewport, captures a
 * full-page PNG, and records provenance. ALL http(s) requests are ABORTED, so a
 * page that needs any external resource renders visibly broken — the bake is a
 * self-containment check as much as a render.
 *
 * These are version- and platform-dependent target snapshots (text uses
 * system fonts), regenerable — not a zero-tolerance oracle.
 *
 * Run:  pnpm -C packages/grida-reftest exec tsx "$(pwd)/fixtures/web-first/pages/bake_reference.ts"
 */

import { createHash } from "node:crypto";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";
import { exit } from "node:process";

import { chromium } from "@playwright/test";
import { PNG } from "pngjs";

const DIR = dirname(fileURLToPath(import.meta.url));
const REF = join(DIR, "reference");
const VIEWPORT_WIDTH = 1200;

function sha256(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

async function main(): Promise<void> {
  const files = (await readdir(DIR)).filter((f) => f.endsWith(".html")).sort();
  if (files.length === 0) throw new Error("no page fixtures found");
  await mkdir(REF, { recursive: true });

  const browser = await chromium.launch({
    args: ["--no-sandbox", "--disable-setuid-sandbox"],
  });
  const browserVersion = browser.version();
  const context = await browser.newContext({
    javaScriptEnabled: false,
    viewport: { width: VIEWPORT_WIDTH, height: 900 },
    deviceScaleFactor: 1,
    colorScheme: "light",
    locale: "en-US",
    timezoneId: "UTC",
  });
  // Abort every network request — a self-contained page needs none.
  await context.route("**/*", (route) => {
    const url = route.request().url();
    if (url.startsWith("http://") || url.startsWith("https://")) route.abort();
    else route.continue();
  });

  const records: Array<Record<string, unknown>> = [];
  for (const file of files) {
    const path = join(DIR, file);
    const source = await readFile(path);
    const capture = async () => {
      // Use a fresh page for each pass. Full-page capture changes viewport /
      // scroll internals around sticky elements, so reusing the page makes the
      // second observation depend on the first capture.
      const page = await context.newPage();
      const external: string[] = [];
      page.on("requestfailed", (request) => {
        const url = request.url();
        if (url.startsWith("http")) external.push(url);
      });
      await page.goto(pathToFileURL(path).href, { waitUntil: "networkidle" });
      await page.evaluate(() => document.fonts.ready);
      await page.evaluate(() => window.scrollTo(0, 0));
      await page.waitForTimeout(50);
      const png = await page.screenshot({
        fullPage: true,
        type: "png",
        animations: "disabled",
      });
      await page.close();
      return { png, external };
    };
    const first = await capture();
    const second = await capture();
    const png = first.png;

    const decoded = PNG.sync.read(png);
    const decodedSecond = PNG.sync.read(second.png);
    if (
      decoded.width !== decodedSecond.width ||
      decoded.height !== decodedSecond.height ||
      !decoded.data.equals(decodedSecond.data)
    ) {
      throw new Error(`${idFor(file)}: Chromium capture pixels are not deterministic`);
    }
    const external = [...first.external, ...second.external];
    if (external.length) {
      throw new Error(
        `${idFor(file)} attempted external requests:\n  ${external.join("\n  ")}`,
      );
    }

    // Record the encoded oracle dimensions, not DOM scroll metrics. Full-page
    // capture can round the raster extent by a device pixel.
    const id = idFor(file);
    await writeFile(join(REF, `${id}.png`), png, { flag: "w" });
    records.push({
      id,
      source: file,
      source_sha256: sha256(source),
      reference: `reference/${id}.png`,
      reference_sha256: sha256(png),
      width: decoded.width,
      height: decoded.height,
      external_requests_blocked: external.length,
    });
    console.log(`baked ${id} (${decoded.width}x${decoded.height}, blocked 0 external req)`);
  }

  await context.close();
  await browser.close();

  const manifest = {
    schema_version: 0,
    kind: "chromium-reference",
    note: "Target renders for the Web-first page harness; environment-dependent, regenerable; not a zero-tolerance oracle.",
    browser_version: browserVersion,
    platform: `${process.platform}-${process.arch}`,
    node_version: process.version,
    viewport_width: VIEWPORT_WIDTH,
    device_scale_factor: 1,
    javascript_enabled: false,
    network: "all http(s) aborted (self-containment check)",
    pages: records,
  };
  await writeFile(join(DIR, "reference-bake.json"), `${JSON.stringify(manifest, null, 2)}\n`, { flag: "w" });
  console.log(`\nChromium ${browserVersion}: baked ${records.length} references`);
}

function idFor(file: string): string {
  return file.replace(/\.html$/, "");
}

main().catch((error: unknown) => {
  console.error(error instanceof Error ? error.message : String(error));
  exit(1);
});
