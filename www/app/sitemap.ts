import type { MetadataRoute } from "next";
import { source } from "@/lib/source";

export default function sitemap(): MetadataRoute.Sitemap {
  const updatedAt = new Date();

  return [
    {
      url: "https://nothing.graphics",
      lastModified: updatedAt,
      changeFrequency: "weekly",
      priority: 1,
    },
    ...source.getPages().map((page) => ({
      url: `https://nothing.graphics${page.url}`,
      lastModified: updatedAt,
      changeFrequency: "weekly" as const,
      priority: page.slugs.length === 0 ? 0.9 : 0.7,
    })),
  ];
}
