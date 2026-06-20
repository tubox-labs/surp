import { createFileRoute } from "@tanstack/react-router";
import SRC from "@/content/EXAMPLES.md?raw";
import { DocPage } from "@/components/DocPage";

export const Route = createFileRoute("/examples")({
  head: () => ({
    meta: [
      { title: "Examples — Surp" },
      { name: "description", content: "Worked examples drawn from the examples/ directory of the Surp repository." },
      { property: "og:title", content: "Examples — Surp" },
      { property: "og:url", content: "/examples" },
    ],
    links: [{ rel: "canonical", href: "/examples" }],
  }),
  component: () => (
    <DocPage source={SRC} title="Examples" description="Tiny, faithful examples drawn directly from the examples/ tree."
      crumbs={[{ label: "Examples" }]}
      edit="https://github.com/tubox-labs/surp/blob/main/docs/EXAMPLES.md" />
  ),
});
