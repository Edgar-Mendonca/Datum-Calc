import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

/** KaTeX and the Plex stylesheets list woff2, woff (and ttf) fonts; WebView2 only needs woff2, so the others are left out of the build. */
const katexWoff2 = { name: "katex-woff2", enforce: "pre" as const, transform: (code: string, id: string) => (id.includes("katex") || id.includes("@fontsource")) && id.endsWith(".css") ? code.replace(/,\s*url\([^)]*\.(woff|ttf)\)\s*format\(["'](woff|truetype)["']\)/g, "") : null };

export default defineConfig({
  plugins: [solid(), katexWoff2],
  base: "./",
  build: { outDir: "dist", emptyOutDir: true, target: "es2022", chunkSizeWarningLimit: 900 },
});
