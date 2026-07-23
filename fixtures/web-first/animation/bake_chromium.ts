#!/usr/bin/env -S pnpm --filter @grida/reftest exec tsx
/**
 * Bake and verify the independent Chromium oracle for the Web-first SVG
 * rect-x animation slice.
 *
 * Run:
 *   pnpm -C packages/grida-reftest exec tsx \
 *     "$(pwd)/fixtures/web-first/animation/bake_chromium.ts"
 *
 * This script never invokes the consolidation scoreboard and never emits a
 * score. Existing oracle pixels are verification inputs: a differing bake
 * fails rather than overwriting them.
 */

import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { exit } from "node:process";
import { fileURLToPath } from "node:url";

import { chromium, type BrowserContext, type Page } from "@playwright/test";
import { PNG } from "pngjs";

interface BaseCase {
  source: string;
  oracle: string;
  expected_x: number;
}

interface SampleCase {
  time_ns: number;
  oracle: string;
  expected_x: number;
}

interface AnimationCase {
  source: string;
  samples: SampleCase[];
  retained_seek_order_ns: number[];
}

interface CaseSuite {
  schema_version: number;
  fixture_id: string;
  width: number;
  height: number;
  probe: string;
  authored_base_x: number;
  base: BaseCase;
  animation: AnimationCase;
}

interface Observation {
  base_x: number;
  anim_x: number;
  bbox_x: number;
  current_time_seconds: number;
  animation_element_count: number;
  animations_paused: boolean;
}

interface Capture {
  png: Buffer;
  observation: Observation;
}

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const DIR = dirname(SCRIPT_PATH);
const SUITE_PATH = join(DIR, "cases.json");
const MANIFEST_PATH = join(DIR, "oracle-bake.json");

function sha256(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

function decode(png: Buffer): PNG {
  return PNG.sync.read(png);
}

function rgbaSha256(png: Buffer): string {
  return sha256(decode(png).data);
}

function assertDimensions(png: Buffer, suite: CaseSuite, label: string): void {
  const image = decode(png);
  if (image.width !== suite.width || image.height !== suite.height) {
    throw new Error(
      `${label}: expected ${suite.width}x${suite.height}, got ${image.width}x${image.height}`,
    );
  }
}

function assertSameRgba(a: Buffer, b: Buffer, label: string): void {
  const left = decode(a);
  const right = decode(b);
  if (
    left.width !== right.width ||
    left.height !== right.height ||
    !left.data.equals(right.data)
  ) {
    throw new Error(`${label}: decoded RGBA pixels differ`);
  }
}

function assertFreshDeterminism(a: Buffer, b: Buffer, label: string): void {
  assertSameRgba(a, b, label);
  if (!a.equals(b)) {
    throw new Error(`${label}: fresh Chromium PNG encodings are not byte-identical`);
  }
}

function seconds(timeNs: number): number {
  return timeNs / 1_000_000_000;
}

function assertObservation(
  observation: Observation,
  suite: CaseSuite,
  expectedX: number,
  expectedTimeSeconds: number,
  expectedAnimationElements: number,
  label: string,
): void {
  const exact = [
    ["base x", observation.base_x, suite.authored_base_x],
    ["animated x", observation.anim_x, expectedX],
    ["bounding-box x", observation.bbox_x, expectedX],
    ["current time", observation.current_time_seconds, expectedTimeSeconds],
    [
      "animation element count",
      observation.animation_element_count,
      expectedAnimationElements,
    ],
  ] as const;
  for (const [field, actual, expected] of exact) {
    if (actual !== expected) {
      throw new Error(`${label}: expected ${field}=${expected}, got ${actual}`);
    }
  }
  if (!observation.animations_paused) {
    throw new Error(`${label}: SVG animation timeline was not paused`);
  }
}

function sourceDataUrl(source: Buffer): string {
  return `data:image/svg+xml;base64,${source.toString("base64")}`;
}

async function openSource(page: Page, source: Buffer, suite: CaseSuite): Promise<void> {
  await page.goto(sourceDataUrl(source), { waitUntil: "load" });
  const svg = page.locator("svg").first();
  if ((await svg.count()) !== 1) {
    throw new Error(`${suite.fixture_id}: expected one root <svg>`);
  }
  const box = await svg.boundingBox();
  if (
    !box ||
    box.x !== 0 ||
    box.y !== 0 ||
    box.width !== suite.width ||
    box.height !== suite.height
  ) {
    throw new Error(
      `${suite.fixture_id}: unexpected SVG box ${JSON.stringify(box)}; expected ` +
        `0,0 ${suite.width}x${suite.height}`,
    );
  }
}

async function sampleLoadedPage(
  page: Page,
  suite: CaseSuite,
  timeSeconds: number,
): Promise<Capture> {
  const observation = await page.evaluate(
    ({ probe, time }) => {
      const root = document.documentElement;
      const subject = document.querySelector(probe);
      if (!(root instanceof SVGSVGElement)) {
        throw new Error("document root is not <svg>");
      }
      if (!(subject instanceof SVGRectElement)) {
        throw new Error(`${probe} is not <rect>`);
      }
      root.pauseAnimations();
      root.setCurrentTime(time);
      const bbox = subject.getBBox();
      return {
        base_x: subject.x.baseVal.value,
        anim_x: subject.x.animVal.value,
        bbox_x: bbox.x,
        current_time_seconds: root.getCurrentTime(),
        animation_element_count: root.getElementsByTagName("animate").length,
        animations_paused: root.animationsPaused(),
      };
    },
    { probe: suite.probe, time: timeSeconds },
  );
  const png = await page.locator("svg").first().screenshot({
    animations: "allow",
    caret: "hide",
    omitBackground: true,
    type: "png",
  });
  return { png, observation };
}

async function freshCapture(
  context: BrowserContext,
  source: Buffer,
  suite: CaseSuite,
  timeSeconds: number,
): Promise<Capture> {
  const page = await context.newPage();
  try {
    await openSource(page, source, suite);
    return await sampleLoadedPage(page, suite, timeSeconds);
  } finally {
    await page.close();
  }
}

async function admitOracle(
  path: string,
  fresh: Buffer,
  suite: CaseSuite,
  label: string,
): Promise<Buffer> {
  assertDimensions(fresh, suite, label);
  await mkdir(dirname(path), { recursive: true });
  if (!existsSync(path)) {
    await writeFile(path, fresh, { flag: "wx" });
    return fresh;
  }
  const committed = await readFile(path);
  assertDimensions(committed, suite, `${label} committed oracle`);
  assertSameRgba(committed, fresh, `${label} vs committed Chromium oracle`);
  return committed;
}

function validateSuite(suite: CaseSuite): void {
  if (
    suite.schema_version !== 0 ||
    suite.width !== 64 ||
    suite.height !== 32 ||
    suite.authored_base_x !== 4 ||
    suite.base.expected_x !== 4 ||
    suite.animation.samples.length !== 4
  ) {
    throw new Error("unsupported SVG rect-x animation suite");
  }
  const expectedSamples = [
    [0, 20],
    [1_000_000_000, 32],
    [2_000_000_000, 44],
    [3_000_000_000, 44],
  ] as const;
  for (let index = 0; index < expectedSamples.length; index += 1) {
    const [expectedTime, expectedX] = expectedSamples[index];
    const sample = suite.animation.samples[index];
    if (sample.time_ns !== expectedTime || sample.expected_x !== expectedX) {
      throw new Error(`unexpected sample declaration at index ${index}`);
    }
  }
  const admitted = new Set(suite.animation.samples.map((sample) => sample.time_ns));
  if (
    suite.animation.retained_seek_order_ns.length < admitted.size ||
    suite.animation.retained_seek_order_ns.some((time) => !admitted.has(time)) ||
    [...admitted].some(
      (time) => !suite.animation.retained_seek_order_ns.includes(time),
    )
  ) {
    throw new Error("retained seek order must cover only and all admitted sample times");
  }
}

async function main(): Promise<void> {
  const [scriptBytes, suiteBytes] = await Promise.all([
    readFile(SCRIPT_PATH),
    readFile(SUITE_PATH),
  ]);
  const suite = JSON.parse(suiteBytes.toString("utf8")) as CaseSuite;
  validateSuite(suite);

  const [baseSource, animationSource] = await Promise.all([
    readFile(join(DIR, suite.base.source)),
    readFile(join(DIR, suite.animation.source)),
  ]);
  const browser = await chromium.launch({
    args: ["--no-sandbox", "--disable-setuid-sandbox"],
  });
  const browserVersion = browser.version();
  const context = await browser.newContext({
    javaScriptEnabled: true,
    viewport: { width: suite.width, height: suite.height },
    deviceScaleFactor: 1,
    colorScheme: "light",
    locale: "en-US",
    timezoneId: "UTC",
  });
  const networkAttempts: string[] = [];
  await context.route("**/*", (route) => {
    const url = route.request().url();
    if (url.startsWith("http://") || url.startsWith("https://")) {
      networkAttempts.push(url);
      return route.abort();
    }
    return route.continue();
  });

  try {
    const records: Array<Record<string, unknown>> = [];

    const baseFirst = await freshCapture(context, baseSource, suite, 0);
    const baseSecond = await freshCapture(context, baseSource, suite, 0);
    assertObservation(baseFirst.observation, suite, suite.base.expected_x, 0, 0, "Base");
    assertObservation(baseSecond.observation, suite, suite.base.expected_x, 0, 0, "Base");
    assertFreshDeterminism(baseFirst.png, baseSecond.png, "Base fresh captures");
    const baseOracle = await admitOracle(
      join(DIR, suite.base.oracle),
      baseFirst.png,
      suite,
      "Base",
    );
    records.push({
      id: "base",
      policy: "base-static-projection",
      time_ns: null,
      source: suite.base.source,
      source_sha256: sha256(baseSource),
      oracle: suite.base.oracle,
      oracle_sha256: sha256(baseOracle),
      rgba_sha256: rgbaSha256(baseOracle),
      expected_x: suite.base.expected_x,
      observed: baseFirst.observation,
      fresh_capture_count: 2,
    });
    console.log("verified Base static projection (x=4)");

    const sampleOracles = new Map<number, Buffer>();
    for (const sample of suite.animation.samples) {
      const timeSeconds = seconds(sample.time_ns);
      const first = await freshCapture(context, animationSource, suite, timeSeconds);
      const second = await freshCapture(context, animationSource, suite, timeSeconds);
      const label = `Sample(${sample.time_ns}ns)`;
      assertObservation(first.observation, suite, sample.expected_x, timeSeconds, 1, label);
      assertObservation(second.observation, suite, sample.expected_x, timeSeconds, 1, label);
      assertFreshDeterminism(first.png, second.png, `${label} fresh captures`);
      const oracle = await admitOracle(
        join(DIR, sample.oracle),
        first.png,
        suite,
        label,
      );
      sampleOracles.set(sample.time_ns, oracle);
      records.push({
        id: `sample-${sample.time_ns}ns`,
        policy: "sample",
        time_ns: sample.time_ns,
        source: suite.animation.source,
        source_sha256: sha256(animationSource),
        oracle: sample.oracle,
        oracle_sha256: sha256(oracle),
        rgba_sha256: rgbaSha256(oracle),
        expected_x: sample.expected_x,
        observed: first.observation,
        fresh_capture_count: 2,
      });
      console.log(`verified ${label} (x=${sample.expected_x})`);
    }

    const sampleByTime = new Map(
      suite.animation.samples.map((sample) => [sample.time_ns, sample]),
    );
    const retainedPage = await context.newPage();
    try {
      await openSource(retainedPage, animationSource, suite);
      for (const timeNs of suite.animation.retained_seek_order_ns) {
        const sample = sampleByTime.get(timeNs);
        const oracle = sampleOracles.get(timeNs);
        if (!sample || !oracle) {
          throw new Error(`retained seek references unadmitted time ${timeNs}ns`);
        }
        const label = `retained Sample(${timeNs}ns)`;
        const capture = await sampleLoadedPage(
          retainedPage,
          suite,
          seconds(timeNs),
        );
        assertObservation(
          capture.observation,
          suite,
          sample.expected_x,
          seconds(timeNs),
          1,
          label,
        );
        assertSameRgba(capture.png, oracle, `${label} vs fresh oracle`);
      }
    } finally {
      await retainedPage.close();
    }
    console.log(
      `verified ${suite.animation.retained_seek_order_ns.length} shuffled retained seeks`,
    );

    if (networkAttempts.length !== 0) {
      throw new Error(
        `fixture attempted external network requests:\n  ${networkAttempts.join("\n  ")}`,
      );
    }

    const manifest = {
      schema_version: 0,
      kind: "chromium-svg-animation-oracle",
      note: "Independent exact-RGBA oracle; no score or conformance pass claim.",
      browser_version: browserVersion,
      platform: `${process.platform}-${process.arch}`,
      node_version: process.version,
      bake_script_sha256: sha256(scriptBytes),
      suite: "cases.json",
      suite_sha256: sha256(suiteBytes),
      capture: {
        viewport: { width: suite.width, height: suite.height },
        device_scale_factor: 1,
        color_scheme: "light",
        locale: "en-US",
        timezone: "UTC",
        omit_background: true,
        source_transport: "data-url-from-exact-file-bytes",
        javascript_enabled: true,
        network: "all http(s) requests aborted; zero attempted",
        target: "root-svg-element",
        timeline_control: "pauseAnimations() then setCurrentTime(ns / 1e9)",
        playwright_animation_capture_policy: "allow",
        comparison: "exact decoded RGBA; fresh PNG encodings also byte-identical",
        fresh_captures_per_case: 2,
        retained_seek_order_ns: suite.animation.retained_seek_order_ns,
        retained_seek_count: suite.animation.retained_seek_order_ns.length,
      },
      cases: records,
    };
    await writeFile(MANIFEST_PATH, `${JSON.stringify(manifest, null, 2)}\n`, {
      flag: "w",
    });
    console.log(
      `Chromium ${browserVersion}: verified ${records.length} oracle frames`,
    );
  } finally {
    await context.close();
    await browser.close();
  }
}

main().catch((error: unknown) => {
  console.error(error instanceof Error ? error.message : String(error));
  exit(1);
});
