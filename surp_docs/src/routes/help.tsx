import { createFileRoute, Link } from "@tanstack/react-router";

export const Route = createFileRoute("/help")({
  head: () => ({
    meta: [
      { title: "Help — Surp" },
      { name: "description", content: "Common questions about installing Surp, understanding the v1 format, and the boundary between v1 and RFC-001." },
      { property: "og:title", content: "Help — Surp" },
      { property: "og:url", content: "/help" },
    ],
    links: [{ rel: "canonical", href: "/help" }],
  }),
  component: Help,
});

const FAQ = [
  {
    q: "Is the v1 format stable?",
    a: "Yes — v1 is the project's stable compatibility surface. Major version bumps are reserved for file format changes and incompatible wire semantics; minor releases are additive only.",
  },
  {
    q: "Is RFC-001 the same wire format as a .surp file?",
    a: "No. RFC-001's CBF files are not v1 .surp files. RFC-001 lives in its own namespace (surp_core::rfc001 / surp.rfc001) and uses the .crb file extension in examples.",
  },
  {
    q: "Do I need compression to use Surp?",
    a: "No. The default codec writes uncompressed payloads. Compression (zstd, lz4, snappy) is feature-gated on the Rust side and opt-in via CLI flags.",
  },
  {
    q: "Does the borrowed SurpValue work for compressed data?",
    a: "Zero-copy decode through SurpValue<'a> is available for uncompressed v1 data. Compressed payloads require an owned decode.",
  },
  {
    q: "What's the minimum supported Rust version?",
    a: "The workspace MSRV is Rust 1.85.0 and the workspace uses edition 2024.",
  },
  {
    q: "What Python versions does the package support?",
    a: "Python 3.9 or newer for the native package.",
  },
  {
    q: "How do I report a security vulnerability?",
    a: "Follow the policy in SECURITY.md. Do not file public issues for security reports.",
  },
];

function Help() {
  return (
    <div className="mx-auto max-w-[860px] px-5 sm:px-8 py-16">
      <div className="eyebrow mb-3">Help</div>
      <h1 className="display-lg text-ink">Questions that have a verified answer.</h1>
      <p className="mt-4 text-body text-[17px]">
        Each answer below is anchored in source material — the README, the spec, the
        changelog, or SECURITY.md. If your question isn't here, check the{" "}
        <Link to="/docs" className="text-ink underline underline-offset-2 decoration-hairline-strong">docs</Link> or open a{" "}
        <a href="https://github.com/tubox-labs/surp/discussions" target="_blank" rel="noopener noreferrer" className="text-ink underline underline-offset-2 decoration-hairline-strong">discussion</a>.
      </p>
      <div className="ink-rule my-10" />
      <dl className="grid gap-2">
        {FAQ.map((f, i) => (
          <details key={i} className="group border-t border-hairline py-5 [&:last-child]:border-b">
            <summary className="cursor-pointer list-none flex items-start justify-between gap-4">
              <dt className="text-ink text-[17px] font-medium pr-4">{f.q}</dt>
              <span className="mt-1 text-muted group-open:rotate-45 transition-transform select-none text-[20px] leading-none">+</span>
            </summary>
            <dd className="mt-3 text-body text-[15.5px] leading-relaxed">{f.a}</dd>
          </details>
        ))}
      </dl>
    </div>
  );
}
