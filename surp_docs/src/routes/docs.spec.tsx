import { createFileRoute } from "@tanstack/react-router";
import SRC from "@/content/SPEC.md?raw";
import { DocPage } from "@/components/DocPage";

export const Route = createFileRoute("/docs/spec")({
  head: () => ({
    meta: [
      { title: "v1 Binary Format Specification — Surp" },
      { name: "description", content: "The Surp v1 wire format: block framing, header layout, type IDs, varint encoding, checksums, and resource limits." },
      { property: "og:title", content: "v1 Spec — Surp" },
      { property: "og:description", content: "Generated from docs/SPEC.md in the Surp repository." },
      { property: "og:url", content: "/docs/spec" },
    ],
    links: [{ rel: "canonical", href: "/docs/spec" }],
  }),
  component: () => (
    <DocPage source={SRC} title="v1 Binary Format" description="The stable compatibility surface. Blocks, types, varints, checksums."
      crumbs={[{ to: "/docs", label: "Docs" }, { label: "v1 Spec" }]}
      edit="https://github.com/tubox-labs/surp/blob/main/docs/SPEC.md" />
  ),
});
