import { createFileRoute } from "@tanstack/react-router";
import SRC from "@/content/MCP.md?raw";
import { DocPage } from "@/components/DocPage";

export const Route = createFileRoute("/docs/mcp")({
  head: () => ({
    meta: [
      { title: "MCP Server — Surp" },
      { name: "description", content: "The Surp MCP (Model Context Protocol) surface, as described in the repository." },
      { property: "og:title", content: "MCP — Surp" },
      { property: "og:url", content: "/docs/mcp" },
    ],
    links: [{ rel: "canonical", href: "/docs/mcp" }],
  }),
  component: () => (
    <DocPage source={SRC} title="MCP server" description="Documented from docs/MCP.md in the repository."
      crumbs={[{ to: "/docs", label: "Docs" }, { label: "MCP" }]}
      edit="https://github.com/tubox-labs/surp/blob/main/docs/MCP.md" />
  ),
});
