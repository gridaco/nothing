/**
 * Probe harness — committed so scratch probes inherit the bake's exact
 * capture posture (`chromium_capture.ts`) instead of copy-pasting it.
 *
 * Only mechanics live here. The probe MATRICES stay scratch and are never
 * committed: a probe is a question asked once; what it proves lands as
 * cells, README rows, and records — not as a shadow corpus.
 *
 * A scratch probe reduces to data:
 *
 * ```ts
 * import { runProbeMatrix } from "<repo>/fixtures/web-first/probe_harness";
 * runProbeMatrix({
 *   probes: { "case-a": "<svg …>…</svg>", "case-b": "<svg …>…</svg>" },
 *   pairs: [["case-a", "case-b", "what identity/difference would mean"]],
 *   outDir: "<scratch dir>",
 * }).catch((e) => { console.error(e); process.exit(1); });
 * ```
 *
 * Run via `just probe <script>` (see the justfile in this directory).
 */

import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";

import { PNG } from "pngjs";

import {
  captureFirstSvg,
  deterministicContext,
  launchDeterministicChromium,
  measureOnlySvgText,
  type SvgTextGeometry,
} from "./chromium_capture";

export interface ProbeComparison {
  identical: boolean;
  diffPixels: number;
  maxDelta: number;
}

/** Byte compare first; on mismatch, count differing pixels and the largest
 * per-channel delta over the decoded RGBA. */
export function comparePngs(a: Buffer, b: Buffer): ProbeComparison {
  if (a.equals(b)) return { identical: true, diffPixels: 0, maxDelta: 0 };
  const pa = PNG.sync.read(a);
  const pb = PNG.sync.read(b);
  let diffPixels = 0;
  let maxDelta = 0;
  for (let i = 0; i < pa.data.length; i += 4) {
    let pixelDiffers = false;
    for (let c = 0; c < 4; c += 1) {
      const d = Math.abs(pa.data[i + c] - pb.data[i + c]);
      if (d > 0) pixelDiffers = true;
      if (d > maxDelta) maxDelta = d;
    }
    if (pixelDiffers) diffPixels += 1;
  }
  return { identical: false, diffPixels, maxDelta };
}

export interface ProbeMatrix {
  /** id → source in the declared entry grammar. */
  probes: Record<string, string>;
  /** Same ingress choice as the baker; standalone SVG remains the default. */
  entry?: "standalone-svg" | "html-inline-svg";
  /** [left id, right id, what identity/difference would mean]. */
  pairs: Array<[string, string, string]>;
  /** Where captures are written for eyeballing (scratch space). */
  outDir: string;
  width?: number;
  height?: number;
}

/**
 * Capture every probe twice (byte-determinism asserted, as the bake does),
 * write the PNGs to `outDir`, and print one verdict line per pair:
 * `IDENTICAL` or `DIFFER (Npx, maxΔD)`.
 */
export async function runProbeMatrix(matrix: ProbeMatrix): Promise<void> {
  const width = matrix.width ?? 64;
  const height = matrix.height ?? 64;
  await mkdir(matrix.outDir, { recursive: true });

  const browser = await launchDeterministicChromium();
  console.log(`chromium ${browser.version()}`);
  const context = await deterministicContext(browser);

  const shots = new Map<string, Buffer>();
  for (const [id, svg] of Object.entries(matrix.probes)) {
    const page = await context.newPage();
    const capture = {
      media: matrix.entry === "html-inline-svg" ? "text/html" as const : "image/svg+xml" as const,
      source: Buffer.from(svg),
      width,
      height,
      label: id,
    };
    const first = await captureFirstSvg(page, capture);
    const second = await captureFirstSvg(page, capture);
    await page.close();
    if (!first.equals(second)) throw new Error(`${id}: not byte-deterministic`);
    shots.set(id, first);
    await writeFile(join(matrix.outDir, `${id}.png`), first);
  }

  console.log("\npair verdicts:");
  for (const [left, right, note] of matrix.pairs) {
    const a = shots.get(left);
    const b = shots.get(right);
    if (!a || !b) throw new Error(`missing shot ${left} / ${right}`);
    const r = comparePngs(a, b);
    const verdict = r.identical
      ? "IDENTICAL"
      : `DIFFER (${r.diffPixels}px, maxΔ${r.maxDelta})`;
    console.log(`${left} vs ${right}: ${verdict}  — ${note}`);
  }

  await context.close();
  await browser.close();
}

export interface TextGeometryProbeMatrix {
  /** id -> standalone SVG source with its exact font declared inline. */
  probes: Record<string, string>;
  /** Where exact JSON measurements are written (scratch space). */
  outDir: string;
  width: number;
  height: number;
}

/**
 * Measure SVG text geometry twice per source and require exact structural
 * equality. The shared capture module owns navigation, context posture, and
 * the measurement API calls, so a scratch probe and committed bake cannot
 * silently become two Chromium instruments.
 */
export async function runTextGeometryProbe(
  matrix: TextGeometryProbeMatrix,
): Promise<Record<string, SvgTextGeometry>> {
  await mkdir(matrix.outDir, { recursive: true });
  const browser = await launchDeterministicChromium();
  console.log(`chromium ${browser.version()}`);
  const context = await deterministicContext(browser, {
    javaScriptEnabled: true,
  });
  const results: Record<string, SvgTextGeometry> = {};
  try {
    for (const [id, source] of Object.entries(matrix.probes)) {
      const page = await context.newPage();
      const capture = {
        media: "image/svg+xml" as const,
        source: Buffer.from(source),
        width: matrix.width,
        height: matrix.height,
        label: id,
      };
      const first = await measureOnlySvgText(page, capture);
      const second = await measureOnlySvgText(page, capture);
      await page.close();
      if (JSON.stringify(first) !== JSON.stringify(second)) {
        throw new Error(`${id}: text geometry is not deterministic`);
      }
      results[id] = first;
      await writeFile(
        join(matrix.outDir, `${id}.json`),
        `${JSON.stringify(first, null, 2)}\n`,
      );
      console.log(`${id}: ${JSON.stringify(first)}`);
    }
  } finally {
    await context.close();
    await browser.close();
  }
  return results;
}
