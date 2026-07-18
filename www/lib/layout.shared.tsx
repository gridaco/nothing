import type { BaseLayoutProps } from "fumadocs-ui/layouts/shared";
import { Brand } from "@/components/brand";
import { gitConfig } from "./shared";

export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: <Brand />,
    },
    links: [
      {
        text: "Docs",
        url: "/docs",
        active: "nested-url",
      },
    ],
    githubUrl: `https://github.com/${gitConfig.user}/${gitConfig.repo}`,
  };
}
