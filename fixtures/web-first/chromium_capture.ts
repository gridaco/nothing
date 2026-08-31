/**
 * The one Chromium capture posture for the Web-first suites.
 *
 * Every baker and `probe_harness.ts` imports this module, so a probe measures
 * under exactly the conditions the committed evidence is captured under,
 * instead of a hand-copied posture that can drift. The posture is provenance:
 * `oracle-bake.json` records this file's sha256 alongside the baker's, and
 * `crates/websem/tests/reftest_oracle.rs` refuses a suite whose recorded
 * posture hash is stale.
 */

import {
  chromium,
  type Browser,
  type BrowserContext,
  type Locator,
  type Page,
} from "@playwright/test";

export async function launchDeterministicChromium(): Promise<Browser> {
  return chromium.launch({ args: ["--no-sandbox", "--disable-setuid-sandbox"] });
}

export async function deterministicContext(
  browser: Browser,
  options: { javaScriptEnabled?: boolean } = {},
): Promise<BrowserContext> {
  const context = await browser.newContext({
    javaScriptEnabled: options.javaScriptEnabled ?? false,
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

/** Declare one exact font identity without putting font bytes in the fixture. */
export function declareFontInSvg(
  source: string,
  family: string,
  fontBytes: Uint8Array,
): string {
  if (!/^[A-Za-z0-9 _-]+$/.test(family)) {
    throw new Error(`font family ${JSON.stringify(family)} is not safely declarable`);
  }
  const fontDataUrl = `data:font/ttf;base64,${Buffer.from(fontBytes).toString("base64")}`;
  const rule = `<style>@font-face{font-family:${JSON.stringify(family)};src:url(${fontDataUrl})}</style>`;
  const rootEnd = source.indexOf(">");
  if (rootEnd < 0 || !source.slice(0, rootEnd).includes("<svg")) {
    throw new Error("fixture must open with an <svg> root element");
  }
  return source.slice(0, rootEnd + 1) + rule + source.slice(rootEnd + 1);
}

async function loadFirstSvg(page: Page, capture: SvgCapture): Promise<Locator> {
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
  return svg;
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
  const svg = await loadFirstSvg(page, capture);
  return svg.screenshot({ omitBackground: true, type: "png" });
}

export interface SvgTextCharacterGeometry {
  utf16_code_unit: number;
  substring_length: number;
  start: { x: number; y: number };
  end: { x: number; y: number };
  extent: { x: number; y: number; width: number; height: number };
  rotation: number;
}

export interface SvgTextGeometry {
  text_content: string;
  computed_font_family: string;
  computed_font_size: string;
  computed_text_anchor: string;
  font_ready: boolean;
  number_of_chars: number;
  computed_text_length: number;
  substring_length: number;
  characters: SvgTextCharacterGeometry[];
}

/**
 * Measure the only `<text>` child of the first SVG through the standard
 * SVGTextContentElement geometry APIs. The context must have JavaScript
 * enabled: author scripts remain forbidden by the fixture gate, while this
 * harness-owned evaluation is the measurement instrument.
 */
export async function measureOnlySvgText(
  page: Page,
  capture: SvgCapture,
): Promise<SvgTextGeometry> {
  const svg = await loadFirstSvg(page, capture);
  const text = svg.locator("text");
  if ((await text.count()) !== 1) {
    throw new Error(`${capture.label}: expected exactly one <text> element`);
  }
  await page.evaluate(async () => document.fonts.ready);
  return text.evaluate((node) => {
    const element = node as SVGTextContentElement;
    const style = getComputedStyle(element);
    const count = element.getNumberOfChars();
    const content = element.textContent ?? "";
    const characters = [];
    for (let index = 0; index < count; index += 1) {
      const start = element.getStartPositionOfChar(index);
      const end = element.getEndPositionOfChar(index);
      const extent = element.getExtentOfChar(index);
      characters.push({
        utf16_code_unit: content.charCodeAt(index),
        substring_length: element.getSubStringLength(index, 1),
        start: { x: start.x, y: start.y },
        end: { x: end.x, y: end.y },
        extent: {
          x: extent.x,
          y: extent.y,
          width: extent.width,
          height: extent.height,
        },
        rotation: element.getRotationOfChar(index),
      });
    }
    return {
      text_content: content,
      computed_font_family: style.fontFamily,
      computed_font_size: style.fontSize,
      computed_text_anchor: style.textAnchor,
      font_ready: document.fonts.check(`${style.fontSize} ${style.fontFamily}`, content),
      number_of_chars: count,
      computed_text_length: element.getComputedTextLength(),
      substring_length: element.getSubStringLength(0, count),
      characters,
    };
  });
}
