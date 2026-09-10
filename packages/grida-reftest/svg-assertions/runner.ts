import { spawn } from "node:child_process";
import { constants } from "node:fs";
import { readFile, writeFile, mkdir, open } from "node:fs/promises";
import { dirname, join, resolve, relative } from "node:path";
import { fileURLToPath } from "node:url";
import {
  compare,
  decode,
  evaluate,
  gateReady,
  parseSuite,
  repeatProblem,
  sha256,
  MAX_PNG_BYTES,
  type Case,
  type Comparison,
  type Execution,
  type FileIdentity,
  type Image,
  type ImageRecord,
  type Observation,
  type Sample,
  type Suite,
  type Verdict,
} from "./model";
import { renderReport } from "./report";

export const toolDir = dirname(fileURLToPath(import.meta.url));
export const repoDir = resolve(toolDir, "../../..");
const capturePath = join(repoDir, "fixtures/web-first/chromium_capture.ts");
const env = {
  ...process.env,
  PATH: `${process.env.HOME}/.cargo/bin:${process.env.PATH}`,
};
const MAX_OUTPUT = 4 * 1024 * 1024;

/** Bound file input before allocation, including a file growing during read. */
export async function readBounded(
  path: string,
  limit: number
): Promise<Buffer> {
  // A FIFO must not block in open before descriptor validation can reject it.
  const file = await open(path, constants.O_RDONLY | constants.O_NONBLOCK);
  try {
    const stat = await file.stat();
    if (!stat.isFile() || stat.size > limit)
      throw new Error(`input exceeds file bound: ${path}`);
    const bytes = Buffer.alloc(stat.size + 1);
    let total = 0;
    while (total < bytes.length) {
      const { bytesRead } = await file.read(
        bytes,
        total,
        bytes.length - total,
        null
      );
      if (!bytesRead) break;
      total += bytesRead;
    }
    if (total > stat.size || total > limit)
      throw new Error(`input grew during read: ${path}`);
    return bytes.subarray(0, total);
  } finally {
    await file.close();
  }
}

/** No shell interpolation. Kill the process group on timeout/output overflow,
 * including Chromium grandchildren and cargo's child renderer. */
export function command(
  executable: string,
  args: string[],
  timeout = 60000
): Promise<Execution> {
  return new Promise((resolveResult) => {
    let stdout = "",
      stderr = "",
      error: string | null = null,
      bytes = 0;
    const child = spawn(executable, args, {
      cwd: repoDir,
      env,
      detached: process.platform !== "win32",
      stdio: ["ignore", "pipe", "pipe"],
    });
    const stop = (reason: string): void => {
      error ??= reason;
      try {
        if (child.pid && process.platform !== "win32")
          process.kill(-child.pid, "SIGKILL");
        else child.kill("SIGKILL");
      } catch {
        /* Already exited. */
      }
    };
    const timer = setTimeout(() => stop("execution-timeout"), timeout);
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => {
      bytes += Buffer.byteLength(chunk);
      if (bytes > MAX_OUTPUT) stop("output-limit");
      else stdout += chunk;
    });
    child.stderr.on("data", (chunk: string) => {
      bytes += Buffer.byteLength(chunk);
      if (bytes > MAX_OUTPUT) stop("output-limit");
      else stderr += chunk;
    });
    child.on("error", (e) => {
      error = e.message;
    });
    child.on("close", (exit, signal) => {
      clearTimeout(timer);
      resolveResult({
        command: [executable, ...args],
        exit,
        signal,
        error,
        stdout,
        stderr,
      });
    });
  });
}
const unavailable = (problem: string): Observation => ({
  problem,
  samples: [],
});
function successful(e: Execution): boolean {
  return e.exit === 0 && !e.signal && !e.error;
}

export function executionFailure(e: Execution): string {
  return (
    e.error ||
    e.stderr.trim() ||
    (e.signal
      ? `signal ${e.signal}`
      : e.exit === null
        ? "no exit status"
        : `exit code ${e.exit}`)
  );
}

/** Keep failed capture slots: a second image never masquerades as repeat zero. */
export async function chromiumObservation(
  execution: Execution,
  read: (index: number) => Promise<ImageRecord>
): Promise<Observation> {
  const observation: Observation = {
    samples: [],
    invocation: execution,
    problem: successful(execution)
      ? null
      : `Chromium capture failed: ${executionFailure(execution)}`,
  };
  for (let i = 0; i < 2; i++) {
    let image: ImageRecord | null = null;
    try {
      image = await read(i);
    } catch (error) {
      observation.problem ??= `Chromium output unavailable: ${String(error)}`;
    }
    observation.samples.push({
      execution,
      diagnostics: execution.stderr.trim(),
      image,
    });
  }
  return observation;
}

export function firstSampleComparisons(
  observations: Record<string, Observation>,
  images: ReadonlyMap<string, Image>
): Record<string, Comparison> {
  const result: Record<string, Comparison> = {};
  const labels = ["strict", "best", "chromium", "baked", "stored", "resvg"];
  for (let i = 0; i < labels.length; i++) {
    for (const right of labels.slice(i + 1)) {
      const left = labels[i];
      const a = observations[left]?.samples[0]?.image,
        b = observations[right]?.samples[0]?.image;
      const leftImage = a ? images.get(a.path) : undefined,
        rightImage = b ? images.get(b.path) : undefined;
      if (leftImage && rightImage)
        result[`${left}-${right}`] = compare(leftImage, rightImage);
    }
  }
  return result;
}

/** Remove only the CLI's exact known success footer for this invocation.
 * Everything else, including unknown stderr/stdout, remains evidence. */
export function cliDiagnostics(
  e: Execution,
  source: string,
  out: string,
  c: Pick<Case, "width" | "height">,
  bytes: number | null
): string {
  const lines = e.stderr.trimEnd().split("\n");
  const last = lines.at(-1) ?? "";
  const prefix = `rendered ${source} -> ${out} (${c.width}x${c.height}, base-shared-frame, `;
  if (e.exit === 0 && bytes !== null && last.startsWith(prefix)) {
    const suffix = last.slice(prefix.length);
    const declaredDegradations = suffix.match(/^(\d+) degraded, /);
    const actualDegradations = lines.filter((line) =>
      line.startsWith("degraded: ")
    ).length;
    if (
      suffix === `${bytes} bytes)` ||
      (declaredDegradations !== null &&
        Number(declaredDegradations[1]) === actualDegradations &&
        suffix.replace(/^\d+ degraded, /, "") === `${bytes} bytes)`)
    )
      lines.pop();
  }
  return lines.join("\n").trimEnd();
}
export interface CaseResult {
  case: Case;
  problems: string[];
  observations: Record<string, Observation>;
  pairs: Record<string, Comparison>;
  verdict: Verdict;
}
export interface Report {
  schema_version: 1;
  kind: "svg-assertion-observations";
  manifest: Suite;
  manifest_sha256: string;
  source_manifest: string;
  tools: Record<string, unknown>;
  integrity: string[];
  cases: CaseResult[];
  gate_ready: boolean;
}
export interface Options {
  manifest: string;
  out: string;
  resvg?: { executable: string; sha256: string; version: string };
}

export async function run(options: Options): Promise<Report> {
  const manifestPath = resolve(options.manifest),
    base = dirname(manifestPath),
    out = resolve(options.out);
  const manifestBytes = await readBounded(manifestPath, 1024 * 1024);
  const suite = parseSuite(JSON.parse(manifestBytes.toString("utf8")));
  // Caller supplies a NEW directory. An existing directory is never reused.
  await mkdir(out);
  const write = async (name: string, bytes: string | Buffer): Promise<void> => {
    await writeFile(join(out, name), bytes, { flag: "wx" });
  };
  await write("input-manifest.json", manifestBytes);
  const watched = new Map<string, string>([
    [manifestPath, sha256(manifestBytes)],
  ]);
  const inputLimits = new Map<string, number>([[manifestPath, 1024 * 1024]]);
  const integrity: string[] = [];
  const tools: Record<string, unknown> = {
    node: process.version,
    platform: process.platform,
    arch: process.arch,
    profile: suite.profile,
  };
  for (const path of [
    capturePath,
    ...[
      "model.ts",
      "runner.ts",
      "report.ts",
      "capture-worker.ts",
      "cli.ts",
    ].map((name) => join(toolDir, name)),
  ]) {
    const hash = sha256(await readFile(path));
    watched.set(path, hash);
    tools[relative(repoDir, path)] = hash;
  }
  if (watched.get(capturePath) !== suite.capture.sha256)
    integrity.push("capture-module-hash-drift");
  tools.chromium_version = suite.capture.browser_version;
  tools.git = await command("git", ["rev-parse", "HEAD"]);
  const build = await command(
    "cargo",
    [
      "build",
      "--locked",
      "--message-format=json",
      "-p",
      "n0_cli",
      "--bin",
      "n0",
    ],
    // The pinned Linux GL/SVG/WebP combination has no prebuilt Skia archive;
    // cold source compilation exceeds ten minutes on hosted CI. This build
    // budget is separate from the one-minute render/capture process bound.
    20 * 60000
  );
  tools.n0_build = build;
  let n0Path: string | null = null;
  if (successful(build)) {
    for (const line of build.stdout.split("\n")) {
      try {
        const artifact = JSON.parse(line);
        if (
          artifact.reason === "compiler-artifact" &&
          artifact.target?.name === "n0" &&
          typeof artifact.executable === "string"
        )
          n0Path = artifact.executable;
      } catch {
        /* Cargo may emit non-artifact lines. */
      }
    }
  }
  if (!n0Path) integrity.push("n0-build-unavailable");
  else {
    const hash = sha256(await readFile(n0Path));
    watched.set(n0Path, hash);
    tools.n0_binary_sha256 = hash;
  }
  let resvgProblem = "fresh resvg not requested";
  if (options.resvg) {
    try {
      const path = resolve(options.resvg.executable),
        hash = sha256(await readFile(path));
      watched.set(path, hash);
      const version = await command(path, ["--version"]);
      tools.resvg = { executable: path, sha256: hash, version };
      if (
        hash !== options.resvg.sha256 ||
        !successful(version) ||
        version.stdout.trim() !== options.resvg.version ||
        version.stderr
      )
        resvgProblem = "resvg identity mismatch";
      else resvgProblem = "";
    } catch (error) {
      resvgProblem = String(error);
    }
  }
  async function image(
    path: string,
    retain?: Map<string, Image>
  ): Promise<{ record: ImageRecord; bytes: Buffer }> {
    const bytes = await readBounded(path, MAX_PNG_BYTES),
      decoded = decode(bytes),
      name = relative(out, path);
    retain?.set(name, decoded);
    return {
      record: {
        path: name,
        png_sha256: sha256(bytes),
        rgba_sha256: sha256(decoded.rgba),
        width: decoded.width,
        height: decoded.height,
      },
      bytes,
    };
  }
  async function input(id: FileIdentity, limit: number): Promise<Buffer> {
    const path = resolve(base, id.path),
      bytes = await readBounded(path, limit);
    const hash = sha256(bytes);
    inputLimits.set(path, limit);
    if (watched.has(path) && watched.get(path) !== hash)
      integrity.push(`input changed between reads: ${id.path}`);
    else watched.set(path, hash);
    if (hash !== id.sha256) {
      integrity.push(`declared input hash mismatch: ${id.path}`);
      throw new Error(`input hash mismatch: ${id.path}`);
    }
    return bytes;
  }
  async function reference(
    c: Case,
    id: FileIdentity | null,
    label: string,
    images: Map<string, Image>
  ): Promise<Observation> {
    if (!id) return unavailable(`${label} not supplied`);
    try {
      const bytes = await input(id, MAX_PNG_BYTES),
        name = `${c.id}/${label}.png`;
      await write(name, bytes);
      const { record } = await image(join(out, name), images);
      const sample: Sample = {
        execution: {
          command: ["stored-file", id.path],
          exit: 0,
          signal: null,
          error: null,
          stdout: "",
          stderr: "",
        },
        diagnostics: "",
        image: record,
      };
      return { samples: [sample], problem: null };
    } catch (error) {
      return unavailable(String(error));
    }
  }
  const results: CaseResult[] = [];
  for (const c of suite.cases) {
    // Only first samples of this case are needed for its named comparisons.
    // Repeats retain hashes, never decoded buffers; completed cases release
    // their pixels. Cross-case controls are reloaded one pair at a time below.
    const images = new Map<string, Image>();
    await mkdir(join(out, c.id));
    const problems: string[] = [];
    const observations: Record<string, Observation> = {};
    const result: CaseResult = {
      case: c,
      problems,
      observations,
      pairs: {},
      verdict: { status: "OBSERVATION", kind: "observation", reasons: [] },
    };
    results.push(result);
    const sourcePath = join(out, c.id, "source.svg");
    let sourceReady = false;
    try {
      const bytes = await input(c.source, 1024 * 1024);
      new TextDecoder("utf-8", { fatal: true }).decode(bytes);
      await write(`${c.id}/source.svg`, bytes);
      watched.set(sourcePath, sha256(bytes));
      inputLimits.set(sourcePath, 1024 * 1024);
      sourceReady = true;
    } catch (error) {
      problems.push(String(error));
    }
    observations.baked = await reference(
      c,
      c.assertion?.kind === "render-exact" ? c.assertion.reference : null,
      "baked",
      images
    );
    observations.stored = await reference(c, c.stored, "stored", images);
    if (c.assertion?.kind === "render-exact" && observations.baked.problem)
      problems.push(observations.baked.problem);
    if (c.stored && observations.stored.problem)
      problems.push(observations.stored.problem);
    for (const admission of ["strict", "best"] as const) {
      const observation: Observation = { samples: [], problem: null };
      observations[admission] = observation;
      if (!sourceReady || !n0Path) {
        observation.problem = "source or n0 unavailable";
        continue;
      }
      for (let i = 0; i < 2; i++) {
        const output = join(out, c.id, `${admission}-${i}.png`);
        const execution = await command("cargo", [
          "run",
          "--locked",
          "--quiet",
          "-p",
          "n0_cli",
          "--bin",
          "n0",
          "--",
          sourcePath,
          output,
          `${c.width}x${c.height}`,
          admission === "strict" ? "--strict" : "--best-effort",
        ]);
        let record: ImageRecord | null = null,
          bytes: Buffer | null = null;
        try {
          const rendered = await image(output, i === 0 ? images : undefined);
          record = rendered.record;
          bytes = rendered.bytes;
        } catch (error) {
          if (
            execution.exit === 0 ||
            (error as NodeJS.ErrnoException).code !== "ENOENT"
          )
            execution.error ??= `missing/invalid output: ${String(error)}`;
        }
        observation.samples.push({
          execution,
          image: record,
          diagnostics: cliDiagnostics(
            execution,
            sourcePath,
            output,
            c,
            bytes?.length ?? null
          ),
        });
      }
    }
    const chromium: Observation = { samples: [], problem: null };
    observations.chromium = chromium;
    if (!sourceReady || integrity.includes("capture-module-hash-drift"))
      chromium.problem = "source or pinned capture unavailable";
    else {
      const execution = await command(process.execPath, [
        "--import",
        "tsx",
        join(toolDir, "capture-worker.ts"),
        sourcePath,
        join(out, c.id),
        String(c.width),
        String(c.height),
        suite.capture.browser_version,
      ]);
      observations.chromium = await chromiumObservation(
        execution,
        async (i) =>
          (
            await image(
              join(out, c.id, `chromium-${i}.png`),
              i === 0 ? images : undefined
            )
          ).record
      );
    }
    const resvg: Observation = { samples: [], problem: null };
    observations.resvg = resvg;
    if (
      resvgProblem ||
      !sourceReady ||
      c.review.status !== "reviewed" ||
      c.review.blockers.length
    )
      resvg.problem =
        resvgProblem || "source/environment not reviewed for local resvg";
    else if (options.resvg)
      for (let i = 0; i < 2; i++) {
        const output = join(out, c.id, `resvg-${i}.png`);
        const execution = await command(resolve(options.resvg.executable), [
          "--skip-system-fonts",
          "--width",
          String(c.width),
          sourcePath,
          output,
        ]);
        let record: ImageRecord | null = null;
        try {
          record = (await image(output, i === 0 ? images : undefined)).record;
        } catch (error) {
          execution.error ??= `missing/invalid output: ${String(error)}`;
        }
        resvg.samples.push({
          execution,
          diagnostics: execution.stderr.trim(),
          image: record,
        });
      }
    if (!resvg.problem) resvg.problem = repeatProblem(resvg);
    result.pairs = firstSampleComparisons(observations, images);
  }
  for (const result of results) {
    const c = result.case;
    if (c.assertion?.kind === "render-exact") {
      const id = c.assertion.control.case,
        control = results.find((r) => r.case.id === id);
      const a = result.observations.chromium.samples[0]?.image,
        b = control?.observations.chromium.samples[0]?.image;
      if (
        a &&
        b &&
        control &&
        !control.problems.length &&
        control.case.review.status === "reviewed" &&
        !control.case.review.blockers.length &&
        !repeatProblem(control.observations.chromium) &&
        control.observations.chromium.samples.every(
          (s) =>
            successful(s.execution) && !s.diagnostics && !s.execution.stdout
        )
      ) {
        try {
          const reload = async (record: ImageRecord): Promise<Image> => {
            const bytes = await readBounded(
              join(out, record.path),
              MAX_PNG_BYTES
            );
            if (sha256(bytes) !== record.png_sha256)
              throw new Error(`control output changed: ${record.path}`);
            return decode(bytes);
          };
          result.pairs["chromium-control"] = compare(
            await reload(a),
            await reload(b)
          );
        } catch (error) {
          result.problems.push(String(error));
        }
      }
    }
  }
  for (const [path, hash] of watched) {
    try {
      const limit = inputLimits.get(path);
      const bytes =
        limit === undefined
          ? await readFile(path)
          : await readBounded(path, limit);
      if (sha256(bytes) !== hash)
        integrity.push(
          `input/tool changed during run: ${relative(repoDir, path)}`
        );
    } catch {
      integrity.push(
        `input/tool disappeared during run: ${relative(repoDir, path)}`
      );
    }
  }
  for (const result of results) {
    result.verdict = evaluate(result.case, {
      problems: [...integrity, ...result.problems],
      strict: result.observations.strict,
      best: result.observations.best,
      chromium: result.observations.chromium,
      pairs: result.pairs,
    });
  }
  const verdicts = Object.fromEntries(
    results.map((r) => [r.case.id, r.verdict])
  );
  const report: Report = {
    schema_version: 1,
    kind: "svg-assertion-observations",
    manifest: suite,
    manifest_sha256: sha256(manifestBytes),
    source_manifest: manifestPath,
    tools,
    integrity,
    cases: results,
    gate_ready: gateReady(suite.cases, verdicts, integrity),
  };
  await write("report.json", `${JSON.stringify(report, null, 2)}\n`);
  await write("index.html", renderReport(report));
  return report;
}
