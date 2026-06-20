import { createFileRoute } from "@tanstack/react-router";
import SRC from "@/content/PYTHON_API.md?raw";
import { DocPage } from "@/components/DocPage";

export const Route = createFileRoute("/docs/python-api")({
  head: () => ({
    meta: [
      { title: "Python API Reference — Surp" },
      { name: "description", content: "The native Python package: dumps/loads, Encoder, SurpDecoder, SurpValue views, surp.rfc001 and surp.model." },
      { property: "og:title", content: "Python API — Surp" },
      { property: "og:description", content: "Generated from docs/PYTHON_API.md in the Surp repository." },
      { property: "og:url", content: "/docs/python-api" },
    ],
    links: [{ rel: "canonical", href: "/docs/python-api" }],
  }),
  component: () => (
    <DocPage source={SRC} title="Python API" description="A PyO3 module called surp. From dumps/loads to the typed SurpModel layer."
      crumbs={[{ to: "/docs", label: "Docs" }, { label: "Python API" }]}
      edit="https://github.com/tubox-labs/surp/blob/main/docs/PYTHON_API.md" />
  ),
});
