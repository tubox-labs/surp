import { createFileRoute } from "@tanstack/react-router";
import { Legal } from "./privacy";

export const Route = createFileRoute("/terms")({
  head: () => ({
    meta: [
      { title: "Terms — Surp Docs" },
      { name: "description", content: "Terms governing your use of this documentation site, and the licensing of the underlying Surp project." },
      { property: "og:title", content: "Terms — Surp Docs" },
      { property: "og:url", content: "/terms" },
    ],
    links: [{ rel: "canonical", href: "/terms" }],
  }),
  component: () => (
    <Legal
      title="Terms & Conditions"
      kicker="Plain language. Read this once, refer back if anything changes."
      body={[
        ["The Surp project", "The Surp source code is released under your choice of the MIT License or the Apache License 2.0. The full text of both licenses lives in the repository as LICENSE-MIT and LICENSE-APACHE. Those license terms — not these site terms — govern your use of the Surp software."],
        ["This documentation site", "The pages, diagrams, and prose on this site are published for your benefit as a reference for Surp. You may read, share, and quote them freely with attribution. The site is provided as-is, without warranty of any kind."],
        ["Trademarks", "\"Surp\" and the Tubox Labs name belong to their respective holders. Their appearance on this site does not grant any trademark rights."],
        ["No legal advice", "Nothing on this site is legal, security, or compliance advice. Always confirm a behavior against the source code and the v1 specification before relying on it in production."],
        ["Disclaimer of liability", "To the maximum extent permitted by law, the maintainers and contributors are not liable for any damages arising out of your use of this documentation or the Surp software."],
        ["Changes", "These terms may be revised. Material changes will be reflected here and dated in the changelog."],
      ]}
    />
  ),
});
