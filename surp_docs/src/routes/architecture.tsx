import { createFileRoute, Link } from "@tanstack/react-router";
import { Sidebar } from "@/components/Sidebar";
import { ArchOverview, EncodePipeline, TrustBoundary } from "@/components/Diagrams";
import { I } from "@/lib/icons";

export const Route = createFileRoute("/architecture")({
  head: () => ({
    meta: [
      { title: "Architecture — Surp" },
      { name: "description", content: "How the Surp workspace fits together: crate layout, encode and decode pipelines, the codec/IO boundary, and the trust boundary between bytes and caller code." },
      { property: "og:title", content: "Architecture — Surp" },
      { property: "og:description", content: "Hand-drawn architecture sketches generated from the verified workspace layout." },
      { property: "og:url", content: "/architecture" },
    ],
    links: [{ rel: "canonical", href: "/architecture" }],
  }),
  component: ArchPage,
});

function ArchPage() {
  return (
    <div className="mx-auto max-w-[1280px] px-5 sm:px-8 py-12 lg:grid lg:grid-cols-[220px_minmax(0,1fr)] lg:gap-12">
      <aside className="hidden lg:block sticky top-24 self-start max-h-[calc(100vh-7rem)] overflow-y-auto pr-2">
        <Sidebar active="/architecture" />
      </aside>
      <article className="min-w-0">
        <nav className="text-[12px] text-muted mb-4 flex gap-1.5">
          <Link to="/" className="hover:text-ink">Home</Link>
          <span className="text-muted-soft">/</span>
          <span className="text-ink">Architecture</span>
        </nav>
        <header className="rise-in">
          <div className="eyebrow mb-3">Architecture</div>
          <h1 className="display-mega text-ink leading-[1.02]">A serializer drawn from <span className="italic">four neat lanes</span>.</h1>
          <p className="mt-6 text-[19px] text-body max-w-2xl leading-relaxed">
            Surp's repository is a Cargo workspace of nine crates, a Python package,
            and a small set of fixtures and benches. Underneath the surface area, one
            codec carries the load — and one wire format is the only stable contract.
          </p>
          <div className="ink-rule mt-8" />
        </header>

        <Section eyebrow="Bird's-eye view" title="The workspace at a glance">
          <p>
            The codec lives in <Mono>surp-core</Mono>. Public surfaces (<Mono>surp-cli</Mono>,
            <Mono>surp-python</Mono>, <Mono>surp-ffi</Mono>, <Mono>surp-derive</Mono>) all
            depend on it, never the other way around. IO and storage adapters
            (<Mono>surp-io</Mono>, <Mono>surp-compression</Mono>, <Mono>surp-simd</Mono>) sit
            beside the codec and are pulled in via Cargo features. The RFC-001 work lives
            entirely inside <Mono>surp_core::rfc001</Mono>, with its own namespace and its
            own file extension — never mixed with v1 <Mono>.surp</Mono> bytes.
          </p>
          <ArchOverview />
          <Note>
            Source of truth: <a href="https://github.com/tubox-labs/surp/blob/main/Cargo.toml" target="_blank" rel="noopener noreferrer" className="text-ink underline underline-offset-2 decoration-hairline-strong">Cargo.toml workspace members</a> and the "Workspace Layout" table in the README.
          </Note>
        </Section>

        <Section eyebrow="Encode pipeline" title="From a value tree to a checked file">
          <p>
            An encode never skips a stage. The encoder walks a <Mono>Value</Mono> tree
            and emits scalars with type tags and varint-encoded lengths. When string
            deduplication is on, repeated strings are interned into a dedup table that
            sits inside the same block. The block writer then prefixes the payload with
            a type byte, the payload length, a compression-type byte, and an XXH64
            checksum of the uncompressed payload. A trailer block carries the overall
            checksum; readers verify both before exposing any value.
          </p>
          <EncodePipeline />
        </Section>

        <Section eyebrow="Trust boundary" title="Where untrusted bytes become a value">
          <p>
            The decoder is the only piece of code allowed to look at raw bytes. Limits
            (max depth, max element counts, max payload sizes) are enforced before
            allocation; checksum verification fails closed; corrupt or oversized inputs
            never reach a constructed <Mono>Value</Mono>. The Rust API exposes two
            value flavors: <Mono>Value</Mono> for owned trees, and{" "}
            <Mono>SurpValue&lt;'a&gt;</Mono> for borrowed zero-copy decode of
            uncompressed v1 data.
          </p>
          <TrustBoundary />
        </Section>

        <Section eyebrow="Data ownership" title="One format, two flavors of decode">
          <div className="grid sm:grid-cols-2 gap-px bg-hairline border border-hairline rounded-xl overflow-hidden">
            {[
              {
                t: "Owned — Value",
                d: "Allocates and owns its children. Use when you want a long-lived tree, mutation, or to ship the value across thread boundaries. Always available, including for compressed payloads.",
              },
              {
                t: "Borrowed — SurpValue<'a>",
                d: "Zero-copy view tied to the original byte buffer. Available for uncompressed v1 data. Pay nothing on decode; pay only when you ask a field for an owned string or array.",
              },
            ].map((b) => (
              <div key={b.t} className="bg-surface p-6">
                <h4 className="text-ink font-medium">{b.t}</h4>
                <p className="mt-2 text-[14.5px] text-body">{b.d}</p>
              </div>
            ))}
          </div>
        </Section>

        <Section eyebrow="Subsystem notes" title="What each crate is responsible for">
          <ul className="not-prose grid gap-3">
            {SUBSYSTEMS.map((s) => (
              <li key={s.crate} className="grid sm:grid-cols-[180px_minmax(0,1fr)] gap-4 py-4 border-t border-hairline first:border-t-0">
                <span className="font-mono text-[13.5px] text-ink">{s.crate}</span>
                <span className="text-[14.5px] text-body">{s.body}</span>
              </li>
            ))}
          </ul>
          <Note>Source: the "Workspace Layout" table in <a href="https://github.com/tubox-labs/surp#workspace-layout" target="_blank" rel="noopener noreferrer" className="text-ink underline underline-offset-2 decoration-hairline-strong">the project README</a>.</Note>
        </Section>

        <Section eyebrow="Design choices" title="Why this shape, and what it doesn't try to be">
          <p>
            Three explicit tradeoffs steer the design. <strong>Safety over micro-optimization:</strong>
            checksums are verified before payloads are exposed, and resource limits sit between
            input and allocation. <strong>Determinism:</strong> the same input value produces the
            same bytes, every time — a property that makes diffing, content-addressing, and
            replay tractable. <strong>Schema evolution as a first-class feature:</strong> the
            derive macros encode stable numeric field IDs, and unknown fields are skipped on
            decode, so old readers gracefully ignore new fields.
          </p>
          <p>
            Surp is not a streaming-only format and not an in-place editable format. It is a
            canonical container for value trees with optional random access via the index block.
            Anything that looks like a database, an RPC framework, or a schema registry is out
            of scope.
          </p>
          <div className="mt-8 flex flex-wrap gap-3">
            <Link to="/docs/spec" className="inline-flex items-center gap-2 h-11 px-5 rounded-md bg-primary text-on-primary text-[14px] font-medium">
              Read the v1 spec <I.Arrow width={14} height={14} />
            </Link>
            <Link to="/docs/rfc001" className="inline-flex items-center gap-2 h-11 px-5 rounded-md border border-hairline-strong text-ink text-[14px] font-medium">
              Read RFC-001 <I.Arrow width={14} height={14} />
            </Link>
          </div>
        </Section>
      </article>
    </div>
  );
}

const SUBSYSTEMS = [
  { crate: "surp-core", body: "The codec: encoder, decoder, value tree, block framing, text notation, resource limits, and the RFC-001 modules." },
  { crate: "surp-derive", body: "#[derive(Surp)] and #[derive(SurpSchema)] for named Rust structs; stable numeric field IDs for forward-compatible schema evolution." },
  { crate: "surp-cli", body: "The surp binary tool. Verb-driven; converts JSON↔v1, encodes/decodes the text notation, inspects, validates, runs CLI benches and the RFC-001 commands." },
  { crate: "surp-python", body: "PyO3 extension that exports the Python package named surp; ships SurpValue views, Encoder/SurpDecoder, and the surp.model RFC-001 validation layer." },
  { crate: "surp-io", body: "Tokio framed IO, shared buffers via the bytes crate, optional mmap reader for memory-mapped decode." },
  { crate: "surp-compression", body: "Compression trait and optional zstd, lz4, and snappy adapters. All three are feature-gated; none are required." },
  { crate: "surp-ffi", body: "C ABI helpers — JSON-to-Surp and Surp-to-JSON buffer entry points for embedding in non-Rust hosts." },
  { crate: "surp-simd", body: "Scalar-safe scanning helpers and an optional aarch64 SIMD varint pre-scan path." },
  { crate: "bench", body: "Criterion-driven Rust and Python benchmark harnesses with deterministic datasets and committed result fixtures." },
  { crate: "fuzz", body: "cargo-fuzz targets and corpora for the decoder, the text parser, varints, block framing, and full roundtrips. Excluded from the workspace build by design." },
];

function Section({ eyebrow, title, children }: { eyebrow: string; title: string; children: React.ReactNode }) {
  return (
    <section className="mt-16">
      <div className="eyebrow text-primary mb-3">{eyebrow}</div>
      <h2 className="display-md text-ink">{title}</h2>
      <div className="prose mt-5 max-w-none">{children}</div>
    </section>
  );
}
function Mono({ children }: { children: React.ReactNode }) {
  return <code className="font-mono text-[13px] mx-[2px] bg-surface-strong px-1 py-0.5 rounded">{children}</code>;
}
function Note({ children }: { children: React.ReactNode }) {
  return (
    <aside className="not-prose mt-6 rounded-lg border border-hairline bg-canvas-soft px-4 py-3 text-[13.5px] text-muted">
      <span className="eyebrow text-primary mr-2">Note</span>{children}
    </aside>
  );
}
