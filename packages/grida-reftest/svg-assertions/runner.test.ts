import { describe, expect, it } from "vitest";
import { cliDiagnostics, command, readBounded } from "./runner";
import { escapeHtml } from "./report";
import { mkdtemp, writeFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { execFileSync } from "node:child_process";

describe("bounded file observations", () => {
  it.skipIf(process.platform === "win32")(
    "refuses a FIFO without waiting for a writer",
    async () => {
      const dir = await mkdtemp(join(tmpdir(), "svg-assertion-fifo-test-"));
      try {
        const fifo = join(dir, "input.pipe");
        execFileSync("mkfifo", [fifo]);
        await expect(readBounded(fifo, 100)).rejects.toThrow("file bound");
      } finally {
        await rm(dir, { recursive: true });
      }
    },
    2000
  );
  it("reads exact bytes but refuses oversized files and non-files before allocation", async () => {
    const dir = await mkdtemp(join(tmpdir(), "svg-assertion-file-test-"));
    try {
      const file = join(dir, "input.bin");
      await writeFile(file, Buffer.from([1, 2, 3, 4]));
      expect([...(await readBounded(file, 4))]).toEqual([1, 2, 3, 4]);
      await expect(readBounded(file, 3)).rejects.toThrow("file bound");
      await expect(readBounded(dir, 100)).rejects.toThrow("file bound");
    } finally {
      await rm(dir, { recursive: true });
    }
  });
});

describe("bounded process observations", () => {
  it("records a missing executable instead of fabricating a refusal", async () => {
    const r = await command("/nonexistent/svg-assertions-executable", []);
    expect(r.error).not.toBeNull();
    expect(r.exit).not.toBe(0);
  });
  it("kills a timed-out renderer", async () => {
    const r = await command(
      process.execPath,
      ["-e", "setInterval(()=>{},1000)"],
      50
    );
    expect(r.error).toBe("execution-timeout");
    expect(r.exit).not.toBe(0);
  });
  it("retains stdout, stderr, exit status and crash signal separately", async () => {
    const r = await command(process.execPath, [
      "-e",
      "process.stdout.write('out');process.stderr.write('err');process.exit(7)",
    ]);
    expect(r.stdout).toBe("out");
    expect(r.stderr).toBe("err");
    expect(r.exit).toBe(7);
    expect(r.error).toBeNull();
  });
});
describe("diagnostic and display safety", () => {
  it("does not hide a degraded footer when the declaration lines disappeared", () => {
    const r = {
      command: [],
      exit: 0,
      signal: null,
      error: null,
      stdout: "",
      stderr:
        "rendered input.svg -> output.png (64x64, base-shared-frame, 1 degraded, 137 bytes)\n",
    };
    expect(
      cliDiagnostics(
        r,
        "input.svg",
        "output.png",
        { width: 64, height: 64 },
        137
      )
    ).toContain("1 degraded");
  });
  it("removes only the exact successful CLI footer", () => {
    const r = {
      command: [],
      exit: 0,
      signal: null,
      error: null,
      stdout: "",
      stderr:
        "degraded: skipped svg/circle[1]: unsupported unit\nrendered input.svg -> output.png (64x64, base-shared-frame, 1 degraded, 137 bytes)\n",
    };
    expect(
      cliDiagnostics(
        r,
        "input.svg",
        "output.png",
        { width: 64, height: 64 },
        137
      )
    ).toBe("degraded: skipped svg/circle[1]: unsupported unit");
    expect(
      cliDiagnostics(
        r,
        "different.svg",
        "output.png",
        { width: 64, height: 64 },
        137
      )
    ).toContain("rendered input.svg");
    expect(
      cliDiagnostics(
        { ...r, exit: 1 },
        "input.svg",
        "output.png",
        { width: 64, height: 64 },
        137
      )
    ).toContain("rendered input.svg");
  });
  it("escapes untrusted descriptions and renderer messages", () =>
    expect(escapeHtml('<img src=x onerror="alert(1)">&')).toBe(
      "&lt;img src=x onerror=&quot;alert(1)&quot;&gt;&amp;"
    ));
});
