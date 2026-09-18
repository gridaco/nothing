/** A reference environment is an input, never inferred from matching pixels.
 * No launcher, renderer selection, tolerance, or alternative reference lives here. */
export interface ReferenceEnvironment {
  schema_version: 1;
  host: { platform: string; arch: string; release: string; version: string };
  browser: { product: string; revision: string; arch: string; sha256: string };
  raster: {
    feature_status: Record<string, string>;
    gl_implementation: string;
    gl_renderer: string;
    gl_version: string;
  };
}

function object(value: unknown, keys: string[]): Record<string, unknown> {
  if (
    !value ||
    typeof value !== "object" ||
    Array.isArray(value) ||
    Object.keys(value).length !== keys.length ||
    keys.some((k) => !(k in value))
  )
    throw new Error(`reference identity requires exactly: ${keys.join(", ")}`);
  return value as Record<string, unknown>;
}
function text(value: unknown): string {
  if (typeof value !== "string" || !value.trim() || value.length > 8192)
    throw new Error("reference identity requires bounded nonempty text");
  return value;
}
export function parseEnvironment(value: unknown): ReferenceEnvironment {
  const e = object(value, ["schema_version", "host", "browser", "raster"]);
  if (e.schema_version !== 1)
    throw new Error("unknown reference environment version");
  const host = object(e.host, ["platform", "arch", "release", "version"]);
  const browser = object(e.browser, ["product", "revision", "arch", "sha256"]);
  const raster = object(e.raster, [
    "feature_status",
    "gl_implementation",
    "gl_renderer",
    "gl_version",
  ]);
  const features = raster.feature_status;
  if (
    !features ||
    typeof features !== "object" ||
    Array.isArray(features) ||
    !Object.keys(features).length ||
    Object.keys(features).length > 64
  )
    throw new Error("missing or unbounded raster feature identity");
  const sha256 = text(browser.sha256);
  if (!/^[a-f0-9]{64}$/.test(sha256)) throw new Error("invalid browser hash");
  return {
    schema_version: 1,
    host: {
      platform: text(host.platform),
      arch: text(host.arch),
      release: text(host.release),
      version: text(host.version),
    },
    browser: {
      product: text(browser.product),
      revision: text(browser.revision),
      arch: text(browser.arch),
      sha256,
    },
    raster: {
      feature_status: Object.fromEntries(
        Object.entries(features)
          .sort(([a], [b]) => a.localeCompare(b))
          .map(([k, v]) => [text(k), text(v)])
      ),
      gl_implementation: text(raster.gl_implementation),
      gl_renderer: text(raster.gl_renderer),
      gl_version: text(raster.gl_version),
    },
  };
}

/** The complete declared identity is matched, including kernel build text.
 * Missing or extra identity fields fail closed. No architecture shortcut. */
export function environmentProblem(
  expected: ReferenceEnvironment,
  observation: unknown
): string | null {
  try {
    const actual = parseEnvironment(observation);
    for (const key of ["host", "browser", "raster"] as const)
      if (JSON.stringify(actual[key]) !== JSON.stringify(expected[key]))
        return `reference-environment-mismatch: ${key}`;
    return null;
  } catch (error) {
    return `reference-environment-unavailable: ${String(error)}`;
  }
}
