import { describe, expect, it } from "vitest";
import { parseEnvironment, environmentProblem } from "./reference-environment";

const identity = {
  schema_version: 1,
  host: {
    platform: "darwin",
    arch: "arm64",
    release: "25.5.0",
    version: "kernel-build",
  },
  browser: {
    product: "test",
    revision: "test",
    arch: "arm64",
    sha256: "a".repeat(64),
  },
  raster: {
    feature_status: {
      rasterization: "disabled_software",
      gpu_compositing: "disabled_software",
    },
    gl_implementation: "test",
    gl_renderer: "test",
    gl_version: "test",
  },
};
describe("declared reference environment", () => {
  it("compares typed identity independent of JSON key order", () => {
    const actual = structuredClone(identity);
    actual.raster.feature_status = {
      gpu_compositing: "disabled_software",
      rasterization: "disabled_software",
    };
    expect(environmentProblem(parseEnvironment(identity), actual)).toBeNull();
  });
  it.each(["host", "browser", "raster"] as const)(
    "refuses changed %s even with matching pixels",
    (key) => {
      const actual = structuredClone(identity);
      if (key === "host") actual.host.version = "other-kernel";
      if (key === "browser") actual.browser.arch = "x64";
      if (key === "raster")
        actual.raster.feature_status.rasterization = "enabled_on";
      expect(environmentProblem(parseEnvironment(identity), actual)).toBe(
        `reference-environment-mismatch: ${key}`
      );
    }
  );
  it("rejects missing, unknown, malformed, oversized or unrecognized identities", () => {
    for (const actual of [
      null,
      {},
      { ...identity, schema_version: 2 },
      { ...identity, optionalTypo: true },
      { ...identity, browser: { ...identity.browser, sha256: "unknown" } },
      { ...identity, host: { ...identity.host, arch: "a".repeat(8193) } },
      { ...identity, raster: { ...identity.raster, feature_status: {} } },
      { ...identity, raster: { ...identity.raster, gl_version: null } },
    ])
      expect(environmentProblem(parseEnvironment(identity), actual)).toContain(
        "reference-environment-unavailable"
      );
  });
});
