import { createFileRoute } from "@tanstack/react-router";
import SRC from "@/content/CHANGELOG.md?raw";
import { DocPage } from "@/components/DocPage";

export const Route = createFileRoute("/changelog")({
  head: () => ({
    meta: [
      { title: "Changelog — Surp" },
      { name: "description", content: "Semver-aligned release history for Surp. Generated from CHANGELOG.md." },
      { property: "og:title", content: "Changelog — Surp" },
      { property: "og:url", content: "/changelog" },
    ],
    links: [{ rel: "canonical", href: "/changelog" }],
  }),
  component: () => (
    <DocPage source={SRC} title="Changelog" description="Every release, every line. Straight from CHANGELOG.md."
      crumbs={[{ label: "Changelog" }]}
      edit="https://github.com/tubox-labs/surp/blob/main/CHANGELOG.md" />
  ),
});
