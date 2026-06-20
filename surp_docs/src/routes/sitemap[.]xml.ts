import { createFileRoute } from "@tanstack/react-router";
import type {} from "@tanstack/react-start";

const BASE_URL = "";

const ROUTES = [
  ["/", "1.0", "weekly"],
  ["/getting-started", "0.9", "monthly"],
  ["/docs", "0.9", "monthly"],
  ["/docs/spec", "0.9", "monthly"],
  ["/docs/rfc001", "0.8", "monthly"],
  ["/docs/rust-api", "0.9", "monthly"],
  ["/docs/python-api", "0.9", "monthly"],
  ["/docs/cli", "0.8", "monthly"],
  ["/docs/mcp", "0.6", "monthly"],
  ["/architecture", "0.9", "monthly"],
  ["/examples", "0.8", "monthly"],
  ["/changelog", "0.7", "weekly"],
  ["/community", "0.6", "monthly"],
  ["/help", "0.6", "monthly"],
  ["/security", "0.6", "monthly"],
  ["/privacy", "0.3", "yearly"],
  ["/terms", "0.3", "yearly"],
  ["/cookies", "0.3", "yearly"],
] as const;

export const Route = createFileRoute("/sitemap.xml")({
  server: {
    handlers: {
      GET: async ({ request }) => {
        const urlObj = new URL(request.url);
        const baseUrl = process.env.SITE_URL || `${urlObj.protocol}//${urlObj.host}`;
        const urls = ROUTES.map(([p, pr, cf]) =>
          `  <url>\n    <loc>${baseUrl}${p}</loc>\n    <changefreq>${cf}</changefreq>\n    <priority>${pr}</priority>\n  </url>`
        ).join("\n");
        const xml = `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${urls}\n</urlset>`;
        return new Response(xml, {
          headers: { "Content-Type": "application/xml", "Cache-Control": "public, max-age=3600" },
        });
      },
    },
  },
});
