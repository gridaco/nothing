#!/usr/bin/env -S pnpm --filter @grida/reftest exec tsx
/**
 * Bake the scoreboard's committed Chromium oracle from the exact hashed SVG
 * bytes, without source or DOM rewriting.
 *
 * The corpus declares both output targets. Publication is create-new-only; a
 * re-bake requires a fresh corpus candidate that names fresh review paths.
 */

import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import {
  lstat,
  link,
  mkdir,
  readFile,
  realpath,
  rename,
  rm,
  unlink,
  writeFile,
} from "node:fs/promises";
import { dirname, isAbsolute, join, relative, resolve, sep } from "node:path";
import { argv, exit, pid } from "node:process";
import { fileURLToPath } from "node:url";

import { chromium, type Browser } from "@playwright/test";

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const REPO_ROOT = resolve(dirname(SCRIPT_PATH), "../../..");
const DEFAULT_CORPUS = join(
  REPO_ROOT,
  "fixtures/scoreboard/svg-rect-path-v0/corpus.json"
);
const DEFAULT_OUT = join(
  REPO_ROOT,
  "fixtures/scoreboard/svg-rect-path-v0/chromium"
);
const DEFAULT_BAKE_MANIFEST = join(
  REPO_ROOT,
  "fixtures/scoreboard/svg-rect-path-v0/oracle-bake.json"
);
const FIXTURE_ID = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

interface CorpusFixture {
  id: string;
  source: string;
  source_sha256: string;
  oracle: string;
}

interface CorpusManifest {
  schema_version: number;
  corpus_id: string;
  suite_id: string;
  request: string;
  viewport: { width: number; height: number };
  background: string;
  oracle_bake: string;
  fixtures: CorpusFixture[];
}

function arg(name: string, fallback: string): string {
  const index = argv.indexOf(name);
  if (index < 0) return fallback;
  const value = argv[index + 1];
  if (!value || value.startsWith("--")) {
    throw new Error(`${name} requires a path`);
  }
  return value;
}

function validateArgs(): void {
  const allowed = new Set(["--corpus", "--out", "--bake-manifest"]);
  for (let index = 2; index < argv.length; index += 2) {
    const name = argv[index];
    const value = argv[index + 1];
    if (!allowed.has(name)) {
      throw new Error(`unknown argument: ${name}`);
    }
    if (!value || value.startsWith("--")) {
      throw new Error(`${name} requires a path`);
    }
  }
}

function sha256(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

function repoPath(path: string): string {
  if (isAbsolute(path)) {
    throw new Error(`corpus path must be repository-relative: ${path}`);
  }
  const absolute = resolve(REPO_ROOT, path);
  if (absolute !== REPO_ROOT && !absolute.startsWith(REPO_ROOT + sep)) {
    throw new Error(`path escapes repository root: ${path}`);
  }
  return absolute;
}

function confinedCliPath(path: string, label: string): string {
  const absolute = resolve(REPO_ROOT, path);
  if (absolute === REPO_ROOT || !absolute.startsWith(REPO_ROOT + sep)) {
    throw new Error(`${label} must be inside the repository: ${path}`);
  }
  return absolute;
}

function repoRelative(path: string): string {
  const absolute = resolve(path);
  if (absolute === REPO_ROOT || !absolute.startsWith(REPO_ROOT + sep)) {
    throw new Error(`output must be inside the repository: ${path}`);
  }
  return relative(REPO_ROOT, absolute).split(sep).join("/");
}

function stagingPath(stagingDir: string, id: string): string {
  const path = resolve(stagingDir, `${id}.png`);
  if (!path.startsWith(resolve(stagingDir) + sep)) {
    throw new Error(`${id}: oracle output escapes the staging directory`);
  }
  return path;
}

async function ensureDirectoryWithoutSymlinks(
  directory: string,
  createMissing: boolean
): Promise<void> {
  const relativeDirectory = relative(REPO_ROOT, resolve(directory));
  if (
    relativeDirectory === "" ||
    isAbsolute(relativeDirectory) ||
    relativeDirectory === ".." ||
    relativeDirectory.startsWith(`..${sep}`)
  ) {
    throw new Error(`directory is outside the repository: ${directory}`);
  }

  const canonicalRoot = await realpath(REPO_ROOT);
  let current = REPO_ROOT;
  for (const component of relativeDirectory.split(sep)) {
    current = join(current, component);
    let metadata;
    try {
      metadata = await lstat(current);
    } catch (error) {
      if (
        !createMissing ||
        !(error instanceof Error) ||
        !("code" in error) ||
        error.code !== "ENOENT"
      ) {
        throw error;
      }
      await mkdir(current);
      metadata = await lstat(current);
    }
    if (metadata.isSymbolicLink()) {
      throw new Error(`directory contains a symlink: ${current}`);
    }
    if (!metadata.isDirectory()) {
      throw new Error(`path component is not a directory: ${current}`);
    }
    const canonical = await realpath(current);
    if (
      canonical !== canonicalRoot &&
      !canonical.startsWith(canonicalRoot + sep)
    ) {
      throw new Error(`directory escapes the repository: ${current}`);
    }
  }
}

async function validateExistingRepoFile(
  path: string,
  label: string
): Promise<void> {
  await ensureDirectoryWithoutSymlinks(dirname(path), false);
  const metadata = await lstat(path);
  if (metadata.isSymbolicLink() || !metadata.isFile()) {
    throw new Error(`${label} must be a non-symlink file: ${path}`);
  }
  const canonicalRoot = await realpath(REPO_ROOT);
  const canonical = await realpath(path);
  if (
    canonical === canonicalRoot ||
    !canonical.startsWith(canonicalRoot + sep)
  ) {
    throw new Error(`${label} escapes the repository: ${path}`);
  }
}

function validateRawSource(id: string, source: Buffer): string {
  const text = source.toString("utf8");
  if (!Buffer.from(text, "utf8").equals(source)) {
    throw new Error(`${id}: source is not valid UTF-8`);
  }
  if (/<!DOCTYPE|<!ENTITY|<\?xml-stylesheet/i.test(text)) {
    throw new Error(`${id}: declarations and external entities are forbidden`);
  }
  return text;
}

async function main(): Promise<void> {
  validateArgs();
  const corpusPath = confinedCliPath(arg("--corpus", DEFAULT_CORPUS), "corpus");
  const outputDir = confinedCliPath(arg("--out", DEFAULT_OUT), "output");
  const bakeManifestPath = confinedCliPath(
    arg("--bake-manifest", DEFAULT_BAKE_MANIFEST),
    "bake manifest"
  );
  await validateExistingRepoFile(corpusPath, "corpus");
  await validateExistingRepoFile(SCRIPT_PATH, "bake script");
  await ensureDirectoryWithoutSymlinks(dirname(outputDir), true);
  await ensureDirectoryWithoutSymlinks(dirname(bakeManifestPath), true);
  if (existsSync(outputDir)) {
    throw new Error(
      `output already exists: ${outputDir}; bake into a fresh review directory`
    );
  }
  if (existsSync(bakeManifestPath)) {
    throw new Error(
      `bake manifest already exists: ${bakeManifestPath}; use a fresh candidate path`
    );
  }

  const corpusBytes = await readFile(corpusPath);
  const scriptBytes = await readFile(SCRIPT_PATH);
  const corpus = JSON.parse(corpusBytes.toString("utf8")) as CorpusManifest;
  if (corpus.schema_version !== 0 || corpus.request !== "static_base") {
    throw new Error("unsupported scoreboard corpus contract");
  }
  if (corpus.background.toLowerCase() !== "#ffffff") {
    throw new Error("scoreboard v0 requires the declared white background");
  }
  if (corpus.fixtures.length === 0) {
    throw new Error("scoreboard corpus is empty");
  }

  const width = corpus.viewport.width;
  const height = corpus.viewport.height;
  if (
    !Number.isInteger(width) ||
    !Number.isInteger(height) ||
    width < 1 ||
    height < 1
  ) {
    throw new Error(
      "scoreboard viewport must contain positive integer extents"
    );
  }

  const declaredBakeManifest = repoPath(corpus.oracle_bake);
  if (declaredBakeManifest !== bakeManifestPath) {
    throw new Error(
      `corpus declares bake manifest ${declaredBakeManifest}, not ${bakeManifestPath}`
    );
  }
  const outputRelative = repoRelative(outputDir);
  const ids = corpus.fixtures.map((fixture) => fixture.id);
  const sortedIds = [...ids].sort();
  if (
    new Set(ids).size !== ids.length ||
    ids.some((id) => !FIXTURE_ID.test(id))
  ) {
    throw new Error("fixture IDs must be unique strict kebab-case names");
  }
  if (ids.some((id, index) => id !== sortedIds[index])) {
    throw new Error("fixture IDs must be lexicographically sorted");
  }
  const sourcePaths = new Set<string>();
  const oraclePaths = new Set<string>();
  for (const fixture of corpus.fixtures) {
    const sourcePath = repoPath(fixture.source);
    const oraclePath = repoPath(fixture.oracle);
    const expectedOracle = resolve(outputDir, `${fixture.id}.png`);
    if (oraclePath !== expectedOracle) {
      throw new Error(
        `${fixture.id}: corpus oracle must be ${outputRelative}/${fixture.id}.png`
      );
    }
    if (sourcePaths.has(sourcePath) || oraclePaths.has(oraclePath)) {
      throw new Error(`${fixture.id}: source and oracle paths must be unique`);
    }
    sourcePaths.add(sourcePath);
    oraclePaths.add(oraclePath);
  }

  const stagingDir = `${outputDir}.staging-${pid}`;
  const stagingManifest = `${bakeManifestPath}.staging-${pid}`;
  if (existsSync(stagingDir) || existsSync(stagingManifest)) {
    throw new Error("scoreboard bake staging target already exists");
  }
  await mkdir(stagingDir);

  let browser: Browser | undefined;
  let outputClaimed = false;
  let manifestPublished = false;
  const records: Array<{
    id: string;
    source_sha256: string;
    oracle_sha256: string;
  }> = [];

  try {
    browser = await chromium.launch({
      args: ["--no-sandbox", "--disable-setuid-sandbox"],
    });
    const browserVersion = browser.version();
    const context = await browser.newContext({
      javaScriptEnabled: false,
      viewport: { width, height },
      deviceScaleFactor: 1,
      colorScheme: "light",
      locale: "en-US",
      timezoneId: "UTC",
    });
    await context.route("**/*", (route) => route.abort());

    for (const fixture of corpus.fixtures) {
      const sourcePath = repoPath(fixture.source);
      await validateExistingRepoFile(sourcePath, `${fixture.id} source`);
      const source = await readFile(sourcePath);
      validateRawSource(fixture.id, source);
      const sourceDigest = sha256(source);
      if (sourceDigest !== fixture.source_sha256) {
        throw new Error(
          `${fixture.id}: source digest ${sourceDigest} does not match corpus ${fixture.source_sha256}`
        );
      }

      const page = await context.newPage();
      try {
        const exactSourceUrl = `data:image/svg+xml;base64,${source.toString("base64")}`;
        await page.goto(exactSourceUrl, { waitUntil: "load" });
        const observed = await page.evaluate(
          ({ width, height }) => {
            const root = document.documentElement as unknown as SVGSVGElement;
            const allowedAttributes: Record<string, readonly string[]> = {
              svg: ["xmlns", "width", "height"],
              rect: [
                "x",
                "y",
                "width",
                "height",
                "rx",
                "ry",
                "fill",
                "opacity",
                "transform",
              ],
              path: ["d", "fill", "fill-rule", "opacity", "transform"],
            };
            const elements = [root, ...root.querySelectorAll("*")];
            const violations: string[] = [];
            for (const element of elements) {
              const tag = element.localName;
              const attributes = allowedAttributes[tag];
              if (!attributes) {
                violations.push(
                  `element <${tag}> is outside the closed shape set`
                );
                continue;
              }
              if (element !== root && element.parentElement !== root) {
                violations.push(`<${tag}> must be a direct child of <svg>`);
              }
              for (const attribute of element.getAttributeNames()) {
                if (!attributes.includes(attribute)) {
                  violations.push(
                    `<${tag}> attribute ${attribute} is forbidden`
                  );
                }
              }
              const fill = element.getAttribute("fill");
              if (fill && !/^#[0-9a-f]{6}$/i.test(fill)) {
                violations.push(
                  `<${tag}> fill must be one opaque hexadecimal color`
                );
              }
            }
            const background = root.firstElementChild;
            const backgroundValid =
              background?.localName === "rect" &&
              background.getAttribute("width") === String(width) &&
              background.getAttribute("height") === String(height) &&
              background.getAttribute("fill")?.toLowerCase() === "#ffffff" &&
              background.getAttribute("x") === null &&
              background.getAttribute("y") === null &&
              background.getAttribute("opacity") === null &&
              background.getAttribute("transform") === null;
            return {
              namespace: root.namespaceURI,
              width: root.width.baseVal.value,
              height: root.height.baseVal.value,
              backgroundValid,
              violations,
            };
          },
          { width, height }
        );
        if (
          observed.namespace !== "http://www.w3.org/2000/svg" ||
          observed.width !== width ||
          observed.height !== height ||
          !observed.backgroundValid ||
          observed.violations.length !== 0
        ) {
          throw new Error(
            `${fixture.id}: source violates the closed bake boundary: ${observed.violations.join(
              "; "
            )}`
          );
        }

        const capture = async (): Promise<Buffer> =>
          page.screenshot({
            clip: { x: 0, y: 0, width, height },
            omitBackground: true,
            type: "png",
          });
        const first = await capture();
        const second = await capture();
        if (!first.equals(second)) {
          throw new Error(
            `${fixture.id}: Chromium bake is not byte-deterministic`
          );
        }
        const sourceAfterCapture = await readFile(sourcePath);
        if (!source.equals(sourceAfterCapture)) {
          throw new Error(
            `${fixture.id}: source changed during Chromium capture`
          );
        }

        await writeFile(stagingPath(stagingDir, fixture.id), first, {
          flag: "wx",
        });
        records.push({
          id: fixture.id,
          source_sha256: sourceDigest,
          oracle_sha256: sha256(first),
        });
      } finally {
        await page.close();
      }
    }
    await context.close();
    await browser.close();
    browser = undefined;

    if (!(await readFile(corpusPath)).equals(corpusBytes)) {
      throw new Error("corpus manifest changed during Chromium capture");
    }
    if (!(await readFile(SCRIPT_PATH)).equals(scriptBytes)) {
      throw new Error("bake script changed during Chromium capture");
    }

    const bakeManifest = {
      schema_version: 0,
      corpus_id: corpus.corpus_id,
      corpus_manifest_sha256: sha256(corpusBytes),
      kind: "chromium",
      browser_version: browserVersion,
      bake_script_sha256: sha256(scriptBytes),
      capture: {
        width,
        height,
        device_scale_factor: 1,
        omit_background: true,
        source_transport: "data-url-from-hashed-bytes",
        source_mutation: false,
        style_injection: false,
        animation_control: false,
        javascript_enabled: false,
        network_enabled: false,
      },
      fixtures: records,
    };
    await writeFile(
      stagingManifest,
      `${JSON.stringify(bakeManifest, null, 2)}\n`,
      { flag: "wx" }
    );

    await ensureDirectoryWithoutSymlinks(dirname(outputDir), false);
    await ensureDirectoryWithoutSymlinks(dirname(bakeManifestPath), false);
    await mkdir(outputDir);
    outputClaimed = true;
    for (const record of records) {
      await rename(
        stagingPath(stagingDir, record.id),
        join(outputDir, `${record.id}.png`)
      );
    }
    await link(stagingManifest, bakeManifestPath);
    manifestPublished = true;
    await unlink(stagingManifest);
    await rm(stagingDir, { recursive: true });

    console.log(`Chromium ${browserVersion}: baked ${records.length} fixtures`);
    console.log(`oracle: ${outputDir}`);
    console.log(`manifest: ${bakeManifestPath}`);
  } catch (error) {
    await browser?.close().catch(() => {});
    if (manifestPublished) {
      await unlink(bakeManifestPath).catch(() => {});
    }
    if (outputClaimed) {
      await rm(outputDir, { recursive: true, force: true });
    }
    await rm(stagingManifest, { force: true });
    await rm(stagingDir, { recursive: true, force: true });
    throw error;
  }
}

main().catch((error: unknown) => {
  console.error(error instanceof Error ? error.message : String(error));
  exit(1);
});
