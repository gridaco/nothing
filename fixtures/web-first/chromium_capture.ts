/**
 * The one Chromium capture posture for the Web-first primitive suite.
 *
 * Both consumers import this module — `bake_chromium.ts` (the committed
 * oracles) and `probe_harness.ts` (scratch probes) — so a probe measures
 * under exactly the conditions the cells are baked under, instead of a
 * hand-copied posture that can drift. The posture is provenance:
 * `oracle-bake.json` records this file's sha256 alongside the baker's, and
 * `crates/websem/tests/reftest_oracle.rs` refuses a suite whose recorded
 * posture hash is stale.
 */

import {
  chromium,
  type Browser,
  type BrowserContext,
  type Page,
} from "@playwright/test";

export async function launchDeterministicChromium(): Promise<Browser> {
  return chromium.launch({ args: ["--no-sandbox", "--disable-setuid-sandbox"] });
}

export async function deterministicContext(
  browser: Browser,
): Promise<BrowserContext> {
  const context = await browser.newContext({
    javaScriptEnabled: false,
    viewport: { width: 1280, height: 720 },
    deviceScaleFactor: 1,
    colorScheme: "light",
    locale: "en-US",
    timezoneId: "UTC",
  });
  await context.route("**/*", (route) => route.abort());
  return context;
}

export interface SvgCapture {
  media: "image/svg+xml" | "text/html";
  source: Buffer;
  width: number;
  height: number;
  /** Prefixes error messages, e.g. a fixture or probe id. */
  label: string;
}

/**
 * Capture the first `<svg>` element of the source as PNG bytes.
 *
 * standalone-svg (`image/svg+xml`): the window IS the initial viewport
 * (SVG2 §8.2) the declared dims stand for — a missing root width/height is
 * `auto` and resolves to 100% of it. html-inline-svg keeps the context's
 * fixed 1280x720 posture: that entry has no initial-viewport semantics yet.
 */
export async function captureFirstSvg(
  page: Page,
  capture: SvgCapture,
): Promise<Buffer> {
  if (capture.media === "image/svg+xml") {
    await page.setViewportSize({ width: capture.width, height: capture.height });
  }
  const dataUrl = `data:${capture.media};base64,${capture.source.toString("base64")}`;
  await page.goto(dataUrl, { waitUntil: "load" });

  const svg = page.locator("svg").first();
  if ((await svg.count()) !== 1) {
    throw new Error(`${capture.label}: expected a first <svg> element`);
  }
  const box = await svg.boundingBox();
  if (!box || box.width !== capture.width || box.height !== capture.height) {
    throw new Error(
      `${capture.label}: unexpected SVG box ${JSON.stringify(box)}; expected ${capture.width}x${capture.height}`,
    );
  }
  return svg.screenshot({ omitBackground: true, type: "png" });
}
