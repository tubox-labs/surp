import { createFileRoute } from "@tanstack/react-router";
import SEC from "@/content/SECURITY.md?raw";
import RISK from "@/content/DESIGN_RISKS.md?raw";
import { DocPage } from "@/components/DocPage";

const SRC = `# Security & Design Risks

This page combines the project's \`SECURITY.md\` and \`DESIGN_RISKS.md\` documents,
side by side, so security reporting and known design tradeoffs are reachable
from a single URL.

---

${SEC}

---

${RISK}
`;

export const Route = createFileRoute("/security")({
  head: () => ({
    meta: [
      { title: "Security & Design Risks — Surp" },
      { name: "description", content: "How to report a vulnerability in Surp, plus the project's catalog of known design tradeoffs and risk areas." },
      { property: "og:title", content: "Security & Risks — Surp" },
      { property: "og:url", content: "/security" },
    ],
    links: [{ rel: "canonical", href: "/security" }],
  }),
  component: () => (
    <DocPage source={SRC} title="Security & design risks" description="Reporting policy and the project's own honest list of design tradeoffs."
      crumbs={[{ label: "Security" }]}
      edit="https://github.com/tubox-labs/surp/blob/main/SECURITY.md" />
  ),
});
