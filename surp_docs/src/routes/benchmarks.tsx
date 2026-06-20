import { createFileRoute, Link } from "@tanstack/react-router";
import { BENCH_ROWS, type BenchRow } from "@/lib/bench-data";
import { I } from "@/lib/icons";

export const Route = createFileRoute("/benchmarks")({
  head: () => ({
    meta: [
      { title: "Benchmarks — Surp v1.0.1 regression suite" },
      { name: "description", content: "Deterministic regression benchmark of Surp against JSON, MessagePack, CBOR, and Protocol Buffers across six datasets. Sizes, encode and decode throughput, percentiles, methodology." },
      { property: "og:title", content: "Surp Benchmarks" },
      { property: "og:description", content: "Surp vs JSON, MessagePack, CBOR, Protocol Buffers — six deterministic datasets, full numbers." },
      { property: "og:url", content: "/benchmarks" },
    ],
    links: [{ rel: "canonical", href: "/benchmarks" }],
  }),
  component: Benchmarks,
});

const DATASETS: { id: string; title: string; blurb: string; shape: string }[] = [
  { id: "small_objects",    title: "small_objects",    shape: "100,000 records", blurb: "100k flat objects with 6 fields each (id, name, email, active, score, level). Exercises per-record overhead and small-string handling." },
  { id: "string_heavy",     title: "string_heavy",     shape: "10,000 records",  blurb: "10k records where ~60% of values are drawn from a 50-string pool. Designed to measure the win from string deduplication." },
  { id: "nested_deep",      title: "nested_deep",      shape: "10-deep tree + 50-deep linear", blurb: "A branching tree (depth 10, fanout 3) plus a 50-level linear chain. Stresses recursion, depth limits, and small-object framing." },
  { id: "binary_blobs",     title: "binary_blobs",     shape: "100 × ~64 KiB",   blurb: "Records carrying ~64 KiB random byte payloads. Exposes how each format treats raw bytes (native vs base64)." },
  { id: "mixed_api_events", title: "mixed_api_events", shape: "5,000 events",    blurb: "Synthetic GitHub-event-shaped objects with mixed scalar and nested fields. Models a realistic API payload." },
  { id: "numeric_heavy",    title: "numeric_heavy",    shape: "dense numerics",  blurb: "Arrays dominated by integers and floats. Exposes varint efficiency and float framing." },
];

const FORMATS: { id: string; label: string; tone: string }[] = [
  { id: "surp",       label: "Surp",         tone: "text-primary" },
  { id: "surp_dedup", label: "Surp + Dedup", tone: "text-ink" },
  { id: "json",       label: "JSON",         tone: "text-body" },
  { id: "msgpack",    label: "MessagePack",  tone: "text-body" },
  { id: "cbor",       label: "CBOR",         tone: "text-body" },
  { id: "protobuf",   label: "Protobuf",     tone: "text-body" },
];

function fmtBytes(n: number | null): string {
  if (n == null) return "—";
  if (n >= 1_048_576) return `${(n / 1_048_576).toFixed(1)} MB`;
  if (n >= 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${n} B`;
}
function fmtUs(us: number): string {
  if (us >= 1000) return `${(us / 1000).toFixed(2)} ms`;
  return `${us.toFixed(1)} µs`;
}
function rowFor(format: string, dataset: string, op: string): BenchRow | undefined {
  return BENCH_ROWS.find((r) => r.format === format && r.dataset === dataset && r.op === op);
}

function Benchmarks() {
  return (
    <>
      <Hero />
      <Methodology />
      <Datasets />
      <Charts />
      <SizeSection />
      <ThroughputSection op="encode" />
      <ThroughputSection op="decode" />
      <FullResults />
      <Reproduce />
    </>
  );
}

function Charts() {
  const charts = [
    {
      src: "/bench/serialized-size.svg",
      title: "Serialized size",
      blurb: "Bytes on disk per dataset. Lower is better. Surp lands smaller than JSON on every dataset; Surp + Dedup pulls ahead on string-heavy.",
      eyebrow: "Chart · size",
    },
    {
      src: "/bench/encode-throughput.svg",
      title: "Encode throughput",
      blurb: "Bytes written per second, median over 10 iterations. Higher is better. MsgPack edges ahead on the smallest payloads; Surp stays competitive across shapes.",
      eyebrow: "Chart · encode",
    },
    {
      src: "/bench/decode-throughput.svg",
      title: "Decode throughput",
      blurb: "Bytes parsed per second, median over 10 iterations. Higher is better. Surp's decode path is the most consistent across shapes.",
      eyebrow: "Chart · decode",
    },
  ];
  return (
    <section className="border-b border-hairline-soft">
      <div className="mx-auto max-w-[1200px] px-5 sm:px-8 py-16">
        <div className="eyebrow mb-3">Committed charts · v1.0.1</div>
        <h2 className="display-lg text-ink max-w-2xl">The same SVGs the harness commits to the repo.</h2>
        <p className="mt-3 text-body max-w-2xl">
          Rendered straight from <code className="font-mono text-[13px]">docs/assets/bench/v1.0.1/charts/</code>. No re-projection, no re-aggregation — exactly what <code className="font-mono text-[13px]">surp-bench</code> wrote.
        </p>
        <div className="mt-10 grid gap-10">
          {charts.map((c, i) => (
            <figure
              key={c.src}
              className="rise-in rounded-2xl border border-hairline bg-surface overflow-hidden"
              style={{ animationDelay: `${i * 80}ms` }}
            >
              <div className="flex items-baseline justify-between gap-3 px-5 sm:px-6 pt-5">
                <div>
                  <div className="eyebrow text-muted">{c.eyebrow}</div>
                  <h3 className="font-display text-[22px] text-ink mt-1">{c.title}</h3>
                </div>
                <a
                  href={`https://github.com/tubox-labs/surp/blob/master/docs/assets/bench/v1.0.1/charts/${c.src.split("/").pop()}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-[12px] text-muted hover:text-ink inline-flex items-center gap-1"
                >
                  source <I.ArrowUpRight width={11} height={11} />
                </a>
              </div>
              <figcaption className="px-5 sm:px-6 mt-2 text-[14px] text-body max-w-3xl">{c.blurb}</figcaption>
              <div className="mt-5 border-t border-hairline-soft bg-canvas-soft p-5 sm:p-8">
                <img
                  src={c.src}
                  alt={`${c.title} chart for the Surp v1.0.1 benchmark run`}
                  loading="lazy"
                  className="w-full h-auto block"
                />
              </div>
            </figure>
          ))}
        </div>
      </div>
    </section>
  );
}

function Hero() {
  return (
    <section className="border-b border-hairline-soft">
      <div className="mx-auto max-w-[1200px] px-5 sm:px-8 pt-16 sm:pt-20 pb-12">
        <div className="max-w-3xl">
          <div className="eyebrow mb-4">Benchmarks · committed v1.0.1 full run</div>
          <h1 className="display-mega text-ink">
            How Surp compares,<br />
            <span className="italic text-muted">measured, not claimed.</span>
          </h1>
          <p className="mt-7 text-[18px] leading-relaxed text-body max-w-2xl">
            A deterministic regression suite (<code className="font-mono text-[13.5px]">surp-bench</code>) encodes and decodes six fixed datasets with Surp, Surp&nbsp;+&nbsp;Dedup, JSON, MessagePack, CBOR, and Protocol Buffers, then writes the raw numbers below to{" "}
            <code className="font-mono text-[13.5px]">docs/assets/bench/v1.0.1/</code>. No vendor cherry-picking — every dataset, every operation, every format is shown.
          </p>
          <div className="mt-8 grid grid-cols-2 sm:grid-cols-4 gap-px bg-hairline border border-hairline rounded-xl overflow-hidden">
            <Stat k="Datasets" v="6" />
            <Stat k="Formats" v="6" />
            <Stat k="Iterations" v="10" />
            <Stat k="Regressions" v="0" tone="text-primary" />
          </div>
        </div>
      </div>
    </section>
  );
}

function Stat({ k, v, tone }: { k: string; v: string; tone?: string }) {
  return (
    <div className="bg-surface p-5">
      <div className="eyebrow text-muted">{k}</div>
      <div className={`mt-1 font-display text-[28px] tracking-tight ${tone ?? "text-ink"}`}>{v}</div>
    </div>
  );
}

function Methodology() {
  return (
    <section className="border-b border-hairline-soft bg-canvas-soft">
      <div className="mx-auto max-w-[1200px] px-5 sm:px-8 py-14">
        <div className="grid lg:grid-cols-[280px_minmax(0,1fr)] gap-10">
          <div>
            <div className="eyebrow mb-3">Methodology</div>
            <h2 className="display-md text-ink">The harness is deterministic on purpose.</h2>
            <p className="mt-3 text-body text-[15px]">Same seeds, same datasets, same iteration count. If a number moves, something in the codec moved.</p>
          </div>
          <dl className="grid sm:grid-cols-2 gap-px bg-hairline border border-hairline rounded-xl overflow-hidden text-[14px]">
            {[
              ["Mode", "full"],
              ["Iterations per measurement", "10"],
              ["OS / arch", "macos / aarch64"],
              ["CPU cores", "10"],
              ["Rust", "rustc 1.94.1 (2026-03-25)"],
              ["Dataset version", "1.0.0 (sha256-pinned)"],
              ["PRNG", "xorshift64, fixed seeds"],
              ["Allocator", "system (jemalloc optional)"],
              ["Output dir", "docs/assets/bench/v1.0.1/"],
              ["Regressions detected", "none"],
            ].map(([k, v]) => (
              <div key={k} className="bg-surface p-4">
                <dt className="text-[11px] eyebrow text-muted">{k}</dt>
                <dd className="mt-1 text-ink font-mono text-[13px]">{v}</dd>
              </div>
            ))}
          </dl>
        </div>
        <div className="mt-8 text-[13.5px] text-muted max-w-3xl leading-relaxed">
          The Protobuf comparison uses a generic <code className="font-mono">Value</code> schema so it can represent the same schema-less payloads as Surp and JSON. Numbers shown are <em>median</em> with <em>p95</em> and coefficient of variation, taken from <code className="font-mono">summary.csv</code> for the committed v1.0.1 run.
        </div>
      </div>
    </section>
  );
}

function Datasets() {
  return (
    <section className="border-b border-hairline-soft">
      <div className="mx-auto max-w-[1200px] px-5 sm:px-8 py-16">
        <div className="eyebrow mb-3">Datasets</div>
        <h2 className="display-lg text-ink max-w-xl">Six shapes that stress different parts of a codec.</h2>
        <div className="mt-10 grid sm:grid-cols-2 gap-px bg-hairline border border-hairline rounded-xl overflow-hidden">
          {DATASETS.map((d) => (
            <div key={d.id} className="bg-surface p-7">
              <div className="flex items-baseline justify-between gap-3">
                <h3 className="font-display text-[20px] text-ink">{d.title}</h3>
                <span className="text-[11px] eyebrow text-muted">{d.shape}</span>
              </div>
              <p className="mt-3 text-body text-[14.5px] leading-relaxed">{d.blurb}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function SizeSection() {
  // Build per-dataset size rows
  const rows = DATASETS.map((d) => {
    const get = (f: string) => rowFor(f, d.id, "encode")?.size ?? null;
    const surp = get("surp");
    const json = get("json");
    const ratio = surp && json ? surp / json : null;
    return {
      dataset: d.id,
      surp,
      dedup: get("surp_dedup"),
      json,
      msgpack: get("msgpack"),
      cbor: get("cbor"),
      protobuf: get("protobuf"),
      ratio,
    };
  });

  // For bar chart, normalize per dataset against max
  return (
    <section className="border-b border-hairline-soft bg-canvas-soft">
      <div className="mx-auto max-w-[1200px] px-5 sm:px-8 py-16">
        <div className="eyebrow mb-3">Serialized size</div>
        <h2 className="display-lg text-ink">Bytes on disk, per dataset.</h2>
        <p className="mt-3 text-body max-w-2xl">Surp lands smaller than JSON on every dataset; ratio shown in the right-most column. Surp + Dedup pulls ahead on string-heavy and ties Surp on raw binary.</p>

        <div className="mt-8 grid gap-6">
          {rows.map((r) => {
            const values = [
              { id: "surp", v: r.surp, label: "Surp" },
              { id: "surp_dedup", v: r.dedup, label: "Surp+Dedup" },
              { id: "json", v: r.json, label: "JSON" },
              { id: "msgpack", v: r.msgpack, label: "MsgPack" },
              { id: "cbor", v: r.cbor, label: "CBOR" },
              { id: "protobuf", v: r.protobuf, label: "Protobuf" },
            ];
            const max = Math.max(...values.map((x) => x.v ?? 0));
            return (
              <div key={r.dataset} className="rounded-xl border border-hairline bg-surface p-5">
                <div className="flex items-baseline justify-between gap-3">
                  <h3 className="font-mono text-[13.5px] text-ink">{r.dataset}</h3>
                  <span className="text-[11px] eyebrow text-muted">
                    surp/json = <span className="text-primary">{r.ratio ? r.ratio.toFixed(2) + "×" : "—"}</span>
                  </span>
                </div>
                <div className="mt-4 grid gap-2">
                  {values.map((x) => (
                    <div key={x.id} className="grid grid-cols-[110px_1fr_90px] gap-3 items-center text-[12.5px]">
                      <span className={`font-mono ${x.id.startsWith("surp") ? "text-ink" : "text-muted"}`}>{x.label}</span>
                      <div className="h-2 rounded-full bg-canvas-soft overflow-hidden">
                        <div
                          className={`h-full ${x.id === "surp" ? "bg-primary" : x.id === "surp_dedup" ? "bg-ink/70" : "bg-hairline-strong"}`}
                          style={{ width: x.v ? `${(x.v / max) * 100}%` : "0%" }}
                        />
                      </div>
                      <span className="text-right font-mono text-muted">{fmtBytes(x.v)}</span>
                    </div>
                  ))}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </section>
  );
}

function ThroughputSection({ op }: { op: "encode" | "decode" }) {
  return (
    <section className="border-b border-hairline-soft">
      <div className="mx-auto max-w-[1200px] px-5 sm:px-8 py-16">
        <div className="eyebrow mb-3">{op === "encode" ? "Encode throughput" : "Decode throughput"}</div>
        <h2 className="display-lg text-ink">
          {op === "encode" ? "Bytes written per second." : "Bytes parsed per second."}
        </h2>
        <p className="mt-3 text-body max-w-2xl">
          {op === "encode"
            ? "MsgPack is fastest to encode on the smallest payloads; Surp is competitive everywhere and notably wins decode (next chart) by a healthy margin."
            : "Surp's decode path is the most consistent across shapes — CBOR loses ground on small_objects, Protobuf rebounds on binary_blobs because it stores bytes natively."}
        </p>

        <div className="mt-8 grid gap-6">
          {DATASETS.map((d) => {
            const values = FORMATS.map((f) => {
              const r = rowFor(f.id, d.id, op);
              return { id: f.id, label: f.label, mbps: r?.mbps ?? null, medianUs: r?.medianUs ?? null, p95Us: r?.p95Us ?? null };
            }).filter((x) => x.mbps != null);
            const max = Math.max(...values.map((x) => x.mbps ?? 0));
            return (
              <div key={d.id} className="rounded-xl border border-hairline bg-surface p-5">
                <div className="flex items-baseline justify-between gap-3">
                  <h3 className="font-mono text-[13.5px] text-ink">{d.id}</h3>
                  <span className="text-[11px] eyebrow text-muted">higher is better · MB/s</span>
                </div>
                <div className="mt-4 grid gap-2">
                  {values.map((x) => (
                    <div key={x.id} className="grid grid-cols-[110px_1fr_140px] gap-3 items-center text-[12.5px]">
                      <span className={`font-mono ${x.id === "surp" ? "text-primary" : x.id === "surp_dedup" ? "text-ink" : "text-muted"}`}>{x.label}</span>
                      <div className="h-2 rounded-full bg-canvas-soft overflow-hidden">
                        <div
                          className={`h-full ${x.id === "surp" ? "bg-primary" : x.id === "surp_dedup" ? "bg-ink/70" : "bg-hairline-strong"}`}
                          style={{ width: max ? `${((x.mbps ?? 0) / max) * 100}%` : "0%" }}
                        />
                      </div>
                      <span className="text-right font-mono text-muted">
                        <span className="text-ink">{x.mbps?.toFixed(0)}</span> MB/s · <span className="text-muted-soft">{x.medianUs ? fmtUs(x.medianUs) : "—"}</span>
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </section>
  );
}

function FullResults() {
  return (
    <section className="border-b border-hairline-soft bg-canvas-soft">
      <div className="mx-auto max-w-[1200px] px-5 sm:px-8 py-16">
        <div className="eyebrow mb-3">Full numbers</div>
        <h2 className="display-lg text-ink">Every measurement, every format.</h2>
        <p className="mt-3 text-body max-w-2xl">Median, p95, coefficient of variation, throughput, and serialized size for all 6 datasets × 6 formats × 3 operations. Mirrored from <code className="font-mono">summary.csv</code>.</p>

        <div className="mt-8 overflow-x-auto rounded-xl border border-hairline bg-surface">
          <table className="w-full text-[12.5px] font-mono">
            <thead className="bg-canvas-soft text-muted">
              <tr className="text-left">
                <th className="px-3 py-2 font-medium">Format</th>
                <th className="px-3 py-2 font-medium">Dataset</th>
                <th className="px-3 py-2 font-medium">Op</th>
                <th className="px-3 py-2 font-medium text-right">Median</th>
                <th className="px-3 py-2 font-medium text-right">p95</th>
                <th className="px-3 py-2 font-medium text-right">CV%</th>
                <th className="px-3 py-2 font-medium text-right">MB/s</th>
                <th className="px-3 py-2 font-medium text-right">Size</th>
              </tr>
            </thead>
            <tbody>
              {BENCH_ROWS.map((r, i) => (
                <tr key={i} className="border-t border-hairline-soft hover:bg-canvas-soft">
                  <td className={`px-3 py-1.5 ${r.format === "surp" ? "text-primary" : r.format === "surp_dedup" ? "text-ink" : "text-body"}`}>{r.format}</td>
                  <td className="px-3 py-1.5 text-ink">{r.dataset}</td>
                  <td className="px-3 py-1.5 text-muted">{r.op}</td>
                  <td className="px-3 py-1.5 text-right text-ink">{fmtUs(r.medianUs)}</td>
                  <td className="px-3 py-1.5 text-right text-muted">{fmtUs(r.p95Us)}</td>
                  <td className="px-3 py-1.5 text-right text-muted">{r.cv.toFixed(1)}</td>
                  <td className="px-3 py-1.5 text-right text-ink">{r.mbps != null ? r.mbps.toFixed(1) : "—"}</td>
                  <td className="px-3 py-1.5 text-right text-muted">{fmtBytes(r.size)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </section>
  );
}

function Reproduce() {
  return (
    <section>
      <div className="mx-auto max-w-[1200px] px-5 sm:px-8 py-16">
        <div className="grid lg:grid-cols-[1fr_1fr] gap-10">
          <div>
            <div className="eyebrow mb-3">Reproduce locally</div>
            <h2 className="display-lg text-ink">Run it yourself.</h2>
            <p className="mt-3 text-body">All datasets are seeded; numbers will scale to your hardware but the rank order should hold.</p>
            <div className="mt-5 flex flex-wrap gap-3">
              <a href="https://github.com/tubox-labs/surp/tree/main/bench" target="_blank" rel="noopener noreferrer" className="inline-flex items-center gap-2 h-11 px-5 rounded-md bg-ink text-canvas text-[14px] font-medium">
                <I.GitHub width={14} height={14} /> bench/ on GitHub <I.ArrowUpRight width={13} height={13} />
              </a>
              <Link to="/architecture" className="inline-flex items-center gap-2 h-11 px-5 rounded-md border border-hairline-strong text-ink text-[14px] font-medium">Architecture <I.Arrow width={13} height={13} /></Link>
            </div>
          </div>
          <div className="rounded-xl border border-hairline bg-code-bg text-code-fg p-5 font-mono text-[12.5px] leading-relaxed overflow-x-auto">
            <div className="text-muted-soft"># CI fast mode — minutes</div>
            <div>cargo run -p surp-bench --release -- \</div>
            <div>{"  "}--mode ci --output bench/results</div>
            <div className="mt-3 text-muted-soft"># Full mode — committed in repo</div>
            <div>cargo run -p surp-bench --release -- \</div>
            <div>{"  "}--mode full --output bench/results/full</div>
            <div className="mt-3 text-muted-soft"># Python harness</div>
            <div>python bench/python/bench_surp.py</div>
          </div>
        </div>
      </div>
    </section>
  );
}
