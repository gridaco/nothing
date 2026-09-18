/** Passive provenance of the browser launched by the sole capture module.
 * Owns no launcher, capture posture, image normalization, or reference choice. */
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { open, readlink } from "node:fs/promises";
import { release, version } from "node:os";
import { promisify } from "node:util";
import type { Browser } from "@playwright/test";

/** Read the executable's ABI, not Node's architecture or the user-agent text. */
export function executableArch(header: Buffer): "arm64" | "x64" {
  if (header.length < 20) throw new Error("truncated executable header");
  if (header.readUInt32LE(0) === 0xfeedfacf) {
    const cpu = header.readUInt32LE(4);
    if (cpu === 0x0100000c) return "arm64";
    if (cpu === 0x01000007) return "x64";
  }
  if (
    header.subarray(0, 4).equals(Buffer.from([127, 69, 76, 70])) &&
    header[4] === 2 &&
    header[5] === 1
  ) {
    const cpu = header.readUInt16LE(18);
    if (cpu === 183) return "arm64";
    if (cpu === 62) return "x64";
  }
  throw new Error("unsupported browser executable ABI");
}

/** Hash one bounded regular executable without retaining it in memory. */
async function executableIdentity(path: string) {
  const file = await open(path, "r");
  try {
    const before = await file.stat();
    if (!before.isFile() || before.size > 512 * 1024 * 1024)
      throw new Error("browser executable outside file bound");
    const header = Buffer.alloc(20);
    const { bytesRead } = await file.read(header, 0, header.length, 0);
    const arch = executableArch(header.subarray(0, bytesRead));
    const hash = createHash("sha256");
    let count = 0;
    for await (const chunk of createReadStream(path, {
      fd: file.fd,
      autoClose: false,
      start: 0,
    })) {
      count += chunk.length;
      if (count > before.size) throw new Error("browser executable grew");
      hash.update(chunk);
    }
    const after = await file.stat();
    if (
      count !== before.size ||
      before.size !== after.size ||
      before.mtimeMs !== after.mtimeMs ||
      before.ctimeMs !== after.ctimeMs
    )
      throw new Error("browser executable changed while reading");
    return { arch, sha256: hash.digest("hex") };
  } finally {
    await file.close();
  }
}

export async function captureIdentity(browser: Browser) {
  const session = await browser.newBrowserCDPSession();
  try {
    const browserVersion = await session.send("Browser.getVersion");
    const system = await session.send("SystemInfo.getInfo");
    const processes = await session.send("SystemInfo.getProcessInfo");
    const pid = processes.processInfo.find((p) => p.type === "browser")?.id;
    if (!pid || !Number.isSafeInteger(pid) || pid < 1)
      throw new Error("browser process identity unavailable");
    let executable: string;
    if (process.platform === "linux")
      executable = await readlink(`/proc/${pid}/exe`);
    else if (process.platform === "darwin") {
      executable = (
        await promisify(execFile)("ps", ["-p", String(pid), "-o", "comm="], {
          timeout: 5000,
          maxBuffer: 16384,
        })
      ).stdout.trim();
    } else throw new Error("browser identity requires Linux or macOS");
    const binary = await executableIdentity(executable);
    const aux = system.gpu.auxAttributes ?? {};
    return {
      schema_version: 1,
      host: {
        platform: process.platform,
        arch: process.arch,
        release: release(),
        version: version(),
      },
      browser: {
        product: browserVersion.product,
        revision: browserVersion.revision,
        ...binary,
      },
      raster: {
        feature_status: system.gpu.featureStatus,
        gl_implementation: aux.glImplementationParts ?? null,
        gl_renderer: aux.glRenderer ?? null,
        gl_version: aux.glVersion ?? null,
      },
    };
  } finally {
    await session.detach();
  }
}
