// Lightweight search index derived from copied source markdown.
// At build time, Vite's ?raw imports give us the actual text.

import README from "@/content/README.md?raw";
import CLI from "@/content/CLI.md?raw";
import RUST from "@/content/RUST_API.md?raw";
import PY from "@/content/PYTHON_API.md?raw";
import SPEC from "@/content/SPEC.md?raw";
import RFC from "@/content/RFC-001-IMPLEMENTATION.md?raw";
import MCP from "@/content/MCP.md?raw";
import EX from "@/content/EXAMPLES.md?raw";
import CL from "@/content/CHANGELOG.md?raw";
import SEC from "@/content/SECURITY.md?raw";
import RISK from "@/content/DESIGN_RISKS.md?raw";

type Entry = { path: string; hash?: string; title: string; section: string; body: string };

const sources: Array<{ path: string; section: string; src: string }> = [
  { path: "/", section: "Overview", src: README },
  { path: "/getting-started", section: "Getting Started", src: README },
  { path: "/docs/cli", section: "CLI", src: CLI },
  { path: "/docs/rust-api", section: "Rust API", src: RUST },
  { path: "/docs/python-api", section: "Python API", src: PY },
  { path: "/docs/spec", section: "v1 Format", src: SPEC },
  { path: "/docs/rfc001", section: "RFC-001", src: RFC },
  { path: "/docs/mcp", section: "MCP", src: MCP },
  { path: "/examples", section: "Examples", src: EX },
  { path: "/changelog", section: "Changelog", src: CL },
  { path: "/security", section: "Security", src: SEC + "\n" + RISK },
];

const slug = (s: string) =>
  s.toLowerCase().replace(/[^a-z0-9\s-]/g, "").trim().replace(/\s+/g, "-");

function build(): Entry[] {
  const out: Entry[] = [];
  for (const s of sources) {
    // Add the page entry itself
    const firstH1 = s.src.match(/^#\s+(.+)$/m);
    out.push({
      path: s.path,
      title: firstH1 ? firstH1[1] : s.section,
      section: s.section,
      body: s.src.slice(0, 600),
    });
    // Add H2/H3 sub-entries
    const re = /^(##|###)\s+(.+)$/gm;
    let m: RegExpExecArray | null;
    while ((m = re.exec(s.src))) {
      const title = m[2].trim();
      // grab a snippet after the heading
      const start = m.index + m[0].length;
      const snippet = s.src.slice(start, start + 280).replace(/\s+/g, " ");
      out.push({
        path: s.path,
        hash: slug(title),
        title,
        section: s.section,
        body: snippet,
      });
    }
  }
  // Static pages
  const staticPages: Entry[] = [
    { path: "/architecture", title: "Architecture", section: "Architecture", body: "Workspace crates, data flow, block framing, encoder/decoder pipeline, RFC-001 boundary." },
    { path: "/community", title: "Community", section: "Project", body: "Discussions, contribution guide, code of conduct." },
    { path: "/help", title: "Help", section: "Project", body: "Common questions, troubleshooting, how to file an issue." },
    { path: "/privacy", title: "Privacy Policy", section: "Legal", body: "How this documentation site handles data." },
    { path: "/terms", title: "Terms", section: "Legal", body: "Usage terms for this documentation." },
    { path: "/cookies", title: "Cookie Policy", section: "Legal", body: "Cookies, preferences, and theme storage." },
  ];
  return [...out, ...staticPages];
}

export const searchIndex = build();
