import { createFileRoute, Link } from "@tanstack/react-router";
import { I } from "@/lib/icons";
import { Sidebar, docsNav } from "@/components/Sidebar";

export const Route = createFileRoute("/docs/")({
  head: () => ({
    meta: [
      { title: "Documentation — Surp" },
      { name: "description", content: "Surp documentation: getting started, architecture, the v1 binary format, RFC-001 CTN/CBF/CQL, Rust and Python APIs, and the CLI reference." },
      { property: "og:title", content: "Documentation — Surp" },
      { property: "og:description", content: "Browse every documented surface of Surp." },
      { property: "og:url", content: "/docs" },
    ],
    links: [{ rel: "canonical", href: "/docs" }],
  }),
  component: DocsIndex,
});

function DocsIndex() {
  return (
    <div className="mx-auto max-w-[1280px] px-5 sm:px-8 py-14 lg:grid lg:grid-cols-[220px_minmax(0,1fr)] lg:gap-12">
      <aside className="hidden lg:block sticky top-24 self-start">
        <Sidebar active="/docs" />
      </aside>
      <div className="min-w-0">
        <div className="eyebrow mb-3">Documentation</div>
        <h1 className="display-lg text-ink max-w-2xl">Everything Surp does, written from the source.</h1>
        <p className="mt-4 text-body max-w-2xl text-[17px]">
          Each page in this site is generated from the repository's own Markdown
          (<code className="font-mono text-[13px]">README.md</code>, <code className="font-mono text-[13px]">docs/*.md</code>, <code className="font-mono text-[13px]">SECURITY.md</code>, <code className="font-mono text-[13px]">CHANGELOG.md</code>) and reads the verified
          API names and CLI flags directly. Nothing in here is invented.
        </p>
        <div className="ink-rule my-10" />
        <div className="grid gap-12">
          {docsNav.map((g) => (
            <section key={g.label}>
              <div className="eyebrow text-primary mb-4">{g.label}</div>
              <div className="grid sm:grid-cols-2 gap-px bg-hairline border border-hairline rounded-xl overflow-hidden">
                {g.items.map((it) => (
                  <Link key={it.to} to={it.to} className="group bg-surface p-6 hover:bg-canvas-soft transition-colors flex items-start justify-between gap-4">
                    <div>
                      <div className="text-ink text-[17px] font-medium">{it.label}</div>
                      <p className="mt-1.5 text-[13.5px] text-muted">{describe(it.to)}</p>
                    </div>
                    <I.ArrowUpRight className="text-muted-soft group-hover:text-primary shrink-0 mt-1" />
                  </Link>
                ))}
              </div>
            </section>
          ))}
        </div>
      </div>
    </div>
  );
}

function describe(to: string): string {
  return ({
    "/getting-started": "Install, build, and run your first round-trip.",
    "/docs": "This page.",
    "/examples": "Worked examples from the repo's examples/ tree.",
    "/architecture": "Hand-drawn diagrams of the workspace and codec.",
    "/docs/spec": "The block-framed v1 binary format, byte by byte.",
    "/docs/rfc001": "CTN, CBF, and the baseline CQL path engine.",
    "/docs/rust-api": "Encoder, Decoder, Value, SurpValue, derive macros.",
    "/docs/python-api": "dumps/loads, Encoder, SurpDecoder, surp.model.",
    "/docs/cli": "Every surp subcommand and flag.",
    "/docs/mcp": "Model Context Protocol server surface.",
    "/changelog": "Semantic-versioned release history.",
    "/community": "Where conversations and contributions happen.",
    "/help": "Common questions and troubleshooting.",
    "/security": "SECURITY.md and DESIGN_RISKS.md, side by side.",
  })[to] ?? "";
}
