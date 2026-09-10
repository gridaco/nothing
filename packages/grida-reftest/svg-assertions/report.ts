import type { Report } from "./runner";

export const escapeHtml = (value: unknown): string =>
  String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");

/** Static local viewer. No scripts, remote assets, winner selection or scores. */
export function renderReport(report: Report): string {
  const e = escapeHtml;
  const labels: Record<string, string> = {
    baked: "Baked Chromium reference",
    chromium: "Fresh Chromium",
    strict: "n0 strict",
    best: "n0 best effort",
    stored: "Upstream stored PNG",
    resvg: "Fresh resvg",
  };
  return `<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src 'self'; style-src 'unsafe-inline'"><title>SVG assertion observations</title>
<style>body{font:16px system-ui;margin:2rem;color:#16202c;background:#f5f6f8}h1{margin-bottom:.4rem}section{margin:2rem 0;padding:1.5rem;background:white;border:1px solid #bbc5d0;border-radius:8px}.images{display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:1rem}figure{margin:0;min-width:0}img{width:100%;height:auto;background:repeating-conic-gradient(#ddd 0% 25%,#fff 0% 50%) 0/16px 16px;border:1px solid #aaa}pre{white-space:pre-wrap;overflow-wrap:anywhere;font-size:12px}.PASS{border-left:6px solid #168050}.FAIL{border-left:6px solid #c22535}.UNRESOLVED,.OBSERVATION{border-left:6px solid #a26b00}small{color:#42556b}.warning{font-weight:600}summary{cursor:pointer}a{color:#164faa}</style>
<h1>SVG assertion observations</h1><p>One described claim, one discrete verdict. Image differences are evidence, not grades.</p>
<p class="warning">${report.gate_ready ? "Required assertions satisfied." : "Required assertion gate is NOT ready."} This is not an SVG support or conformance result.</p>
<p>Profile: ${e(report.manifest.profile)} · Chromium ${e(report.manifest.capture.browser_version)} · <a href="report.json">Raw observations</a></p>
<details><summary>Run provenance: versions, hashes and build identity</summary><pre>${e(JSON.stringify(report.tools, null, 2))}</pre></details>
${report.integrity.length ? `<pre>${e(report.integrity.join("\n"))}</pre>` : ""}
${report.cases
  .map(
    (
      r
    ) => `<section class="${e(r.verdict.status)}"><h2>${e(r.case.id)}</h2><p>${e(r.case.description ?? "Undescribed upstream observation")}</p>
<p><strong>${e(r.verdict.status)} · ${r.verdict.kind === "refusal" ? "expected refusal assertion — NOT rendering support" : e(r.verdict.kind)}</strong> · ${r.case.required ? "required" : "exploratory"}</p>
<p>${e(r.case.assertion?.decision ?? "No reference decision")}</p><pre>${e(r.verdict.reasons.join("\n"))}</pre>
<div class="images">${["baked", "chromium", "strict", "best", "stored", "resvg"]
      .map((name) => {
        const o = r.observations[name],
          sample = o.samples[0],
          image = sample?.image;
        return `<figure><figcaption><strong>${e(labels[name])}</strong></figcaption>${image ? `<img loading="lazy" alt="${e(labels[name])} output for ${e(r.case.id)}" src="${e(image.path)}">` : "<p>No image</p>"}<pre>${e(o.problem ?? sample?.diagnostics ?? "")}</pre></figure>`;
      })
      .join(
        ""
      )}</div><details><summary>Exact comparisons, diagnostics and both repeats</summary><pre>${e(JSON.stringify({ pairs: r.pairs, observations: r.observations }, null, 2))}</pre></details></section>`
  )
  .join("\n")}</html>`;
}
