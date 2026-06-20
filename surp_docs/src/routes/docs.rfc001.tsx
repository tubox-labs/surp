import { createFileRoute } from "@tanstack/react-router";
import SRC from "@/content/RFC-001-IMPLEMENTATION.md?raw";
import { DocPage } from "@/components/DocPage";

export const Route = createFileRoute("/docs/rfc001")({
  head: () => ({
    meta: [
      { title: "RFC-001: CTN, CBF, and CQL — Surp" },
      { name: "description", content: "The additive next-generation implementation: the CTN text notation, the CBF binary format, and the baseline CQL path query engine." },
      { property: "og:title", content: "RFC-001 — Surp" },
      { property: "og:description", content: "Generated from docs/RFC-001-IMPLEMENTATION.md." },
      { property: "og:url", content: "/docs/rfc001" },
    ],
    links: [{ rel: "canonical", href: "/docs/rfc001" }],
  }),
  component: () => (
    <DocPage source={SRC} title="RFC-001 implementation" description="CTN · CBF · CQL — an additive, namespaced next-generation path. Not the v1 wire format."
      crumbs={[{ to: "/docs", label: "Docs" }, { label: "RFC-001" }]}
      edit="https://github.com/tubox-labs/surp/blob/main/docs/RFC-001-IMPLEMENTATION.md" />
  ),
});
