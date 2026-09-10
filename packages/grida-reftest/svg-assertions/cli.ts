import { resolve } from "node:path";
import { run, type Options } from "./runner";

async function main(): Promise<void> {
  const argv = process.argv.slice(2),
    values = new Map<string, string>();
  let observe = false;
  while (argv.length) {
    const flag = argv.shift()!;
    if (flag === "--observe" && !observe) {
      observe = true;
      continue;
    }
    if (
      ![
        "--manifest",
        "--out",
        "--resvg",
        "--resvg-sha256",
        "--resvg-version",
      ].includes(flag) ||
      values.has(flag) ||
      !argv.length
    )
      throw new Error(`unknown, duplicate, or incomplete option: ${flag}`);
    values.set(flag, argv.shift()!);
  }
  if (!values.get("--manifest") || !values.get("--out"))
    throw new Error(
      "usage: cli.ts --manifest FILE --out NEW_DIRECTORY [--observe] [--resvg FILE --resvg-sha256 HASH --resvg-version TEXT]"
    );
  const options: Options = {
    manifest: resolve(values.get("--manifest")!),
    out: resolve(values.get("--out")!),
  };
  if (
    ["--resvg", "--resvg-sha256", "--resvg-version"].some((key) =>
      values.has(key)
    )
  ) {
    const executable = values.get("--resvg"),
      sha256 = values.get("--resvg-sha256"),
      version = values.get("--resvg-version");
    if (!executable || !sha256 || !/^[a-f0-9]{64}$/.test(sha256) || !version)
      throw new Error(
        "fresh resvg requires an executable, sha256 and exact version text"
      );
    options.resvg = { executable, sha256, version };
  }
  const report = await run(options);
  for (const r of report.cases)
    console.log(
      `${r.verdict.status} ${r.case.id} [${r.verdict.kind}]: ${r.verdict.reasons.join("; ")}`
    );
  console.log(`Observations: ${options.out}/index.html`);
  if (observe) {
    console.log(
      "OBSERVE ONLY: this invocation does not certify a required gate."
    );
    if (report.integrity.length) process.exitCode = 1;
  } else if (!report.gate_ready) process.exitCode = 1;
}
main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
