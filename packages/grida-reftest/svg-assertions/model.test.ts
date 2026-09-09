import { describe, expect, it } from "vitest";
import { PNG } from "pngjs";
import { crc32, deflateSync } from "node:zlib";
import {
  compare,
  decode,
  evaluate,
  gateReady,
  parseSuite,
  repeatProblem,
  sha256,
  type Case,
  type Evidence,
  type Image,
  type Observation,
  type Suite,
} from "./model";

const hash = "a".repeat(64);
const file = { path: "subject.svg", sha256: hash };
function rendering(id = "subject", control = "control"): Case {
  return {
    id,
    source: file,
    description: "The solid rectangle covers its underlay.",
    required: true,
    width: 2,
    height: 1,
    review: {
      status: "reviewed",
      reason: "Hermetic source and active color control reviewed.",
      blockers: [],
    },
    stored: null,
    assertion: {
      kind: "render-exact",
      decision: "Pinned Chromium compatibility, not general conformance.",
      reference: { path: "expected.png", sha256: hash },
      control: {
        case: control,
        why: "Changing the subject fill changes the result.",
      },
    },
  };
}
function suite(): Suite {
  return {
    schema_version: 1,
    profile: "static-self-contained-svg-v1",
    capture: { sha256: hash, browser_version: "test-version" },
    cases: [rendering(), rendering("control", "subject")],
  };
}
function observation(): Observation {
  const sample = {
    execution: {
      command: ["test-observation"],
      exit: 0,
      signal: null,
      error: null,
      stdout: "",
      stderr: "",
    },
    diagnostics: "",
    image: {
      path: "image.png",
      png_sha256: hash,
      rgba_sha256: hash,
      width: 2,
      height: 1,
    },
  };
  return {
    problem: null,
    samples: [structuredClone(sample), structuredClone(sample)],
  };
}
const image = (values: number[], width = 2, height = 1): Image => ({
  width,
  height,
  rgba: Buffer.from(values),
});
const original = image([0, 0, 0, 0, 20, 30, 40, 255]);
const changed = image([1, 0, 0, 0, 20, 30, 40, 255]);

function chunk(type: string, data = Buffer.alloc(0)): Buffer {
  const result = Buffer.alloc(data.length + 12);
  result.writeUInt32BE(data.length);
  result.write(type, 4, "ascii");
  data.copy(result, 8);
  result.writeUInt32BE(crc32(result.subarray(4, -4)), result.length - 4);
  return result;
}
function ihdr(
  width = 1,
  height = 1,
  depth = 8,
  color = 6,
  interlace = 0
): Buffer {
  const data = Buffer.alloc(13);
  data.writeUInt32BE(width);
  data.writeUInt32BE(height, 4);
  data[8] = depth;
  data[9] = color;
  data[12] = interlace;
  return chunk("IHDR", data);
}
function png(...chunks: Buffer[]): Buffer {
  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    ...chunks,
  ]);
}
function evidence(): Evidence {
  return {
    problems: [],
    strict: observation(),
    best: observation(),
    chromium: observation(),
    pairs: {
      "strict-baked": compare(original, original),
      "best-baked": compare(original, original),
      "chromium-baked": compare(original, original),
      "strict-best": compare(original, original),
      "chromium-control": compare(original, changed),
    },
  };
}
describe("strict case contracts", () => {
  it("accepts only a described assertion with an identified independent expectation", () =>
    expect(parseSuite(suite())).toEqual(suite()));
  it.each(["similarity", "tolerance", "score", "optionalTypo"])(
    "rejects an unknown field %s",
    (key) => {
      const s = { ...suite(), [key]: true };
      expect(() => parseSuite(s)).toThrow("expected exactly");
    }
  );
  it("rejects empty, duplicate and missing-control cases", () => {
    expect(() => parseSuite({ ...suite(), cases: [] })).toThrow(
      "declare 1..128 cases"
    );
    expect(() =>
      parseSuite({ ...suite(), cases: [rendering(), rendering()] })
    ).toThrow("duplicate case id");
    expect(() => parseSuite({ ...suite(), cases: [rendering()] })).toThrow(
      "missing or self-referential control"
    );
  });
  it("rejects undescribed assertions, path IDs, dimensions and malformed hashes", () => {
    const invalid: Array<[Partial<Case>, string]> = [
      [{ description: null }, "assertions require a description"],
      [{ id: "../escape" }, "invalid case id"],
      [{ width: 0 }, "dimensions must be 1..2048"],
      [{ height: 2049 }, "dimensions must be 1..2048"],
      [{ source: { ...file, sha256: "unknown" } }, "expected sha256"],
    ];
    for (const [patch, reason] of invalid)
      expect(() =>
        parseSuite({
          ...suite(),
          cases: [
            { ...rendering(), ...patch },
            rendering("control", "subject"),
          ],
        })
      ).toThrow(reason);
  });
});
describe("exact pixels, not image grades", () => {
  it("one hidden RGB byte fails exact comparison and stays attributed", () =>
    expect(compare(original, changed)).toEqual({
      relation: "different-rgba",
      pixels: 1,
      max_delta: 1,
      hidden_rgb_pixels: 1,
      alpha_pixels: 0,
      bounds: [0, 0, 0, 0],
    }));
  it("rejects equal pixel counts with different dimensions", () =>
    expect(compare(original, image([...original.rgba], 1, 2)).relation).toBe(
      "dimension-mismatch"
    ));
  it("compares decoded pixels rather than PNG compression", () => {
    const png = new PNG({ width: 2, height: 1 });
    png.data.set(original.rgba);
    const a = PNG.sync.write(png, { deflateLevel: 0 }),
      b = PNG.sync.write(png, { deflateLevel: 9 });
    expect(a.equals(b)).toBe(false);
    expect(compare(decode(a), decode(b)).relation).toBe("exact-rgba");
  });
  it("rejects invalid and oversized PNGs before decoding", () => {
    expect(() => decode(Buffer.alloc(24))).toThrow("invalid PNG");
    const png = PNG.sync.write(new PNG({ width: 1, height: 1 }));
    png.writeUInt32BE(99999, 16);
    png.writeUInt32BE(crc32(png.subarray(12, 29)), 29);
    expect(() => decode(png)).toThrow("dimensions must be 1..2048");
  });
  it("refuses non-palette tRNS before the decoder can erase hidden RGB", () => {
    for (const rgb of [
      [1, 2, 3],
      [4, 5, 6],
    ]) {
      const transparent = Buffer.alloc(6);
      rgb.forEach((value, i) => transparent.writeUInt16BE(value, i * 2));
      const bytes = png(
        ihdr(1, 1, 8, 2),
        chunk("tRNS", transparent),
        chunk("IDAT", deflateSync(Buffer.from([0, ...rgb]))),
        chunk("IEND")
      );
      expect(() => decode(bytes)).toThrow("non-palette tRNS loses hidden RGB");
    }
  });
  it("refuses empty, truncated, unchecked or surplus compressed image data", () => {
    const complete = deflateSync(Buffer.from([0, 1, 2, 3, 0]));
    const wrongChecksum = Buffer.from(complete);
    wrongChecksum[wrongChecksum.length - 1] ^= 1;
    for (const compressed of [
      Buffer.alloc(0),
      deflateSync(Buffer.from([0, 1, 2, 3])),
      complete.subarray(0, -1),
      complete.subarray(0, -4),
      wrongChecksum,
      Buffer.concat([complete, Buffer.from([0])]),
      deflateSync(Buffer.alloc(6)),
      deflateSync(Buffer.alloc(100000)),
    ])
      expect(() =>
        decode(png(ihdr(), chunk("IDAT", compressed), chunk("IEND")))
      ).toThrow("invalid PNG compressed data");
  });
  it("accepts one complete zlib stream split across contiguous empty or nonempty IDATs", () => {
    const compressed = deflateSync(Buffer.from([0, 1, 2, 3, 0]));
    const bytes = png(
      ihdr(),
      chunk("IDAT"),
      chunk("IDAT", compressed.subarray(0, 3)),
      chunk("IDAT"),
      chunk("IDAT", compressed.subarray(3)),
      chunk("IDAT"),
      chunk("IEND")
    );
    expect([...decode(bytes).rgba]).toEqual([1, 2, 3, 0]);
  });
  it("preserves every hidden RGB channel in palette and RGBA8 PNGs", () => {
    const decoded = [
      [1, 2, 3],
      [4, 5, 6],
    ].map((rgb) => {
      const bytes = png(
        ihdr(1, 1, 1, 3),
        chunk("PLTE", Buffer.from(rgb)),
        chunk("tRNS", Buffer.from([0])),
        chunk("IDAT", deflateSync(Buffer.from([0, 0]))),
        chunk("IEND")
      );
      const result = decode(bytes);
      expect([...result.rgba]).toEqual([...rgb, 0]);
      const rgba = new PNG({ width: 1, height: 1 });
      rgba.data.set([...rgb, 0]);
      expect(compare(result, decode(PNG.sync.write(rgba))).relation).toBe(
        "exact-rgba"
      );
      return result;
    });
    expect(compare(decoded[0], decoded[1])).toMatchObject({
      relation: "different-rgba",
      hidden_rgb_pixels: 1,
      pixels: 1,
    });
  });
  it("rejects replacement headers before their dimensions reach allocation", () => {
    const bytes = png(
      ihdr(),
      ihdr(2049, 1),
      chunk("IDAT", deflateSync(Buffer.alloc(1 + 2049 * 4))),
      chunk("IEND")
    );
    expect(() => decode(bytes)).toThrow("duplicate PNG IHDR");
  });
  it("refuses rescaling, unbounded interlaced inflate and animation encodings", () => {
    for (const header of [ihdr(1, 1, 16), ihdr(1, 1, 8, 6, 1)])
      expect(() =>
        decode(
          png(
            header,
            chunk("IDAT", deflateSync(Buffer.alloc(9))),
            chunk("IEND")
          )
        )
      ).toThrow("unsupported PNG encoding");
    expect(() =>
      decode(
        png(
          ihdr(),
          chunk("acTL", Buffer.alloc(8)),
          chunk("IDAT", deflateSync(Buffer.alloc(5))),
          chunk("IEND")
        )
      )
    ).toThrow("unsupported PNG chunk: acTL");
  });
  it("rejects malformed chunk boundaries, CRC, ordering and trailing bytes", () => {
    const good = png(
      ihdr(),
      chunk("IDAT", deflateSync(Buffer.alloc(5))),
      chunk("IEND")
    );
    const badCrc = Buffer.from(good);
    badCrc[29] ^= 1;
    expect(() => decode(badCrc)).toThrow("CRC");
    expect(() => decode(good.subarray(0, -1))).toThrow("truncated PNG chunk");
    expect(() => decode(Buffer.concat([good, Buffer.from([0])]))).toThrow(
      "invalid PNG end"
    );
    expect(() => decode(png(chunk("tEXt"), ihdr(), chunk("IEND")))).toThrow(
      "begin with IHDR"
    );
    expect(() =>
      decode(
        png(ihdr(), chunk("IDAT"), chunk("tEXt"), chunk("IDAT"), chunk("IEND"))
      )
    ).toThrow("data order");
  });
});
describe("assertion verdicts", () => {
  it("accepts a fully evidenced exact assertion", () =>
    expect(evaluate(rendering(), evidence()).status).toBe("PASS"));
  it("does not let a matching second opinion rescue a declared-reference mismatch", () => {
    const e = evidence();
    e.pairs["strict-baked"] = compare(original, changed);
    e.pairs["strict-resvg"] = compare(original, original);
    expect(evaluate(rendering(), e).status).toBe("FAIL");
  });
  it("fails a missing comparison even if every available image matches", () => {
    const e = evidence();
    delete e.pairs["best-baked"];
    expect(evaluate(rendering(), e).status).toBe("FAIL");
  });
  it("does not admit matching omissions or unexpected refusals", () => {
    const e = evidence();
    e.best.samples.forEach(
      (s) =>
        (s.diagnostics = "degraded: skipped svg/image[1]: unsupported image")
    );
    expect(evaluate(rendering(), e).status).toBe("FAIL");
    e.strict.samples.forEach((s) => {
      s.execution.exit = 1;
      s.image = null;
    });
    expect(evaluate(rendering(), e).status).toBe("FAIL");
  });
  it("does not judge an environment-blocked or unreviewed source", () => {
    const c = rendering();
    c.review.blockers.push("relative image not provided");
    expect(evaluate(c, evidence()).status).toBe("UNRESOLVED");
    c.review.blockers = [];
    c.review.status = "unreviewed";
    expect(evaluate(c, evidence()).status).toBe("UNRESOLVED");
  });
  it("does not judge a placeholder with an ineffective control", () => {
    const e = evidence();
    e.pairs["chromium-control"] = compare(original, original);
    expect(evaluate(rendering(), e).status).toBe("UNRESOLVED");
  });
  it("preserves undescribed inputs as observations", () => {
    const c = rendering();
    c.assertion = null;
    c.description = null;
    expect(evaluate(c, evidence()).status).toBe("OBSERVATION");
  });
  it("refuses to judge with changed inputs or an unavailable browser", () => {
    const e = evidence();
    e.problems.push("hash drift");
    expect(evaluate(rendering(), e).status).toBe("UNRESOLVED");
    e.problems = [];
    e.chromium.problem = "wrong version";
    expect(evaluate(rendering(), e).status).toBe("UNRESOLVED");
  });
  it("detects missing outputs, crashes and nondeterministic repeats", () => {
    for (const fault of ["missing", "crash", "different", "diagnostic"]) {
      const e = evidence();
      if (fault === "missing") e.best.samples[1].image = null;
      if (fault === "crash") e.best.samples[1].execution.signal = "SIGSEGV";
      if (fault === "different")
        e.best.samples[1].image!.rgba_sha256 = sha256("different");
      if (fault === "diagnostic")
        e.best.samples[1].diagnostics = "warning: changed";
      expect(repeatProblem(e.best)).not.toBeNull();
      expect(evaluate(rendering(), e).status).toBe("FAIL");
    }
  });
  it("verifies named refusals without turning them into rendering support", () => {
    const c = rendering();
    c.assertion = {
      kind: "refusal",
      decision: "Guard the declared unsupported unit.",
      strict: {
        exit: 1,
        diagnostics: "error: render failed: attribute cx is not a number",
      },
      best: {
        exit: 0,
        diagnostics:
          "degraded: skipped svg/circle[1]: attribute cx is not a number",
      },
    };
    const expected = c.assertion;
    const e = evidence();
    e.strict.samples.forEach((s) => {
      s.execution.exit = 1;
      s.image = null;
      s.diagnostics = expected.strict.diagnostics;
    });
    e.best.samples.forEach((s) => {
      s.diagnostics = expected.best.diagnostics;
    });
    expect(evaluate(c, e)).toEqual({
      status: "PASS",
      kind: "refusal",
      reasons: [],
    });
    e.best.samples.forEach((s) => (s.diagnostics = ""));
    expect(evaluate(c, e).status).toBe("FAIL");
    e.strict.samples = [];
    expect(evaluate(c, e).status).toBe("FAIL");
  });
  it("fails closed on omitted, substituted, unresolved, or empty required gates", () => {
    const c = rendering(),
      v = evaluate(c, evidence());
    expect(gateReady([c], { subject: v }, [])).toBe(true);
    expect(gateReady([c], {}, [])).toBe(false);
    expect(gateReady([c], { replacement: v }, [])).toBe(false);
    expect(
      gateReady([c], { subject: { ...v, status: "UNRESOLVED" } }, [])
    ).toBe(false);
    expect(gateReady([c], { subject: v }, ["tool drift"])).toBe(false);
    expect(gateReady([], {}, [])).toBe(false);
  });
});
