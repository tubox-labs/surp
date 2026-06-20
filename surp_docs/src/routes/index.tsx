import { createFileRoute, Link } from "@tanstack/react-router";
import { I } from "@/lib/icons";
import { highlight } from "@/lib/highlight";

export const Route = createFileRoute("/")({
  head: () => ({
    meta: [
      { title: "Surp — Compact binary serialization for Rust, Python & the CLI" },
      { name: "description", content: "Surp is a Rust-backed binary format with checksums, dedup, optional compression, Python bindings and a CLI. Stable v1 wire format, plus the additive RFC-001 work." },
      { property: "og:title", content: "Surp — Compact binary serialization" },
      { property: "og:description", content: "A canonical binary alternative to JSON, with a stable v1 format and an additive RFC-001 path." },
      { property: "og:url", content: "/" },
    ],
    links: [{ rel: "canonical", href: "/" }],
  }),
  component: Home,
});

function Home() {
  return (
    <>
      <Hero />
      <Pillars />
      <Snippets />
      <DeepArchitecture />
      <LowLevel />
      <Timeline />
      <CTA />
    </>
  );
}

function Hero() {
  return (
    <section className="relative">
      <div className="mx-auto max-w-[1200px] px-5 sm:px-8 pt-10 sm:pt-14 pb-16">
        <div className="max-w-3xl rise-in">
          <h1 className="display-mega text-ink">
            A compact, canonical
            <br />
            binary serializer.
            <br />
            <span className="italic text-muted">Human-readable when you need it.</span>
          </h1>
          <p className="mt-7 text-[19px] leading-relaxed text-body max-w-2xl">
            Surp is a Rust-backed serialization toolkit. A stable, block-framed v1 binary
            format with XXH64 checksums and optional compression — paired with a text notation,
            a CLI, native Python bindings, and a C ABI. The additive{" "}
            <Link to="/docs/rfc001" className="text-ink underline underline-offset-4 decoration-hairline-strong hover:decoration-primary">RFC-001 work</Link>{" "}
            lives in its own namespace.
          </p>
          <div className="mt-8 flex flex-wrap items-center gap-3">
            <Link
              to="/getting-started"
              className="inline-flex items-center gap-2 h-11 px-5 rounded-md bg-primary text-on-primary text-[14px] font-medium hover:bg-primary-active transition-colors"
            >
              Get started <I.Arrow width={14} height={14} />
            </Link>
            <a
              href="https://github.com/tubox-labs/surp"
              target="_blank" rel="noopener noreferrer"
              className="inline-flex items-center gap-2 h-11 px-5 rounded-md border border-hairline-strong text-ink text-[14px] font-medium hover:bg-surface-strong transition-colors"
            >
              <I.GitHub width={15} height={15} /> Source on GitHub <I.ArrowUpRight width={14} height={14} />
            </a>
          </div>
        </div>

        <div className="mt-14 rise-in">
          <FauxIDE />
        </div>
      </div>
    </section>
  );
}

const TEXT_NOTATION = `{
  id: 1001;
  name: "Alice";
  active: true;
  tags: ["admin", "ops"];
  settings: {
    theme: "dark";
    region: "us";
  };
  avatar: b64#AQID;
}`;

function FauxIDE() {
  return (
    <div className="rounded-xl border border-hairline bg-surface overflow-hidden shadow-[0_24px_60px_-30px_rgba(0,0,0,0.25)]">
      <div className="flex items-center gap-1.5 px-4 py-2.5 border-b border-hairline">
        <span className="w-2.5 h-2.5 rounded-full bg-surface-strong" />
        <span className="w-2.5 h-2.5 rounded-full bg-surface-strong" />
        <span className="w-2.5 h-2.5 rounded-full bg-surface-strong" />
        <span className="ml-3 text-[11px] eyebrow text-muted tracking-[0.18em]">USER.SURP · V1 TEXT NOTATION</span>
        <span className="ml-auto text-[11px] eyebrow text-muted">XXH64 ✓</span>
      </div>
      <div className="grid sm:grid-cols-[180px_minmax(0,1fr)]">
        <aside className="hidden sm:block bg-canvas-soft border-r border-hairline p-4 text-[12.5px] font-mono text-muted">
          <div className="eyebrow mb-3">workspace</div>
          {[
            "surp-core", "surp-derive", "surp-cli", "surp-python",
            "surp-io", "surp-compression", "surp-ffi", "surp-simd",
          ].map((c) => (
            <div key={c} className={`py-0.5 ${c === "surp-core" ? "text-ink font-medium" : ""}`}>
              <span className="text-muted-soft mr-1.5">·</span>{c}
            </div>
          ))}
        </aside>
        <div className="p-5 bg-code-bg">
          <pre className="font-mono text-[13px] leading-7 text-code-fg overflow-x-auto">
            <code dangerouslySetInnerHTML={{ __html: highlight(TEXT_NOTATION, "surp") }} />
          </pre>
        </div>
      </div>
      <div className="border-t border-hairline px-4 py-2.5 flex flex-wrap items-center gap-2 text-[11px] text-muted">
        <span className="eyebrow text-ink">$ surp from-json user.json -o user.surp</span>
        <span className="mx-2 text-muted-soft">→</span>
        <span>written · 124 bytes · 1 block · checksum valid</span>
        <span className="ml-auto">~0.27ms</span>
      </div>
    </div>
  );
}

function Pillars() {
  const items = [
    { eyebrow: "Format",   title: "Stable v1 wire format",        body: "Block-framed files, per-block XXH64 checksums and a trailer checksum. Optional string deduplication. Forward and backward compatible schema evolution.", to: "/docs/spec" },
    { eyebrow: "Codec",    title: "Zero-copy when it can",         body: "Owned decode through Value, borrowed zero-copy decode through SurpValue<'a> for uncompressed v1 data. Resource limits enforced before any allocation.", to: "/docs/rust-api" },
    { eyebrow: "Surfaces", title: "CLI, Python, Rust, C ABI",      body: "The same codec drives a binary tool, a native PyO3 Python package, the public Rust API, and small C ABI helpers for JSON↔Surp buffers.", to: "/docs/cli" },
    { eyebrow: "RFC-001",  title: "An additive next-gen path",     body: "CTN text + CBF binary + a baseline CQL path query engine, under surp_core::rfc001 and surp.rfc001. Separate namespace, separate file type.", to: "/docs/rfc001" },
  ];
  return (
    <section className="border-t border-hairline-soft">
      <div className="mx-auto max-w-[1200px] px-5 sm:px-8 py-20">
        <div className="flex items-end justify-between gap-6 mb-10">
          <div>
            <div className="eyebrow mb-3">What you get</div>
            <h2 className="display-lg text-ink max-w-xl">Four surfaces. One codec underneath.</h2>
          </div>
          <Link to="/architecture" className="hidden sm:inline-flex items-center gap-2 text-[14px] text-ink hover:text-primary">
            See the architecture <I.Arrow width={14} height={14} />
          </Link>
        </div>
        <div className="grid sm:grid-cols-2 gap-px bg-hairline border border-hairline rounded-xl overflow-hidden">
          {items.map((it) => (
            <Link key={it.title} to={it.to} className="group bg-surface p-7 hover:bg-canvas-soft transition-colors">
              <div className="eyebrow text-primary">{it.eyebrow}</div>
              <h3 className="display-md mt-3 text-ink group-hover:underline underline-offset-4 decoration-hairline-strong">{it.title}</h3>
              <p className="mt-3 text-body text-[15px] leading-relaxed">{it.body}</p>
              <div className="mt-5 inline-flex items-center gap-2 text-[13px] text-muted group-hover:text-ink">
                Read <I.Arrow width={12} height={12} />
              </div>
            </Link>
          ))}
        </div>
      </div>
    </section>
  );
}

const RUST_SNIPPET = `use surp_core::{Decoder, Encoder, Value};

let value = Value::Object(vec![
    ("name".into(), Value::Str("Alice".into())),
    ("age".into(),  Value::UInt(30)),
    ("active".into(), Value::Bool(true)),
]);

let mut encoder = Encoder::new();
encoder.encode_value(&value)?;
let bytes = encoder.finish()?;

let mut decoder = Decoder::new(&bytes);
let decoded = decoder.decode_next()?.to_owned_value();
assert_eq!(decoded, value);`;

const PY_SNIPPET = `import surp

payload = {
    "name": "Alice",
    "age": 30,
    "active": True,
    "avatar": b"\\x01\\x02\\x03",
}

data = surp.dumps(payload, dedup=True, sort_keys=True)
decoded = surp.loads(data)
assert decoded == payload

view = surp.loads_value(data)
assert view["name"].value == "Alice"`;

function Snippets() {
  return (
    <section className="border-t border-hairline-soft bg-canvas-soft">
      <div className="mx-auto max-w-[1200px] px-5 sm:px-8 py-20 grid lg:grid-cols-2 gap-10">
        <div>
          <div className="eyebrow mb-3">Rust</div>
          <h3 className="display-md text-ink">Encode and decode without ceremony.</h3>
          <p className="mt-3 text-body max-w-md">A small surface area: <code className="font-mono text-[13px]">Value</code>, <code className="font-mono text-[13px]">Encoder</code>, <code className="font-mono text-[13px]">Decoder</code>, and the borrowed <code className="font-mono text-[13px]">SurpValue&lt;'a&gt;</code>. Derive macros cover named structs.</p>
          <SnippetCode lang="rust" code={RUST_SNIPPET} />
        </div>
        <div>
          <div className="eyebrow mb-3">Python</div>
          <h3 className="display-md text-ink">A package called <span className="italic">surp</span>. Nothing else to learn.</h3>
          <p className="mt-3 text-body max-w-md">PyO3 module with the obvious <code className="font-mono text-[13px]">dumps</code>/<code className="font-mono text-[13px]">loads</code>, plus a typed <code className="font-mono text-[13px]">SurpValue</code> view for discoverable access.</p>
          <SnippetCode lang="python" code={PY_SNIPPET} />
        </div>
      </div>
    </section>
  );
}

function SnippetCode({ code, lang }: { code: string; lang: string }) {
  return (
    <div className="codeblock mt-5" data-lang={lang}>
      <div className="codeblock-head"><span>{lang}</span><span className="text-muted-soft">verified from README.md</span></div>
      <div className="codeblock-body">
        <pre className="codeblock-code"><code dangerouslySetInnerHTML={{ __html: highlight(code, lang) }} /></pre>
      </div>
    </div>
  );
}

/* ───────────────────────── Deep Architecture (high-level) ───────────────────────── */

function DeepArchitecture() {
  return (
    <section className="border-t border-hairline-soft">
      <div className="mx-auto max-w-[1200px] px-5 sm:px-8 py-20">
        <div className="flex items-end justify-between gap-6 mb-10">
          <div className="max-w-2xl">
            <div className="eyebrow mb-3">System design · high-level</div>
            <h2 className="display-lg text-ink">One codec, traced from caller to bytes.</h2>
            <p className="mt-4 text-body text-[15.5px] leading-relaxed">
              Every surface — the CLI, the Python module, the C ABI, the MCP server — collapses into the same in-memory <code className="font-mono text-[13.5px]">Value</code> tree, which the encoder walks to produce the v1 block stream. Decoding plays the same stages in reverse, with limits and checksums checked <em>before</em> any value crosses back into caller code.
            </p>
          </div>
          <Link to="/architecture" className="hidden sm:inline-flex items-center gap-2 text-[14px] text-ink hover:text-primary">
            Architecture page <I.Arrow width={14} height={14} />
          </Link>
        </div>

        <HighLevelDiagram />

        <div className="mt-12 grid sm:grid-cols-3 gap-px bg-hairline border border-hairline rounded-xl overflow-hidden">
          {[
            ["Trust boundary", "Inputs (file, network, FFI buffer) are untrusted. Limits, checksums, and varint bounds are enforced before allocations grow."],
            ["Single codec", "All language surfaces call into surp-core. Behaviour matches across Python, Rust, the CLI, and the C ABI by construction."],
            ["Additive RFC-001", "CBF/CTN/CQL live under their own namespace. v1 files stay byte-compatible forever; nothing in RFC-001 changes how a .surp file is read."],
          ].map(([k, v]) => (
            <div key={k} className="bg-surface p-6">
              <div className="eyebrow text-primary">{k}</div>
              <p className="mt-2 text-body text-[14px] leading-relaxed">{v}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function HighLevelDiagram() {
  // Hand-drawn rough sketch — three swim lanes (Surfaces → Core → Bytes)
  const lanes = [
    { y: 60, label: "SURFACES" },
    { y: 200, label: "CORE  (surp-core)" },
    { y: 340, label: "BYTES" },
  ];
  return (
    <div className="rounded-2xl border border-hairline bg-canvas-soft p-3">
      <svg viewBox="0 0 1100 420" role="img" aria-label="Surp high-level architecture" className="w-full h-auto">
        <defs>
          <filter id="rough-h" x="-2%" y="-2%" width="104%" height="104%">
            <feTurbulence type="fractalNoise" baseFrequency="0.85" numOctaves="2" seed="7" />
            <feDisplacementMap in="SourceGraphic" scale="0.9" />
          </filter>
          <marker id="ah-h" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
            <path d="M0,0 L10,5 L0,10 z" fill="var(--ink)" />
          </marker>
          <pattern id="dots-h" x="0" y="0" width="8" height="8" patternUnits="userSpaceOnUse">
            <circle cx="1" cy="1" r="0.6" fill="var(--hairline-strong)" />
          </pattern>
        </defs>

        <rect x="0" y="0" width="1100" height="420" fill="var(--canvas-soft)" rx="14" />
        <rect x="0" y="0" width="1100" height="420" fill="url(#dots-h)" opacity="0.4" rx="14" />

        {/* lane dividers */}
        <g filter="url(#rough-h)" fill="none" stroke="var(--hairline-strong)" strokeWidth="1.1" strokeDasharray="3 5">
          <line x1="30" y1="120" x2="1070" y2="120" />
          <line x1="30" y1="260" x2="1070" y2="260" />
        </g>
        <g fontFamily="var(--font-sans)" fontSize="10" letterSpacing="1.6" fill="var(--muted)" textAnchor="start">
          {lanes.map((l) => (<text key={l.label} x="32" y={l.y - 28}>{l.label}</text>))}
        </g>

        {/* Surfaces row */}
        <RoughBox x={70}  y={40} w={170} h={56} title="surp-cli"     sub="binary tool" />
        <RoughBox x={260} y={40} w={170} h={56} title="surp-python"   sub="PyO3 module" />
        <RoughBox x={450} y={40} w={170} h={56} title="surp-ffi"      sub="C ABI" />
        <RoughBox x={640} y={40} w={170} h={56} title="surp-mcp"      sub="MCP server" />
        <RoughBox x={830} y={40} w={200} h={56} title="surp-derive"   sub="#[derive(Surp)]" tone="accent" />

        {/* arrows to Value */}
        <g stroke="var(--ink)" strokeWidth="1.1" fill="none" markerEnd="url(#ah-h)" filter="url(#rough-h)">
          {[155, 345, 535, 725, 930].map((x, i) => (
            <path key={i} d={`M${x} 100 C ${x} 140, 550 150, 550 178`} />
          ))}
        </g>

        {/* Core row — Value tree */}
        <RoughBox x={340}  y={180} w={420} h={56} title="Value / SurpValue<'a>" sub="ordered Object · Array · scalars · Bytes" tone="primary" />
        <RoughBox x={90}   y={180} w={210} h={56} title="Encoder"  sub="walks · varints · dedup table" />
        <RoughBox x={800}  y={180} w={210} h={56} title="Decoder"  sub="bounds · limits · zero-copy" />

        {/* arrows core → bytes */}
        <g stroke="var(--ink)" strokeWidth="1.1" fill="none" markerEnd="url(#ah-h)" filter="url(#rough-h)">
          <path d="M195 240 C 195 280, 280 290, 280 318" />
          <path d="M905 240 C 905 280, 820 290, 820 318" />
        </g>

        {/* Bytes row — block stream */}
        <g transform="translate(220 320)" filter="url(#rough-h)">
          <rect x="0"   y="0" width="60" height="56" fill="var(--canvas)"      stroke="var(--ink)" strokeWidth="1.2" rx="4" />
          <rect x="64"  y="0" width="110" height="56" fill="var(--canvas)"     stroke="var(--ink)" strokeWidth="1.2" rx="4" />
          <rect x="178" y="0" width="110" height="56" fill="var(--canvas)"     stroke="var(--ink)" strokeWidth="1.2" rx="4" />
          <rect x="292" y="0" width="110" height="56" fill="var(--canvas)"     stroke="var(--ink)" strokeWidth="1.2" rx="4" />
          <rect x="406" y="0" width="170" height="56" fill="var(--canvas)"     stroke="var(--ink)" strokeWidth="1.2" rx="4" />
          <rect x="580" y="0" width="80"  height="56" fill="var(--primary)"    stroke="var(--ink)" strokeWidth="1.2" rx="4" opacity="0.18" />
        </g>
        <g fontFamily="var(--font-mono)" fontSize="11" fill="var(--ink)" textAnchor="middle">
          <text x="250" y="354">MAGIC</text>
          <text x="275" y="368" fontSize="9" fill="var(--muted)">SURP</text>

          <text x="343" y="354">VERSION</text>
          <text x="343" y="368" fontSize="9" fill="var(--muted)">u8</text>

          <text x="457" y="354">BLOCK 0</text>
          <text x="457" y="368" fontSize="9" fill="var(--muted)">type · len · ck</text>

          <text x="571" y="354">BLOCK 1</text>
          <text x="571" y="368" fontSize="9" fill="var(--muted)">… payload …</text>

          <text x="711" y="354">STRING TABLE (opt)</text>
          <text x="711" y="368" fontSize="9" fill="var(--muted)">dedup</text>

          <text x="840" y="354">TRAILER</text>
          <text x="840" y="368" fontSize="9" fill="var(--muted)">XXH64</text>
        </g>

        {/* Trust boundary line */}
        <g filter="url(#rough-h)">
          <line x1="800" y1="170" x2="800" y2="280" stroke="var(--primary)" strokeWidth="1.4" strokeDasharray="2 4" />
        </g>
        <g fontFamily="var(--font-sans)" fontSize="10" fill="var(--primary)" letterSpacing="1.4">
          <text x="810" y="166">TRUST ↓</text>
          <text x="810" y="178" fill="var(--muted)" fontSize="9">limits + ck verified before alloc</text>
        </g>
      </svg>
      <div className="px-4 pt-2 pb-3 text-[11px] eyebrow text-muted tracking-[0.18em]">FIG · HIGH-LEVEL DATA FLOW</div>
    </div>
  );
}

function RoughBox({ x, y, w, h, title, sub, tone = "default" }: { x: number; y: number; w: number; h: number; title: string; sub: string; tone?: "default" | "primary" | "accent" }) {
  const fill = tone === "primary" ? "var(--primary)" : tone === "accent" ? "var(--ink)" : "var(--canvas)";
  const opacity = tone === "primary" ? 0.12 : tone === "accent" ? 0.06 : 1;
  const textFill = "var(--ink)";
  return (
    <g filter="url(#rough-h)">
      <rect x={x} y={y} width={w} height={h} fill={fill} fillOpacity={opacity} stroke="var(--ink)" strokeWidth="1.2" rx="6" />
      <text x={x + 14} y={y + 22} fontFamily="var(--font-mono)" fontSize="13" fill={textFill}>{title}</text>
      <text x={x + 14} y={y + 40} fontFamily="var(--font-sans)" fontSize="11" fill="var(--muted)">{sub}</text>
    </g>
  );
}

/* ───────────────────────── Low-level (block + encode pipeline) ───────────────────────── */

function LowLevel() {
  return (
    <section className="border-t border-hairline-soft bg-canvas-soft">
      <div className="mx-auto max-w-[1200px] px-5 sm:px-8 py-20">
        <div className="max-w-2xl mb-10">
          <div className="eyebrow mb-3">System design · low-level</div>
          <h2 className="display-lg text-ink">A block, byte by byte.</h2>
          <p className="mt-4 text-body text-[15.5px] leading-relaxed">
            v1 stores values inside framed blocks. Each block is self-describing: type, length, compression flag, payload, and a per-block XXH64 checksum. The trailer closes the file with an overall checksum and (optionally) an index for random access.
          </p>
        </div>

        <BlockDiagram />

        <div className="mt-12 grid lg:grid-cols-[1fr_1fr] gap-10 items-start">
          <EncodePipeline />
          <div>
            <div className="eyebrow mb-3 text-primary">Why these choices</div>
            <h3 className="display-md text-ink">Small reads stay small. Big reads stay safe.</h3>
            <ul className="mt-5 space-y-4 text-[14.5px] text-body leading-relaxed">
              <li className="grid grid-cols-[14px_1fr] gap-3"><span className="mt-2 h-1.5 w-1.5 rounded-full bg-primary" /><span><strong className="text-ink">Length-prefixed blocks</strong> let a reader skip past a payload it doesn't care about — no streaming-state machine, no peek-ahead.</span></li>
              <li className="grid grid-cols-[14px_1fr] gap-3"><span className="mt-2 h-1.5 w-1.5 rounded-full bg-primary" /><span><strong className="text-ink">Per-block XXH64</strong> means corruption is localized; a damaged block fails fast, the others still verify.</span></li>
              <li className="grid grid-cols-[14px_1fr] gap-3"><span className="mt-2 h-1.5 w-1.5 rounded-full bg-primary" /><span><strong className="text-ink">Optional string-dedup block</strong> trades a small encode cost for repeated string savings — see <Link to="/benchmarks" className="text-ink underline underline-offset-2 decoration-hairline-strong hover:decoration-primary">string_heavy</Link>.</span></li>
              <li className="grid grid-cols-[14px_1fr] gap-3"><span className="mt-2 h-1.5 w-1.5 rounded-full bg-primary" /><span><strong className="text-ink">Limits before allocation</strong> — max-depth, max-bytes, max-strings — are enforced from the varint header alone, so an adversarial input never grows memory.</span></li>
              <li className="grid grid-cols-[14px_1fr] gap-3"><span className="mt-2 h-1.5 w-1.5 rounded-full bg-primary" /><span><strong className="text-ink">Zero-copy borrows</strong> through <code className="font-mono text-[13px]">SurpValue&lt;'a&gt;</code> only apply to uncompressed v1 data; compressed blocks fall back to owned <code className="font-mono text-[13px]">Value</code>.</span></li>
            </ul>
          </div>
        </div>
      </div>
    </section>
  );
}

function BlockDiagram() {
  return (
    <div className="rounded-2xl border border-hairline bg-canvas p-3">
      <svg viewBox="0 0 1100 230" role="img" aria-label="Surp v1 block layout" className="w-full h-auto">
        <defs>
          <filter id="rough-b" x="-2%" y="-2%" width="104%" height="104%">
            <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" seed="11" />
            <feDisplacementMap in="SourceGraphic" scale="0.7" />
          </filter>
        </defs>

        {/* Bytes ruler */}
        <g fontFamily="var(--font-mono)" fontSize="10" fill="var(--muted-soft)" textAnchor="middle">
          {Array.from({ length: 11 }).map((_, i) => {
            const x = 60 + i * 96;
            return <g key={i}><line x1={x} y1={42} x2={x} y2={48} stroke="var(--hairline-strong)" /><text x={x} y={38}>{i}</text></g>;
          })}
          <text x="1090" y="38" textAnchor="end" fill="var(--muted)">byte offset →</text>
        </g>

        {/* Fields */}
        {[
          { x: 60,  w: 60,  label: "type",        sub: "u8",            tone: "primary" },
          { x: 124, w: 130, label: "len",         sub: "varint",        tone: "default" },
          { x: 258, w: 70,  label: "comp",        sub: "u8",            tone: "default" },
          { x: 332, w: 420, label: "payload",     sub: "raw or zstd / lz4 compressed", tone: "ink" },
          { x: 756, w: 130, label: "checksum",    sub: "XXH64",         tone: "primary" },
          { x: 890, w: 170, label: "next block",  sub: "→",             tone: "ghost" },
        ].map((f) => {
          const fill =
            f.tone === "primary" ? "var(--primary)" :
            f.tone === "ink" ? "var(--ink)" :
            f.tone === "ghost" ? "transparent" :
            "var(--canvas-soft)";
          const opacity = f.tone === "primary" ? 0.14 : f.tone === "ink" ? 0.05 : f.tone === "ghost" ? 1 : 1;
          const stroke = f.tone === "ghost" ? "var(--hairline-strong)" : "var(--ink)";
          const dash = f.tone === "ghost" ? "3 4" : undefined;
          return (
            <g key={f.label} filter="url(#rough-b)">
              <rect x={f.x} y={70} width={f.w} height={80} fill={fill} fillOpacity={opacity} stroke={stroke} strokeWidth="1.2" rx="6" strokeDasharray={dash} />
              <text x={f.x + f.w / 2} y={108} textAnchor="middle" fontFamily="var(--font-mono)" fontSize="13" fill="var(--ink)">{f.label}</text>
              <text x={f.x + f.w / 2} y={128} textAnchor="middle" fontFamily="var(--font-sans)" fontSize="11" fill="var(--muted)">{f.sub}</text>
            </g>
          );
        })}

        {/* Caption arrow */}
        <g filter="url(#rough-b)" stroke="var(--ink)" fill="none" strokeWidth="1">
          <path d="M60 175 L 886 175" />
          <path d="M60 170 L 60 180" />
          <path d="M886 170 L 886 180" />
        </g>
        <text x="473" y="200" textAnchor="middle" fontFamily="var(--font-sans)" fontSize="11" fill="var(--muted)" letterSpacing="1.2">ONE FRAMED BLOCK · self-describing · independently checksummed</text>
      </svg>
      <div className="px-4 pt-2 pb-3 text-[11px] eyebrow text-muted tracking-[0.18em]">FIG · BLOCK LAYOUT (V1)</div>
    </div>
  );
}

function EncodePipeline() {
  const stages = [
    ["Value tree",    "Caller hands in a Rust struct (via derive) or a Python dict → mapped to ordered Value::Object."],
    ["Walk + bound",  "Encoder walks depth-first, checks size budget at each level, picks varint width per length."],
    ["String dedup",  "If enabled, repeated strings are replaced by indices into a side string table block."],
    ["Block framing", "Payload is wrapped with type / len / comp / xxh64. Compression (zstd/lz4) is optional and per-block."],
    ["Trailer",       "Overall XXH64 closes the file. Optional index block enables random-access reads."],
  ];
  return (
    <div className="rounded-2xl border border-hairline bg-canvas p-5">
      <div className="eyebrow text-primary mb-4">Encode pipeline</div>
      <ol className="grid">
        {stages.map(([k, v], i) => (
          <li key={k} className="grid grid-cols-[28px_1fr] gap-3 py-3 border-t border-hairline-soft first:border-t-0">
            <span className="mt-1 inline-flex h-6 w-6 items-center justify-center rounded-full border border-hairline text-[11px] font-mono text-ink">{i + 1}</span>
            <div>
              <div className="text-ink font-medium text-[14px]">{k}</div>
              <div className="text-body text-[13.5px] mt-0.5 leading-relaxed">{v}</div>
            </div>
          </li>
        ))}
      </ol>
    </div>
  );
}

function Timeline() {
  return (
    <section className="border-t border-hairline-soft">
      <div className="mx-auto max-w-[1200px] px-5 sm:px-8 py-20">
        <div className="max-w-2xl">
          <div className="eyebrow mb-3">The shape of a release</div>
          <h2 className="display-lg text-ink">From source to a checked .surp file.</h2>
          <p className="mt-4 text-body">
            Every encode follows the same set of stages — and decode plays them back in reverse.
            The pastel pills below are scoped to product surfaces; here they label the editorial stages.
          </p>
        </div>
        <ol className="mt-10 grid gap-5">
          {[
            ["tl-pill-thinking", "Schema in mind", "You start with a Rust struct, a Python dict, or a JSON file. Surp doesn't require a schema, but a derive macro can encode IDs for stable evolution."],
            ["tl-pill-grep", "Encode", "Encoder walks the value tree, emits varints for lengths, optionally dedups strings. No compression by default."],
            ["tl-pill-read", "Block framing", "The block writer prefixes payloads with type + length + compression type + an XXH64 checksum."],
            ["tl-pill-edit", "Trailer", "A final overall checksum closes the file. The container is now random-access friendly with an optional index block."],
            ["tl-pill-done", "Verified bytes", "On decode, limits and checksums are checked before any value crosses the boundary into caller code."],
          ].map(([cls, label, body], i) => (
            <li key={i} className="grid grid-cols-[auto_1fr] gap-5 items-start py-5 border-t border-hairline">
              <span className={`tl-pill ${cls}`}>{label}</span>
              <p className="text-body leading-relaxed text-[15px]">{body}</p>
            </li>
          ))}
        </ol>
      </div>
    </section>
  );
}

function CTA() {
  return (
    <section className="border-t border-hairline-soft">
      <div className="mx-auto max-w-[1200px] px-5 sm:px-8 py-24 text-center">
        <h2 className="display-lg text-ink">Read the spec. Ship a value.</h2>
        <p className="mt-3 text-body max-w-xl mx-auto">Everything in this site is generated from the Surp source. The links below take you straight to verified material.</p>
        <div className="mt-7 flex flex-wrap justify-center gap-3">
          <Link to="/docs/spec" className="h-11 px-5 inline-flex items-center rounded-md bg-primary text-on-primary text-[14px] font-medium">Read the v1 spec</Link>
          <Link to="/architecture" className="h-11 px-5 inline-flex items-center rounded-md border border-hairline-strong text-ink text-[14px] font-medium">Architecture</Link>
          <Link to="/benchmarks" className="h-11 px-5 inline-flex items-center rounded-md border border-hairline-strong text-ink text-[14px] font-medium">Benchmarks</Link>
        </div>
      </div>
    </section>
  );
}
