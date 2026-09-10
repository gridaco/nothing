/** Bounded by the parent process. No alternate Chromium capture posture. */
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import {
  captureFirstSvg,
  deterministicContext,
  launchDeterministicChromium,
} from "../../../fixtures/web-first/chromium_capture";

async function main(): Promise<void> {
  const [source, out, width, height, version] = process.argv.slice(2);
  const bytes = await readFile(source);
  const browser = await launchDeterministicChromium();
  try {
    if (browser.version() !== version)
      throw new Error(
        `Chromium version drift: ${browser.version()} != ${version}`
      );
    const context = await deterministicContext(browser);
    try {
      const page = await context.newPage();
      page.setDefaultTimeout(15000);
      page.setDefaultNavigationTimeout(15000);
      const failures: string[] = [];
      page.on("requestfailed", (request) =>
        failures.push(`${request.url()}: ${request.failure()?.errorText}`)
      );
      for (let i = 0; i < 2; i++) {
        const png = await captureFirstSvg(page, {
          source: bytes,
          width: Number(width),
          height: Number(height),
          media: "image/svg+xml",
          label: "assertion input",
        });
        await writeFile(join(out, `chromium-${i}.png`), png, { flag: "wx" });
      }
      if (failures.length)
        throw new Error(`resource requests failed: ${failures.join("; ")}`);
      await page.close();
    } finally {
      await context.close();
    }
  } finally {
    await browser.close();
  }
}
main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
