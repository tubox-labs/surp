import { createFileRoute } from "@tanstack/react-router";
import SRC from "@/content/RUST_API.md?raw";
import { DocPage } from "@/components/DocPage";

export const Route = createFileRoute("/docs/rust-api")({
  head: () => ({
    meta: [
      { title: "Rust API Reference — Surp" },
      { name: "description", content: "The public Rust API: Value, SurpValue, Encoder, Decoder, derive macros, text notation, and RFC-001 helpers." },
      { property: "og:title", content: "Rust API — Surp" },
      { property: "og:description", content: "Generated from docs/RUST_API.md in the Surp repository." },
      { property: "og:url", content: "/docs/rust-api" },
    ],
    links: [{ rel: "canonical", href: "/docs/rust-api" }],
  }),
  component: () => (
    <DocPage source={SRC} title="Rust API" description="Encoder, Decoder, Value, SurpValue, and the derive macros — all surfaced from docs/RUST_API.md."
      crumbs={[{ to: "/docs", label: "Docs" }, { label: "Rust API" }]}
      edit="https://github.com/tubox-labs/surp/blob/main/docs/RUST_API.md" />
  ),
});
