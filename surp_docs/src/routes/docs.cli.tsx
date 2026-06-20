import { createFileRoute } from "@tanstack/react-router";
import SRC from "@/content/CLI.md?raw";
import { DocPage } from "@/components/DocPage";

export const Route = createFileRoute("/docs/cli")({
  head: () => ({
    meta: [
      { title: "CLI Reference — Surp" },
      { name: "description", content: "Every surp subcommand: from-json, to-json, validate, inspect, encode, decode, pretty, bench, and the RFC-001 commands." },
      { property: "og:title", content: "CLI — Surp" },
      { property: "og:description", content: "Generated from docs/CLI.md in the Surp repository." },
      { property: "og:url", content: "/docs/cli" },
    ],
    links: [{ rel: "canonical", href: "/docs/cli" }],
  }),
  component: () => (
    <DocPage source={SRC} title="CLI Reference" description="The surp command, verb by verb. Pulled from docs/CLI.md."
      crumbs={[{ to: "/docs", label: "Docs" }, { label: "CLI" }]}
      edit="https://github.com/tubox-labs/surp/blob/main/docs/CLI.md" />
  ),
});
