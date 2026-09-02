#!/usr/bin/env -S pnpm --filter @grida/reftest exec tsx
/** Bake exact SVGTextContentElement geometry for the rung-B text suite. */

import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { exit } from "node:process";
import { fileURLToPath } from "node:url";

import {
  declareFontInSvg,
  deterministicContext,
  launchDeterministicChromium,
  measureOnlySvgText,
} from "../../chromium_capture";

interface Case {
  id: string;
  source: string;
  oracle: string;
  width: number;
  height: number;
  text: string;
  x: string;
  y: string;
  text_anchor: string;
  font_family: string;
  font_size: string;
}

interface SuiteFont {
  family: string;
  path: string;
  sha256: string;
  face_index: number;
  license: string;
  license_sha256: string;
}

interface Suite {
  schema_version: number;
  fonts: SuiteFont[];
  cases: Case[];
}

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const DIR = dirname(SCRIPT_PATH);
const SUITE_PATH = join(DIR, "cases.json");
const OUT_MANIFEST = join(DIR, "oracle-bake.json");
const CAPTURE_MODULE = "../../chromium_capture.ts";

function sha256(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

async function main(): Promise<void> {
  const suiteBytes = await readFile(SUITE_PATH);
  const suite = JSON.parse(suiteBytes.toString("utf8")) as Suite;
  if (suite.schema_version !== 3 || suite.fonts.length === 0 || suite.cases.length === 0) {
    throw new Error("unsupported or empty text geometry suite");
  }
  let previous = "";
  const identities = new Set<string>();
  for (const fixture of suite.cases) {
    if (fixture.id <= previous) {
      throw new Error("geometry cases must have unique ids in sorted order");
    }
    for (const identity of [fixture.id, fixture.source, fixture.oracle]) {
      if (identities.has(identity)) {
        throw new Error(`${fixture.id}: duplicate id, source, or oracle`);
      }
      identities.add(identity);
    }
    previous = fixture.id;
  }

  const fontBytes = new Map<string, Buffer>();
  let previousFamily = "";
  for (const font of suite.fonts) {
    if (font.family <= previousFamily || fontBytes.has(font.family)) {
      throw new Error("geometry fonts must have unique families in sorted order");
    }
    if (font.face_index !== 0) {
      throw new Error("the inline @font-face geometry baker admits face index 0 only");
    }
    const bytes = await readFile(join(DIR, font.path));
    if (sha256(bytes) !== font.sha256) {
      throw new Error(`declared font digest does not match ${font.path}`);
    }
    const licenseBytes = await readFile(join(DIR, font.license));
    if (sha256(licenseBytes) !== font.license_sha256) {
      throw new Error(`declared license digest does not match ${font.license}`);
    }
    fontBytes.set(font.family, bytes);
    previousFamily = font.family;
  }

  const scriptBytes = await readFile(SCRIPT_PATH);
  const captureBytes = await readFile(join(DIR, CAPTURE_MODULE));
  const browser = await launchDeterministicChromium();
  const browserVersion = browser.version();
  const context = await deterministicContext(browser, {
    javaScriptEnabled: true,
  });
  const records: unknown[] = [];
  try {
    for (const fixture of suite.cases) {
      const bytes = fontBytes.get(fixture.font_family);
      if (!bytes) {
        throw new Error(`${fixture.id}: undeclared font family ${fixture.font_family}`);
      }
      const sourcePath = join(DIR, fixture.source);
      const sourceBytes = await readFile(sourcePath);
      const source = sourceBytes.toString("utf8");
      if (source.includes("<script") || source.includes("@font-face")) {
        throw new Error(`${fixture.id}: fixture carries executable code or font bytes`);
      }
      const declared = declareFontInSvg(source, fixture.font_family, bytes);
      const page = await context.newPage();
      const capture = {
        media: "image/svg+xml" as const,
        source: Buffer.from(declared),
        width: fixture.width,
        height: fixture.height,
        label: fixture.id,
      };
      const first = await measureOnlySvgText(page, capture);
      const second = await measureOnlySvgText(page, capture);
      await page.close();
      if (JSON.stringify(first) !== JSON.stringify(second)) {
        throw new Error(`${fixture.id}: Chromium geometry is not deterministic`);
      }
      if (
        !first.font_ready ||
        first.text_content !== fixture.text ||
        first.computed_font_family !== fixture.font_family ||
        first.computed_font_size !== `${fixture.font_size}px` ||
        first.computed_text_anchor !== fixture.text_anchor
      ) {
        throw new Error(
          `${fixture.id}: declared font/text posture did not take effect: ${JSON.stringify(first)}`,
        );
      }

      const oracle = {
        schema_version: 1,
        kind: "chromium-svg-text-geometry",
        measurement: first,
      };
      const fresh = Buffer.from(`${JSON.stringify(oracle, null, 2)}\n`);
      const oraclePath = join(DIR, fixture.oracle);
      if (existsSync(oraclePath)) {
        const existing = await readFile(oraclePath);
        if (!existing.equals(fresh)) {
          throw new Error(`${fixture.id}: Chromium geometry differs from the committed oracle`);
        }
      } else {
        await mkdir(dirname(oraclePath), { recursive: true });
        await writeFile(oraclePath, fresh, { flag: "wx" });
        console.log(`created ${fixture.oracle}`);
      }
      records.push({
        id: fixture.id,
        source: fixture.source,
        source_sha256: sha256(sourceBytes),
        oracle: fixture.oracle,
        oracle_sha256: sha256(await readFile(oraclePath)),
        width: fixture.width,
        height: fixture.height,
        font_family: fixture.font_family,
      });
    }
  } finally {
    await context.close();
    await browser.close();
  }

  const manifest = {
    schema_version: 3,
    kind: "chromium-svg-text-geometry-oracle",
    browser_version: browserVersion,
    suite: "cases.json",
    suite_sha256: sha256(suiteBytes),
    bake_script: "bake_chromium.ts",
    bake_script_sha256: sha256(scriptBytes),
    capture_module: CAPTURE_MODULE,
    capture_module_sha256: sha256(captureBytes),
    fonts: suite.fonts.map((font) => ({
      family: font.family,
      sha256: font.sha256,
      face_index: font.face_index,
      license_sha256: font.license_sha256,
      declaration: "inline @font-face injected by the shared capture module",
    })),
    capture_policy: {
      viewport: "the fixture's declared size as the initial viewport",
      device_scale_factor: 1,
      javascript: "enabled only for harness-owned SVGTextContentElement measurement; fixtures contain no script",
      network: "aborted",
      index_unit: "UTF-16 code unit, per SVGTextContentElement",
      apis: [
        "getNumberOfChars",
        "getComputedTextLength",
        "getSubStringLength",
        "getStartPositionOfChar",
        "getEndPositionOfChar",
        "getExtentOfChar",
        "getRotationOfChar",
      ],
      comparison: "JSON number -> IEEE-754 binary64; artifact binary32 promoted to binary64; exact equality, no tolerance",
      fresh_measurements_per_case: 2,
    },
    records,
  };
  await writeFile(OUT_MANIFEST, `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(`baked ${records.length} text geometry witnesses with ${browserVersion}`);
}

main().catch((error) => {
  console.error(error);
  exit(1);
});
