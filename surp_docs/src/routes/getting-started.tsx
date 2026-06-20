import { createFileRoute } from "@tanstack/react-router";
import README from "@/content/README.md?raw";
import { DocPage } from "@/components/DocPage";

// Trim README to the install / quick-start sections
const SRC = `# Getting Started

This page is generated from the project README. It walks you from a fresh
machine to your first encoded \`.surp\` file.

> **Source of truth.** Everything below comes from the repository README at
> commit time of build. If you want the absolute latest, the
> [GitHub README](https://github.com/tubox-labs/surp#installation) is
> authoritative.

${README.split("## Installation")[1]?.split("## More Documentation")[0]?.trim() || README}
`;

export const Route = createFileRoute("/getting-started")({
  head: () => ({
    meta: [
      { title: "Getting Started — Surp" },
      { name: "description", content: "Install Surp from crates.io or PyPI, build the CLI from source, and encode your first value in Rust or Python." },
      { property: "og:title", content: "Getting Started — Surp" },
      { property: "og:description", content: "Install paths, first encode, first decode, and the canonical loop." },
      { property: "og:url", content: "/getting-started" },
    ],
    links: [{ rel: "canonical", href: "/getting-started" }],
  }),
  component: () => (
    <DocPage
      source={SRC}
      title="Getting Started"
      description="From a fresh machine to your first round-trip in Rust, Python and the CLI."
      crumbs={[{ label: "Getting Started" }]}
      edit="https://github.com/tubox-labs/surp/blob/main/README.md"
    />
  ),
});
