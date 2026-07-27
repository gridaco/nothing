#!/usr/bin/env -S pnpm --filter @grida/reftest exec tsx
/**
 * Bake and verify the independent Chromium oracles for the Web-first SVG
 * sampling corpus: every fixture's static Base projection plus one capture per
 * admitted exact sample time.
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

interface FixtureCase {
  id: string;
  width: number;
  height: number;
  probe: string;
  authored_base_x: number;
  /**
   * Resolved-frame shape, carried in the suite but never read here: capture is
   * always the browser's own pixels, and the browser has no frame. See
   * `crates/websem/tests/svg_animation_x.rs` for its meaning.
   */
  frame?: unknown;
  base: BaseCase;
  animation: AnimationCase;
}

interface CaseSuite {
  schema_version: number;
  fixtures: FixtureCase[];
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

function assertDimensions(png: Buffer, fixture: FixtureCase, label: string): void {
  const image = decode(png);
  if (image.width !== fixture.width || image.height !== fixture.height) {
    throw new Error(
      `${label}: expected ${fixture.width}x${fixture.height}, got ${image.width}x${image.height}`,
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
  fixture: FixtureCase,
  expectedX: number,
  expectedTimeSeconds: number,
  expectedAnimationElements: number,
  label: string,
): void {
  const exact = [
    ["base x", observation.base_x, fixture.authored_base_x],
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

async function openSource(
  page: Page,
  source: Buffer,
  fixture: FixtureCase,
): Promise<void> {
  // The window IS the initial viewport (SVG2 §8.2) the fixture's declared dims
  // stand for, so a root that leaves width/height unauthored (`auto` -> 100% of
  // the initial viewport) captures at exactly those dims.
  await page.setViewportSize({ width: fixture.width, height: fixture.height });
  await page.goto(sourceDataUrl(source), { waitUntil: "load" });
  const svg = page.locator("svg").first();
  if ((await svg.count()) !== 1) {
    throw new Error(`${fixture.id}: expected one root <svg>`);
  }
  const box = await svg.boundingBox();
  if (
    !box ||
    box.x !== 0 ||
    box.y !== 0 ||
    box.width !== fixture.width ||
    box.height !== fixture.height
  ) {
    throw new Error(
      `${fixture.id}: unexpected SVG box ${JSON.stringify(box)}; expected ` +
        `0,0 ${fixture.width}x${fixture.height}`,
    );
  }
}

async function sampleLoadedPage(
  page: Page,
  fixture: FixtureCase,
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
    { probe: fixture.probe, time: timeSeconds },
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
  fixture: FixtureCase,
  timeSeconds: number,
): Promise<Capture> {
  const page = await context.newPage();
  try {
    await openSource(page, source, fixture);
    return await sampleLoadedPage(page, fixture, timeSeconds);
  } finally {
    await page.close();
  }
}

async function admitOracle(
  path: string,
  fresh: Buffer,
  fixture: FixtureCase,
  label: string,
): Promise<Buffer> {
  assertDimensions(fresh, fixture, label);
  await mkdir(dirname(path), { recursive: true });
  if (!existsSync(path)) {
    await writeFile(path, fresh, { flag: "wx" });
    return fresh;
  }
  const committed = await readFile(path);
  assertDimensions(committed, fixture, `${label} committed oracle`);
  assertSameRgba(committed, fresh, `${label} vs committed Chromium oracle`);
  return committed;
}

/**
 * Structural validation only: what a *declaration* must satisfy to be bakeable
 * and to mean what the Rust laws read it as. The declared values themselves are
 * the suite's business — they are what Chromium is then asked to confirm.
 */
function validateSuite(suite: CaseSuite): void {
  if (suite.schema_version !== 1 || suite.fixtures.length === 0) {
    throw new Error("unsupported or empty SVG sampling suite");
  }
  const ids = new Set<string>();
  const oracles = new Set<string>();
  for (const fixture of suite.fixtures) {
    const label = `fixture ${fixture.id}`;
    if (!fixture.id || ids.has(fixture.id)) {
      throw new Error(`${label}: id must be present and unique`);
    }
    ids.add(fixture.id);
    if (
      !Number.isInteger(fixture.width) ||
      !Number.isInteger(fixture.height) ||
      fixture.width <= 0 ||
      fixture.height <= 0
    ) {
      throw new Error(`${label}: dims must be positive integers`);
    }
    if (!fixture.probe.startsWith("#")) {
      throw new Error(`${label}: probe must be an id selector`);
    }
    const samples = fixture.animation.samples;
    if (samples.length === 0) {
      throw new Error(`${label}: at least one sample time must be admitted`);
    }
    for (const [index, sample] of samples.entries()) {
      if (!Number.isInteger(sample.time_ns)) {
        throw new Error(`${label}: sample ${index} time must be integer nanoseconds`);
      }
      if (index > 0 && sample.time_ns <= samples[index - 1].time_ns) {
        throw new Error(`${label}: sample times must strictly increase`);
      }
      if (!Number.isFinite(sample.expected_x)) {
        throw new Error(`${label}: sample ${index} expected_x must be finite`);
      }
    }
    // The Base view is the authored state, not `Sample(0)`. A fixture whose
    // authored value equals its first sample could not tell the two apart, so
    // the corpus would silently stop covering the distinction.
    if (fixture.authored_base_x !== fixture.base.expected_x) {
      throw new Error(`${label}: the Base case must expect the authored value`);
    }
    if (fixture.authored_base_x === samples[0].expected_x) {
      throw new Error(
        `${label}: the authored value must differ from the first sample's, or Base ` +
          `and Sample(${samples[0].time_ns}ns) are indistinguishable`,
      );
    }
    const admitted = new Set(samples.map((sample) => sample.time_ns));
    const order = fixture.animation.retained_seek_order_ns;
    if (
      order.length < admitted.size ||
      order.some((time) => !admitted.has(time)) ||
      [...admitted].some((time) => !order.includes(time))
    ) {
      throw new Error(
        `${label}: retained seek order must cover only and all admitted sample times`,
      );
    }
    for (const oracle of [fixture.base.oracle, ...samples.map((s) => s.oracle)]) {
      if (oracles.has(oracle)) {
        throw new Error(`${label}: oracle path ${oracle} is declared twice`);
      }
      oracles.add(oracle);
    }
  }
}

async function bakeFixture(
  context: BrowserContext,
  fixture: FixtureCase,
): Promise<Record<string, unknown>> {
  const [baseSource, animationSource] = await Promise.all([
    readFile(join(DIR, fixture.base.source)),
    readFile(join(DIR, fixture.animation.source)),
  ]);
  const records: Array<Record<string, unknown>> = [];

  const baseLabel = `${fixture.id} Base`;
  const baseFirst = await freshCapture(context, baseSource, fixture, 0);
  const baseSecond = await freshCapture(context, baseSource, fixture, 0);
  assertObservation(baseFirst.observation, fixture, fixture.base.expected_x, 0, 0, baseLabel);
  assertObservation(baseSecond.observation, fixture, fixture.base.expected_x, 0, 0, baseLabel);
  assertFreshDeterminism(baseFirst.png, baseSecond.png, `${baseLabel} fresh captures`);
  const baseOracle = await admitOracle(
    join(DIR, fixture.base.oracle),
    baseFirst.png,
    fixture,
    baseLabel,
  );
  records.push({
    id: `${fixture.id}/base`,
    policy: "base-static-projection",
    time_ns: null,
    source: fixture.base.source,
    source_sha256: sha256(baseSource),
    oracle: fixture.base.oracle,
    oracle_sha256: sha256(baseOracle),
    rgba_sha256: rgbaSha256(baseOracle),
    expected_x: fixture.base.expected_x,
    observed: baseFirst.observation,
    fresh_capture_count: 2,
  });
  console.log(`verified ${baseLabel} static projection (x=${fixture.base.expected_x})`);

  const sampleOracles = new Map<number, Buffer>();
  for (const sample of fixture.animation.samples) {
    const timeSeconds = seconds(sample.time_ns);
    const label = `${fixture.id} Sample(${sample.time_ns}ns)`;
    const first = await freshCapture(context, animationSource, fixture, timeSeconds);
    const second = await freshCapture(context, animationSource, fixture, timeSeconds);
    assertObservation(first.observation, fixture, sample.expected_x, timeSeconds, 1, label);
    assertObservation(second.observation, fixture, sample.expected_x, timeSeconds, 1, label);
    assertFreshDeterminism(first.png, second.png, `${label} fresh captures`);
    const oracle = await admitOracle(join(DIR, sample.oracle), first.png, fixture, label);
    sampleOracles.set(sample.time_ns, oracle);
    records.push({
      id: `${fixture.id}/sample-${sample.time_ns}ns`,
      policy: "sample",
      time_ns: sample.time_ns,
      source: fixture.animation.source,
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
    fixture.animation.samples.map((sample) => [sample.time_ns, sample]),
  );
  const retainedPage = await context.newPage();
  try {
    await openSource(retainedPage, animationSource, fixture);
    for (const timeNs of fixture.animation.retained_seek_order_ns) {
      const sample = sampleByTime.get(timeNs);
      const oracle = sampleOracles.get(timeNs);
      if (!sample || !oracle) {
        throw new Error(`retained seek references unadmitted time ${timeNs}ns`);
      }
      const label = `${fixture.id} retained Sample(${timeNs}ns)`;
      const capture = await sampleLoadedPage(retainedPage, fixture, seconds(timeNs));
      assertObservation(
        capture.observation,
        fixture,
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
    `verified ${fixture.animation.retained_seek_order_ns.length} shuffled retained ` +
      `seeks for ${fixture.id}`,
  );

  return {
    id: fixture.id,
    width: fixture.width,
    height: fixture.height,
    probe: fixture.probe,
    authored_base_x: fixture.authored_base_x,
    retained_seek_order_ns: fixture.animation.retained_seek_order_ns,
    retained_seek_count: fixture.animation.retained_seek_order_ns.length,
    cases: records,
  };
}

async function main(): Promise<void> {
  const [scriptBytes, suiteBytes] = await Promise.all([
    readFile(SCRIPT_PATH),
    readFile(SUITE_PATH),
  ]);
  const suite = JSON.parse(suiteBytes.toString("utf8")) as CaseSuite;
  validateSuite(suite);

  const browser = await chromium.launch({
    args: ["--no-sandbox", "--disable-setuid-sandbox"],
  });
  const browserVersion = browser.version();
  const context = await browser.newContext({
    javaScriptEnabled: true,
    viewport: { width: suite.fixtures[0].width, height: suite.fixtures[0].height },
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
    const fixtures: Array<Record<string, unknown>> = [];
    let frames = 0;
    for (const fixture of suite.fixtures) {
      const record = await bakeFixture(context, fixture);
      frames += (record.cases as unknown[]).length;
      fixtures.push(record);
    }

    if (networkAttempts.length !== 0) {
      throw new Error(
        `fixture attempted external network requests:\n  ${networkAttempts.join("\n  ")}`,
      );
    }

    const manifest = {
      schema_version: 1,
      kind: "chromium-svg-animation-oracle",
      note: "Independent exact-RGBA oracle; no score or conformance pass claim.",
      browser_version: browserVersion,
      platform: `${process.platform}-${process.arch}`,
      node_version: process.version,
      bake_script_sha256: sha256(scriptBytes),
      suite: "cases.json",
      suite_sha256: sha256(suiteBytes),
      capture: {
        viewport: "per-fixture declared dims (the initial viewport)",
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
      },
      fixtures,
    };
    await writeFile(MANIFEST_PATH, `${JSON.stringify(manifest, null, 2)}\n`, {
      flag: "w",
    });
    console.log(
      `Chromium ${browserVersion}: verified ${frames} oracle frames across ` +
        `${fixtures.length} fixtures`,
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
