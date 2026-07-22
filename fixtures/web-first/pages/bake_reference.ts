#!/usr/bin/env -S pnpm --filter @grida/reftest exec tsx
/**
 * Bake committed Chromium REFERENCE renders for the Web-first page harness.
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
 * These are version- and platform-pinned target snapshots (text uses system
 * fonts), regenerable — not a zero-tolerance oracle.
 *
 * Run:  pnpm -C packages/grida-reftest exec tsx "$(pwd)/fixtures/web-first/pages/bake_reference.ts"
 */

import { createHash } from "node:crypto";
import { readFile, readdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";
import { exit } from "node:process";

import { chromium } from "@playwright/test";

const DIR = dirname(fileURLToPath(import.meta.url));
const REF = join(DIR, "reference");
const VIEWPORT_WIDTH = 1200;

function sha256(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

async function main(): Promise<void> {
  const files = (await readdir(DIR)).filter((f) => f.endsWith(".html")).sort();
  if (files.length === 0) throw new Error("no page fixtures found");

  const browser = await chromium.launch({
    args: ["--no-sandbox", "--disable-setuid-sandbox"],
  });
  const browserVersion = browser.version();
  const context = await browser.newContext({
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
    const page = await context.newPage();
    const external: string[] = [];
    page.on("requestfailed", (r) => {
      const u = r.url();
      if (u.startsWith("http")) external.push(u);
    });
    await page.goto(pathToFileURL(path).href, { waitUntil: "networkidle" });
    const png = await page.screenshot({ fullPage: true, type: "png" });
    const dims = await page.evaluate(() => ({
      w: document.documentElement.scrollWidth,
      h: document.documentElement.scrollHeight,
    }));
    await page.close();

    const id = file.replace(/\.html$/, "");
    await writeFile(join(REF, `${id}.png`), png, { flag: "w" });
    records.push({
      id,
      source: file,
      source_sha256: sha256(source),
      reference: `reference/${id}.png`,
      reference_sha256: sha256(png),
      width: dims.w,
      height: dims.h,
      external_requests_blocked: external.length,
    });
    console.log(`baked ${id} (${dims.w}x${dims.h}, blocked ${external.length} external req)`);
    if (external.length) {
      console.warn(`  WARNING: ${id} attempted external requests:\n   ${external.join("\n   ")}`);
    }
  }

  await context.close();
  await browser.close();

  const manifest = {
    schema_version: 0,
    kind: "chromium-reference",
    note: "Target renders for the Web-first page harness; version/platform-pinned, regenerable; not a zero-tolerance oracle.",
    browser_version: browserVersion,
    viewport_width: VIEWPORT_WIDTH,
    device_scale_factor: 1,
    network: "all http(s) aborted (self-containment check)",
    pages: records,
  };
  await writeFile(join(DIR, "reference-bake.json"), `${JSON.stringify(manifest, null, 2)}\n`, { flag: "w" });
  console.log(`\nChromium ${browserVersion}: baked ${records.length} references`);
}

main().catch((error: unknown) => {
  console.error(error instanceof Error ? error.message : String(error));
  exit(1);
});
