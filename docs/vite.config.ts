import { readdirSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import react from "@vitejs/plugin-react";
import mdx from "fumadocs-mdx/vite";
import { nitro } from "nitro/vite";
import { defineConfig } from "vite";

const docsContentDir = fileURLToPath(
  new URL("./content/docs", import.meta.url),
);

function getDocsStaticPages() {
  const pages = new Set<string>();

  function visit(dir: string) {
    for (const entry of readdirSync(dir)) {
      const path = join(dir, entry);
      const stat = statSync(path);

      if (stat.isDirectory()) {
        visit(path);
        continue;
      }

      if (!entry.endsWith(".mdx")) continue;

      const segments = relative(docsContentDir, path)
        .split(sep)
        .map((segment) => segment.replace(/\.mdx$/, ""));

      if (segments.at(-1) === "index") segments.pop();

      const pagePath = segments.length === 0 ? "/" : `/${segments.join("/")}`;
      const markdownPath =
        segments.length === 0 ? "/index.md" : `/${segments.join("/")}.md`;

      pages.add(pagePath);
      pages.add(markdownPath);
    }
  }

  visit(docsContentDir);

  return Array.from(pages)
    .sort((a, b) => a.localeCompare(b))
    .map((path) => ({ path }));
}

const staticPages = [
  ...getDocsStaticPages(),
  { path: "/api/search" },
  { path: "/llms-full.txt" },
  { path: "/llms.txt" },
];

export default defineConfig({
  server: {
    port: 3000,
  },
  plugins: [
    mdx(),
    tailwindcss(),
    tanstackStart({
      prerender: {
        enabled: true,
        crawlLinks: true,
        failOnError: true,
      },
      pages: staticPages,
    }),
    react(),
    nitro(),
  ],
  resolve: {
    tsconfigPaths: true,
    alias: {},
  },
});
