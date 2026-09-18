import { describe, expect, it } from "vitest";
import { executableArch } from "./capture-identity";

describe("actual browser executable architecture", () => {
  it.each([[0x0100000c, "arm64"], [0x01000007, "x64"]] as const)("reads Mach-O CPU %i", (cpu, arch) => {
    const header = Buffer.alloc(20);
    header.writeUInt32LE(0xfeedfacf); header.writeUInt32LE(cpu, 4);
    expect(executableArch(header)).toBe(arch);
  });
  it.each([[183, "arm64"], [62, "x64"]] as const)("reads ELF CPU %i", (cpu, arch) => {
    const header = Buffer.from([127, 69, 76, 70, 2, 1, ...Array(14).fill(0)]);
    header.writeUInt16LE(cpu, 18);
    expect(executableArch(header)).toBe(arch);
    header[5] = 2;
    expect(() => executableArch(header)).toThrow();
  });
  it("refuses unknown, truncated and non-executable identities", () => {
    for (const bytes of [Buffer.alloc(0), Buffer.alloc(19), Buffer.alloc(20)])
      expect(() => executableArch(bytes)).toThrow();
  });
});
