import { docs, workingGroup } from "collections/server";
import { loader } from "fumadocs-core/source";
import { lucideIconsPlugin } from "fumadocs-core/source/lucide-icons";
import { posix } from "node:path";
import {
  docsContentRoute,
  docsImageRoute,
  docsRoute,
  gitConfig,
} from "./shared";

// See https://fumadocs.dev/docs/headless/source-api for more info
export const source = loader({
  baseUrl: docsRoute,
  source: {
    root: docs.toFumadocsSource(),
    workingGroup: workingGroup.toFumadocsSource({ baseDir: "wg" }),
  },
  plugins: [lucideIconsPlugin()],
});

export function getPageGitHubPath(page: (typeof source)["$inferPage"]) {
  if (page.type === "workingGroup") {
    return `docs/${page.path}`;
  }

  return `www/content/docs/${page.path}`;
}

export function resolvePageHref(
  page: (typeof source)["$inferPage"],
  href: string
) {
  const resolved = source.resolveHref(href, page);
  if (resolved !== href || !/^\.{1,2}\//.test(href)) return resolved;

  const hashIndex = href.indexOf("#");
  const hash = hashIndex === -1 ? "" : href.slice(hashIndex);
  const pathWithQuery = hashIndex === -1 ? href : href.slice(0, hashIndex);
  const queryIndex = pathWithQuery.indexOf("?");
  const query = queryIndex === -1 ? "" : pathWithQuery.slice(queryIndex);
  const targetPath =
    queryIndex === -1 ? pathWithQuery : pathWithQuery.slice(0, queryIndex);
  const suffix = `${query}${hash}`;

  const candidates: string[] = [];
  if (targetPath.endsWith("/")) {
    candidates.push(`${targetPath}index.md`);
  } else if (posix.extname(targetPath) === "") {
    candidates.push(`${targetPath}.md`, `${targetPath}/index.md`);
  }

  for (const candidate of candidates) {
    const candidateHref = `${candidate}${suffix}`;
    const candidateResolved = source.resolveHref(candidateHref, page);
    if (candidateResolved !== candidateHref) return candidateResolved;
  }

  if (page.type !== "workingGroup") return href;

  const repoPath = posix.normalize(
    posix.join(posix.dirname(getPageGitHubPath(page)), targetPath)
  );
  if (repoPath === ".." || repoPath.startsWith("../")) return href;

  const kind =
    targetPath.endsWith("/") || posix.extname(targetPath) === ""
      ? "tree"
      : "blob";
  const encodedPath = repoPath
    .split("/")
    .map((segment) => encodeURIComponent(segment))
    .join("/");

  return `https://github.com/${gitConfig.user}/${gitConfig.repo}/${kind}/${gitConfig.branch}/${encodedPath}${suffix}`;
}

export function getPageImage(page: (typeof source)["$inferPage"]) {
  const segments = [...page.slugs, "image.png"];

  return {
    segments,
    url: `${docsImageRoute}/${segments.join("/")}`,
  };
}

export function getPageMarkdownUrl(page: (typeof source)["$inferPage"]) {
  const segments = [...page.slugs, "content.md"];

  return {
    segments,
    url: `${docsContentRoute}/${segments.join("/")}`,
  };
}

export async function getLLMText(page: (typeof source)["$inferPage"]) {
  const processed = await page.data.getText("processed");

  return `# ${page.data.title} (${page.url})

${processed}`;
}
