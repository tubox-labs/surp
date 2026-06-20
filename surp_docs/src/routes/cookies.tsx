import { createFileRoute } from "@tanstack/react-router";
import { Legal } from "./privacy";

export const Route = createFileRoute("/cookies")({
  head: () => ({
    meta: [
      { title: "Cookie Policy — Surp Docs" },
      { name: "description", content: "What this site stores in your browser. No tracking cookies, no third parties — just your theme preference and your consent acknowledgement." },
      { property: "og:title", content: "Cookie Policy — Surp Docs" },
      { property: "og:url", content: "/cookies" },
    ],
    links: [{ rel: "canonical", href: "/cookies" }],
  }),
  component: () => (
    <Legal
      title="Cookie Policy"
      kicker="Two keys. Both local. Both yours."
      body={[
        ["What we store", "This site uses no HTTP cookies. It stores two small values in your browser's localStorage: surp-theme records whether you prefer the light or dark theme; surp-cookie-consent records that you've seen the consent banner."],
        ["Why we ask", "Even though those values never leave your device, some regulations treat any in-browser persistence as worth a notice. The banner you see on first visit is that notice."],
        ["How to clear them", "Open your browser's storage controls for this site and remove both keys. The banner will reappear on your next visit and the theme will revert to your system preference."],
        ["Third parties", "There are none. The site loads two web-font stylesheets from Google Fonts and the project repository links to GitHub; those services have their own policies."],
        ["Changes", "If this site ever introduces additional storage (for example, an analytics tool), this page and the banner will be updated to disclose it."],
      ]}
    />
  ),
});
