/** Assertion results are not image scores. This module owns no renderer,
 * process, filesystem, tolerance, or reference-selection heuristic. */
import { createHash } from "node:crypto";
import { PNG } from "pngjs";
import { crc32, inflateSync } from "node:zlib";

export type FileIdentity = { path: string; sha256: string };
export type Assertion =
  | {
      kind: "render-exact";
      reference: FileIdentity;
      decision: string;
      control: { case: string; why: string };
    }
  | {
      kind: "refusal";
      decision: string;
      strict: { exit: number; diagnostics: string };
      best: { exit: number; diagnostics: string };
    };
export interface Case {
  id: string;
  source: FileIdentity;
  description: string | null;
  required: boolean;
  width: number;
  height: number;
  review: {
    status: "reviewed" | "unreviewed";
    reason: string;
    blockers: string[];
  };
  assertion: Assertion | null;
  stored: FileIdentity | null;
}
export interface Suite {
  schema_version: 1;
  profile: "static-self-contained-svg-v1";
  capture: { sha256: string; browser_version: string };
  cases: Case[];
}
export const sha256 = (bytes: Uint8Array | string): string =>
  createHash("sha256").update(bytes).digest("hex");

// Reject unknown fields: a misspelled requirement must not quietly disappear.
function object(value: unknown, fields: string[]): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error("expected object");
  const record = value as Record<string, unknown>;
  if (
    Object.keys(record).some((key) => !fields.includes(key)) ||
    fields.some((key) => !(key in record))
  )
    throw new Error(`expected exactly: ${fields.join(", ")}`);
  return record;
}
function text(value: unknown): string {
  if (typeof value !== "string" || !value.trim() || value.length > 8192)
    throw new Error("expected nonempty bounded text");
  return value;
}
function hash(value: unknown): string {
  const result = text(value);
  if (!/^[a-f0-9]{64}$/.test(result)) throw new Error("expected sha256");
  return result;
}
function identity(value: unknown): FileIdentity {
  const f = object(value, ["path", "sha256"]);
  const path = text(f.path);
  if (path.includes("\0") || path.includes("\n"))
    throw new Error("invalid input path");
  return { path, sha256: hash(f.sha256) };
}
function dimension(value: unknown): number {
  if (!Number.isInteger(value) || Number(value) < 1 || Number(value) > 2048)
    throw new Error("dimensions must be 1..2048");
  return Number(value);
}
function departure(value: unknown): { exit: number; diagnostics: string } {
  const r = object(value, ["exit", "diagnostics"]);
  if (r.exit !== 0 && r.exit !== 1)
    throw new Error("refusal exit must be 0 or 1");
  return { exit: r.exit, diagnostics: text(r.diagnostics) };
}
function assertion(value: unknown): Assertion | null {
  if (value === null) return null;
  if (!value || typeof value !== "object" || !("kind" in value))
    throw new Error("missing assertion kind");
  if (value.kind === "render-exact") {
    const a = object(value, ["kind", "reference", "decision", "control"]);
    const c = object(a.control, ["case", "why"]);
    return {
      kind: "render-exact",
      reference: identity(a.reference),
      decision: text(a.decision),
      control: { case: text(c.case), why: text(c.why) },
    };
  }
  if (value.kind === "refusal") {
    const a = object(value, ["kind", "decision", "strict", "best"]);
    const strict = departure(a.strict),
      best = departure(a.best);
    if (
      strict.exit !== 1 ||
      !strict.diagnostics.startsWith("error: render failed: ")
    )
      throw new Error("strict must name a render refusal");
    if (best.exit === 0 && !best.diagnostics.startsWith("degraded: "))
      throw new Error("best effort must declare degradation");
    return { kind: "refusal", decision: text(a.decision), strict, best };
  }
  throw new Error("unknown assertion kind");
}
export function parseSuite(value: unknown): Suite {
  const s = object(value, ["schema_version", "profile", "capture", "cases"]);
  if (s.schema_version !== 1 || s.profile !== "static-self-contained-svg-v1")
    throw new Error("unsupported manifest profile/version");
  const cap = object(s.capture, ["sha256", "browser_version"]);
  if (!Array.isArray(s.cases) || !s.cases.length || s.cases.length > 128)
    throw new Error("declare 1..128 cases");
  const cases = s.cases.map((value) => {
    const c = object(value, [
      "id",
      "source",
      "description",
      "required",
      "width",
      "height",
      "review",
      "assertion",
      "stored",
    ]);
    const id = text(c.id),
      r = object(c.review, ["status", "reason", "blockers"]);
    if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(id) || id.length > 100)
      throw new Error("invalid case id");
    if (typeof c.required !== "boolean")
      throw new Error("required must be boolean");
    if (r.status !== "reviewed" && r.status !== "unreviewed")
      throw new Error("unknown review status");
    if (!Array.isArray(r.blockers))
      throw new Error("blockers must be explicit");
    const result: Case = {
      id,
      source: identity(c.source),
      description: c.description === null ? null : text(c.description),
      required: c.required,
      width: dimension(c.width),
      height: dimension(c.height),
      review: {
        status: r.status,
        reason: text(r.reason),
        blockers: r.blockers.map(text),
      },
      assertion: assertion(c.assertion),
      stored: c.stored === null ? null : identity(c.stored),
    };
    if (result.assertion && !result.description)
      throw new Error("assertions require a description");
    return result;
  });
  if (new Set(cases.map((c) => c.id)).size !== cases.length)
    throw new Error("duplicate case id");
  for (const c of cases) {
    const a = c.assertion;
    if (
      a?.kind === "render-exact" &&
      (!cases.some((other) => other.id === a.control.case) ||
        a.control.case === c.id)
    )
      throw new Error(`${c.id}: missing or self-referential control`);
  }
  return {
    schema_version: 1,
    profile: "static-self-contained-svg-v1",
    capture: {
      sha256: hash(cap.sha256),
      browser_version: text(cap.browser_version),
    },
    cases,
  };
}

export interface Image {
  width: number;
  height: number;
  rgba: Buffer;
}
export interface ImageRecord {
  path: string;
  png_sha256: string;
  rgba_sha256: string;
  width: number;
  height: number;
}
export const MAX_PNG_BYTES = 32 * 1024 * 1024;

export function decode(bytes: Buffer): Image {
  // pngjs accepts replacement IHDRs and erases RGB for non-palette tRNS.
  // Validate the complete bounded stream BEFORE trusting its allocation or
  // normalization. Interlaced inflate and 16-bit rescaling are outside v1.
  if (
    bytes.length < 33 ||
    bytes.length > MAX_PNG_BYTES ||
    !bytes.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]))
  )
    throw new Error("invalid PNG");
  let width = 0,
    height = 0,
    color = -1,
    depth = 0,
    palette = 0;
  let header = false,
    transparency = false,
    data = false,
    dataEnded = false,
    end = false;
  const idat: Buffer[] = [];
  for (let offset = 8; offset < bytes.length;) {
    if (offset + 12 > bytes.length) throw new Error("truncated PNG chunk");
    const length = bytes.readUInt32BE(offset),
      next = offset + length + 12;
    if (next > bytes.length) throw new Error("truncated PNG chunk");
    const type = bytes.toString("latin1", offset + 4, offset + 8);
    if (!/^[A-Za-z]{2}[A-Z][A-Za-z]$/.test(type))
      throw new Error("invalid PNG chunk type");
    if (
      crc32(bytes.subarray(offset + 4, next - 4)) !==
      bytes.readUInt32BE(next - 4)
    )
      throw new Error("invalid PNG chunk CRC");
    if (!header && type !== "IHDR") throw new Error("PNG must begin with IHDR");
    if (type === "IHDR") {
      if (header || offset !== 8 || length !== 13)
        throw new Error("invalid or duplicate PNG IHDR");
      header = true;
      width = dimension(bytes.readUInt32BE(offset + 8));
      height = dimension(bytes.readUInt32BE(offset + 12));
      depth = bytes[offset + 16];
      color = bytes[offset + 17];
      if (
        (color === 3
          ? ![1, 2, 4, 8].includes(depth)
          : depth !== 8 || ![0, 2, 4, 6].includes(color)) ||
        bytes[offset + 18] !== 0 ||
        bytes[offset + 19] !== 0 ||
        bytes[offset + 20] !== 0
      )
        throw new Error(
          "unsupported PNG encoding: require noninterlaced 8-bit channels or indexed palette"
        );
    } else if (type === "PLTE") {
      if (
        palette ||
        data ||
        length === 0 ||
        length % 3 ||
        length > 768 ||
        ![2, 3, 6].includes(color) ||
        (color === 3 && length / 3 > 2 ** depth)
      )
        throw new Error("invalid PNG palette");
      palette = length / 3;
    } else if (type === "tRNS") {
      if (color !== 3)
        throw new Error(
          "unsupported PNG encoding: non-palette tRNS loses hidden RGB"
        );
      if (transparency || data || !palette || length === 0 || length > palette)
        throw new Error("invalid PNG transparency");
      transparency = true;
    } else if (type === "IDAT") {
      if (dataEnded || (color === 3 && !palette))
        throw new Error("invalid PNG data order");
      data = true;
      idat.push(bytes.subarray(offset + 8, next - 4));
    } else if (type === "IEND") {
      if (!data || length !== 0 || next !== bytes.length)
        throw new Error("invalid PNG end");
      end = true;
    } else {
      if (["acTL", "fcTL", "fdAT"].includes(type) || /^[A-Z]/.test(type))
        throw new Error(`unsupported PNG chunk: ${type}`);
    }
    if (data && type !== "IDAT") dataEnded = true;
    offset = next;
  }
  if (!end) throw new Error("missing PNG end");
  // pngjs's bounded inflater tolerates incomplete streams/scanlines, inventing
  // transparent pixels. Native strict inflate must first verify the checksum,
  // complete input consumption and exact row extent under a hard output cap.
  const channels = color === 2 ? 3 : color === 4 ? 2 : color === 6 ? 4 : 1;
  const scanlineBytes =
    (Math.ceil((width * channels * depth) / 8) + 1) * height;
  const compressed = Buffer.concat(idat);
  try {
    // Node's info:true runtime result is richer than @types/node's overload.
    const inflated = inflateSync(compressed, {
      maxOutputLength: scanlineBytes + 1,
      info: true,
    }) as unknown as {
      buffer: Buffer;
      engine: { bytesWritten: number };
    };
    if (
      inflated.buffer.length !== scanlineBytes ||
      inflated.engine.bytesWritten !== compressed.length
    )
      throw new Error("incomplete or surplus compressed data / scanlines");
  } catch (error) {
    throw new Error(`invalid PNG compressed data: ${String(error)}`);
  }
  const p = PNG.sync.read(bytes);
  if (
    p.width !== width ||
    p.height !== height ||
    p.data.length !== width * height * 4
  )
    throw new Error("invalid RGBA length");
  return { width: p.width, height: p.height, rgba: p.data };
}
export type Comparison =
  | { relation: "dimension-mismatch"; left: number[]; right: number[] }
  | {
      relation: "exact-rgba" | "different-rgba";
      pixels: number;
      max_delta: number;
      hidden_rgb_pixels: number;
      alpha_pixels: number;
      bounds: number[] | null;
    };
export function compare(a: Image, b: Image): Comparison {
  if (a.width !== b.width || a.height !== b.height)
    return {
      relation: "dimension-mismatch",
      left: [a.width, a.height],
      right: [b.width, b.height],
    };
  let pixels = 0,
    max_delta = 0,
    hidden_rgb_pixels = 0,
    alpha_pixels = 0;
  let minX = a.width,
    minY = a.height,
    maxX = -1,
    maxY = -1;
  for (let i = 0; i < a.rgba.length; i += 4) {
    let different = false;
    for (let ch = 0; ch < 4; ch++) {
      const d = Math.abs(a.rgba[i + ch] - b.rgba[i + ch]);
      different ||= d !== 0;
      max_delta = Math.max(max_delta, d);
    }
    if (!different) continue;
    pixels++;
    if (a.rgba[i + 3] === 0 && b.rgba[i + 3] === 0) hidden_rgb_pixels++;
    if (a.rgba[i + 3] !== b.rgba[i + 3]) alpha_pixels++;
    const x = (i / 4) % a.width,
      y = Math.floor(i / 4 / a.width);
    minX = Math.min(minX, x);
    maxX = Math.max(maxX, x);
    minY = Math.min(minY, y);
    maxY = Math.max(maxY, y);
  }
  return {
    relation: pixels ? "different-rgba" : "exact-rgba",
    pixels,
    max_delta,
    hidden_rgb_pixels,
    alpha_pixels,
    bounds: pixels ? [minX, minY, maxX, maxY] : null,
  };
}
export interface Execution {
  command: string[];
  exit: number | null;
  signal: string | null;
  error: string | null;
  stdout: string;
  stderr: string;
}
export interface Sample {
  execution: Execution;
  diagnostics: string;
  image: ImageRecord | null;
}
export interface Observation {
  samples: Sample[];
  problem: string | null;
  invocation?: Execution;
}
export type Verdict = {
  status: "PASS" | "FAIL" | "UNRESOLVED" | "OBSERVATION";
  kind: "render-exact" | "refusal" | "observation";
  reasons: string[];
};
export interface Evidence {
  problems: string[];
  strict: Observation;
  best: Observation;
  chromium: Observation;
  pairs: Record<string, Comparison>;
}
function sameImage(a: ImageRecord | null, b: ImageRecord | null): boolean {
  return a === null
    ? b === null
    : b !== null &&
        a.width === b.width &&
        a.height === b.height &&
        a.rgba_sha256 === b.rgba_sha256;
}
export function repeatProblem(o: Observation): string | null {
  if (o.problem) return o.problem;
  if (o.samples.length !== 2) return "missing repeated observation";
  const [a, b] = o.samples;
  if (
    a.execution.error ||
    b.execution.error ||
    a.execution.signal ||
    b.execution.signal
  )
    return "renderer execution failed";
  if (
    a.execution.exit !== b.execution.exit ||
    a.diagnostics !== b.diagnostics ||
    a.execution.stdout !== b.execution.stdout ||
    !sameImage(a.image, b.image) ||
    a.image?.png_sha256 !== b.image?.png_sha256
  )
    return "nondeterministic observation";
  return null;
}
function rendered(o: Observation, c: Case): boolean {
  return (
    !repeatProblem(o) &&
    o.samples.every(
      (s) =>
        s.execution.exit === 0 &&
        !s.execution.stdout &&
        !s.diagnostics &&
        s.image?.width === c.width &&
        s.image.height === c.height
    )
  );
}
export function evaluate(c: Case, e: Evidence): Verdict {
  const kind = c.assertion?.kind ?? "observation";
  if (!c.assertion)
    return {
      status: "OBSERVATION",
      kind,
      reasons: [...e.problems, "no reviewed assertion"],
    };
  if (
    e.problems.length ||
    c.review.status !== "reviewed" ||
    c.review.blockers.length
  )
    return {
      status: "UNRESOLVED",
      kind,
      reasons: [
        ...e.problems,
        ...c.review.blockers,
        ...(c.review.status !== "reviewed" ? ["claim not reviewed"] : []),
      ],
    };
  const reasons: string[] = [];
  if (c.assertion.kind === "refusal") {
    for (const admission of ["strict", "best"] as const) {
      const o = e[admission],
        expected = c.assertion[admission];
      const p = repeatProblem(o);
      if (p) reasons.push(`${admission}: ${p}`);
      if (
        o.samples.some(
          (s) =>
            s.execution.exit !== expected.exit ||
            s.execution.stdout ||
            s.diagnostics !== expected.diagnostics ||
            (expected.exit === 0
              ? s.image?.width !== c.width || s.image?.height !== c.height
              : s.image !== null)
        )
      )
        reasons.push(
          `${admission}: expected named refusal/degradation not observed`
        );
    }
  } else {
    if (!rendered(e.strict, c) || !rendered(e.best, c))
      reasons.push("n0 did not render repeatably without degradation");
    if (!rendered(e.chromium, c))
      return {
        status: "UNRESOLVED",
        kind,
        reasons: [
          ...reasons,
          "Chromium reference observation unavailable or unstable",
        ],
      };
    for (const pair of [
      "strict-baked",
      "best-baked",
      "chromium-baked",
      "strict-best",
    ])
      if (e.pairs[pair]?.relation !== "exact-rgba")
        reasons.push(`${pair}: exact assertion violated or comparison missing`);
    if (e.pairs["chromium-control"]?.relation !== "different-rgba")
      return {
        status: "UNRESOLVED",
        kind,
        reasons: [
          ...reasons,
          "control does not demonstrate the claimed branch",
        ],
      };
  }
  return { status: reasons.length ? "FAIL" : "PASS", kind, reasons };
}
/** A required case cannot disappear, be replaced, or become a silent skip. */
export function gateReady(
  cases: Case[],
  verdicts: Record<string, Verdict>,
  integrity: string[]
): boolean {
  return (
    !integrity.length &&
    Object.keys(verdicts).length === cases.length &&
    cases.some((c) => c.required) &&
    cases.every(
      (c) =>
        c.id in verdicts && (!c.required || verdicts[c.id].status === "PASS")
    )
  );
}
