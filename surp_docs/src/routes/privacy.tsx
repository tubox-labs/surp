import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/privacy")({
  head: () => ({
    meta: [
      { title: "Privacy Policy — Surp Docs" },
      { name: "description", content: "How this documentation site handles data. No analytics, no third-party tracking, no cookies beyond a single browser storage key." },
      { property: "og:title", content: "Privacy Policy — Surp Docs" },
      { property: "og:url", content: "/privacy" },
      { name: "robots", content: "index, follow" },
    ],
    links: [{ rel: "canonical", href: "/privacy" }],
  }),
  component: () => (
    <Legal
      title="Privacy Policy"
      kicker="Effective immediately. Updated alongside this documentation."
      body={[
        ["What this site collects", "This documentation site does not run analytics, does not embed third-party trackers, and does not log requests beyond what the static host needs to serve the page. There are no marketing pixels, no session replay tools, and no advertising SDKs."],
        ["Browser storage", "The site stores two small keys in your browser's localStorage: surp-theme (your light/dark preference) and surp-cookie-consent (your acknowledgement of the consent banner). Both are local to your device and never transmitted."],
        ["External links", "Pages link out to GitHub, crates.io, PyPI, and other open-source ecosystems. Once you follow those links, the destination's own policies apply."],
        ["Search", "The search index is built at compile time from the project's own Markdown files and is shipped with the JavaScript bundle. Search queries are evaluated entirely in your browser and never sent to a server."],
        ["Children", "This site is not directed at children under 13 and does not knowingly collect any personal data."],
        ["Changes", "If the way this site handles data changes — for example, if analytics are ever added — this page will be updated and the consent banner will reappear."],
        ["Contact", "For anything privacy-related about the Surp project itself, open an issue on the GitHub repository."],
      ]}
    />
  ),
});

export function Legal({ title, kicker, body }: { title: string; kicker: string; body: [string, string][] }) {
  return (
    <div className="mx-auto max-w-[760px] px-5 sm:px-8 py-16">
      <div className="eyebrow mb-3">Legal</div>
      <h1 className="display-lg text-ink">{title}</h1>
      <p className="mt-3 text-muted text-[13.5px]">{kicker}</p>
      <div className="ink-rule my-10" />
      <div className="prose max-w-none">
        {body.map(([h, p]) => (
          <section key={h}>
            <h2>{h}</h2>
            <p>{p}</p>
          </section>
        ))}
      </div>
    </div>
  );
}
