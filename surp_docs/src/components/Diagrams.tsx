// Hand-drawn, layered architecture sketches.
// Not Mermaid, not boxy. SVG only, soft strokes, slight wobble.

import { type ReactNode } from "react";

function jitter(seed: number) {
  // deterministic per-edge tiny offset
  const x = Math.sin(seed * 9.13) * 1.6;
  const y = Math.cos(seed * 7.41) * 1.6;
  return { x, y };
}

export function ArchOverview() {
  return (
    <figure className="my-10">
      <svg viewBox="0 0 880 460" className="w-full h-auto" role="img" aria-label="Surp architecture overview">
        <defs>
          <filter id="rough" x="-5%" y="-5%" width="110%" height="110%">
            <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" seed="3" />
            <feDisplacementMap in="SourceGraphic" scale="0.8" />
          </filter>
          <marker id="ah" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
            <path d="M0,0 L10,5 L0,10 z" fill="var(--ink)" />
          </marker>
          <pattern id="dots" x="0" y="0" width="6" height="6" patternUnits="userSpaceOnUse">
            <circle cx="1" cy="1" r="0.6" fill="var(--hairline-strong)" />
          </pattern>
        </defs>

        {/* Background paper */}
        <rect x="0" y="0" width="880" height="460" fill="var(--canvas-soft)" rx="14" />
        <rect x="0" y="0" width="880" height="460" fill="url(#dots)" opacity="0.35" rx="14" />

        {/* Lanes */}
        <g filter="url(#rough)" fill="none" stroke="var(--hairline-strong)" strokeWidth="1.2" strokeDasharray="3 5">
          <line x1="40" y1="80" x2="840" y2="80" />
          <line x1="40" y1="200" x2="840" y2="200" />
          <line x1="40" y1="330" x2="840" y2="330" />
        </g>

        <g fontFamily="var(--font-sans)" fontSize="10" fill="var(--muted)" letterSpacing="1.4" textAnchor="end">
          <text x="36" y="60">SURFACES</text>
          <text x="36" y="180">CODEC</text>
          <text x="36" y="310">IO &amp; TRANSPORT</text>
          <text x="36" y="420">STORAGE</text>
        </g>

        {/* Surfaces row */}
        <SketchCard x={80}  y={20} w={150} h={50} label="surp-cli"    sub="binary tool" tone="ink" />
        <SketchCard x={260} y={20} w={170} h={50} label="surp-python"  sub="PyO3 module" tone="ink" />
        <SketchCard x={460} y={20} w={170} h={50} label="surp-ffi"     sub="C ABI helpers" tone="ink" />
        <SketchCard x={660} y={20} w={180} h={50} label="surp-derive"  sub="Surp / SurpSchema" tone="ink" />

        {/* Codec row */}
        <SketchCard x={120} y={130} w={280} h={120} label="surp-core" sub="Encoder · Decoder · Value · text" tone="primary" tall />
        <SketchCard x={460} y={130} w={180} h={55}  label="rfc001"     sub="CTN · CBF · CQL" tone="accent" />
        <SketchCard x={460} y={195} w={180} h={55}  label="limits"     sub="depth · size · count" tone="muted" />
        <SketchCard x={670} y={130} w={170} h={55}  label="checksum"   sub="XXH64 · XXH3" tone="muted" />
        <SketchCard x={670} y={195} w={170} h={55}  label="varint"     sub="LEB128" tone="muted" />

        {/* IO row */}
        <SketchCard x={120} y={270} w={200} h={50} label="surp-io"          sub="tokio framed · mmap" tone="ink" />
        <SketchCard x={340} y={270} w={200} h={50} label="surp-compression" sub="zstd · lz4 · snappy" tone="ink" />
        <SketchCard x={560} y={270} w={130} h={50} label="surp-simd"        sub="varint pre-scan" tone="ink" />
        <SketchCard x={710} y={270} w={130} h={50} label="bench"            sub="criterion" tone="muted" />

        {/* Storage row */}
        <SketchCard x={170} y={360} w={220} h={50} label=".surp file"  sub="block-framed v1" tone="accent" />
        <SketchCard x={410} y={360} w={220} h={50} label=".crb file"   sub="RFC-001 CBF" tone="accent" />
        <SketchCard x={640} y={360} w={180} h={50} label=".ctn fixture" sub="text notation" tone="muted" />

        {/* Connecting lines (hand-drawn feel) */}
        <g filter="url(#rough)" stroke="var(--ink)" strokeWidth="1.1" fill="none" markerEnd="url(#ah)" opacity="0.7">
          <path d="M155 70 C 160 100, 220 110, 250 130" />
          <path d="M345 70 C 340 100, 290 110, 270 130" />
          <path d="M545 70 C 545 100, 540 110, 540 130" />
          <path d="M750 70 C 750 100, 350 110, 300 130" />
          <path d="M260 250 C 260 260, 240 262, 230 268" />
          <path d="M380 250 C 400 260, 420 262, 440 268" />
          <path d="M260 320 C 260 340, 280 350, 290 358" />
          <path d="M520 320 C 520 340, 520 350, 520 358" />
          <path d="M620 250 L 620 268" />
        </g>

        {/* Margin notes (annotation style) */}
        <g fontFamily="var(--font-display)" fontSize="13" fill="var(--muted)" fontStyle="italic">
          <text x="430" y="115">↑ public surfaces depend on the codec, never the other way around</text>
          <text x="220" y="345">↑ all writers emit block-framed v1 by default</text>
        </g>
      </svg>
      <figcaption className="mt-4 text-[13px] text-muted max-w-2xl">
        Four lanes — surfaces, codec, IO &amp; transport, storage — mirror the workspace crate layout.
        Arrows show dependency direction; the codec never imports from a surface.
      </figcaption>
    </figure>
  );
}

function SketchCard({
  x, y, w, h, label, sub, tone = "ink", tall = false,
}: {
  x: number; y: number; w: number; h: number;
  label: string; sub?: string;
  tone?: "ink" | "primary" | "accent" | "muted";
  tall?: boolean;
}) {
  const fill =
    tone === "primary" ? "color-mix(in oklab, var(--primary) 8%, var(--surface-card))" :
    tone === "accent"  ? "color-mix(in oklab, var(--tl-read) 18%, var(--surface-card))" :
    tone === "muted"   ? "var(--canvas-soft)" :
    "var(--surface-card)";
  const stroke = tone === "primary" ? "var(--primary)" : "var(--hairline-strong)";
  return (
    <g filter="url(#rough)">
      <rect x={x} y={y} width={w} height={h} rx={10} fill={fill} stroke={stroke} strokeWidth={tone === "primary" ? 1.4 : 1} />
      <text x={x + 14} y={y + 22} fontFamily="var(--font-sans)" fontWeight="600" fontSize="13" fill="var(--ink)">{label}</text>
      {sub && <text x={x + 14} y={y + 38} fontFamily="var(--font-mono)" fontSize="11" fill="var(--muted)">{sub}</text>}
      {tall && <text x={x + 14} y={y + h - 12} fontFamily="var(--font-display)" fontStyle="italic" fontSize="12" fill="var(--muted)">the heart of the workspace</text>}
    </g>
  );
}

export function EncodePipeline() {
  return (
    <figure className="my-10">
      <svg viewBox="0 0 880 230" className="w-full h-auto" role="img" aria-label="Encode and decode pipeline">
        <defs>
          <filter id="rough2"><feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" seed="5"/><feDisplacementMap in="SourceGraphic" scale="0.7"/></filter>
          <marker id="ah2" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" fill="var(--ink)"/></marker>
        </defs>
        <rect width="880" height="230" rx="14" fill="var(--canvas-soft)"/>

        {[
          { x: 30,  label: "Value tree",   sub: "Value / SurpValue", tone: "ink" },
          { x: 200, label: "Encoder",      sub: "varint · dedup",    tone: "primary" },
          { x: 370, label: "Block writer", sub: "type · len · comp", tone: "ink" },
          { x: 540, label: "Checksum",     sub: "XXH64 per block",   tone: "accent" },
          { x: 710, label: "Trailer",      sub: "overall checksum",  tone: "primary" },
        ].map((s, i) => (
          <SketchCard key={i} x={s.x} y={70} w={150} h={70} label={s.label} sub={s.sub} tone={s.tone as any} />
        ))}

        <g filter="url(#rough2)" stroke="var(--ink)" strokeWidth="1.2" fill="none" markerEnd="url(#ah2)" opacity="0.75">
          <path d="M180 105 L 200 105" />
          <path d="M350 105 L 370 105" />
          <path d="M520 105 L 540 105" />
          <path d="M690 105 L 710 105" />
        </g>

        <g fontFamily="var(--font-display)" fontStyle="italic" fontSize="13" fill="var(--muted)">
          <text x="40" y="180">value tree</text>
          <text x="280" y="180">scalar &amp; container ops</text>
          <text x="450" y="180">framed payload</text>
          <text x="620" y="180">integrity</text>
          <text x="730" y="180">file end</text>
        </g>
      </svg>
      <figcaption className="mt-4 text-[13px] text-muted max-w-2xl">
        The encode path. Decode is the same diagram in reverse — checksums are verified
        before any payload is exposed to caller code.
      </figcaption>
    </figure>
  );
}

export function TrustBoundary({ children }: { children?: ReactNode }) {
  return (
    <figure className="my-10">
      <svg viewBox="0 0 880 260" className="w-full h-auto" role="img" aria-label="Trust boundary diagram">
        <defs>
          <filter id="rough3"><feTurbulence baseFrequency="0.85" numOctaves="2" seed="2"/><feDisplacementMap in="SourceGraphic" scale="0.7"/></filter>
        </defs>
        <rect width="880" height="260" rx="14" fill="var(--canvas-soft)"/>
        {/* untrusted region */}
        <g filter="url(#rough3)">
          <rect x="30" y="30" width="380" height="200" rx="14" fill="color-mix(in oklab, var(--semantic-error) 6%, var(--surface-card))" stroke="var(--semantic-error)" strokeWidth="1.2" strokeDasharray="5 4"/>
          <rect x="470" y="30" width="380" height="200" rx="14" fill="color-mix(in oklab, var(--tl-grep) 12%, var(--surface-card))" stroke="var(--semantic-success)" strokeWidth="1.2"/>
        </g>
        <text x="48" y="56" fontFamily="var(--font-sans)" fontSize="11" letterSpacing="1.4" fill="var(--semantic-error)" fontWeight="600">UNTRUSTED INPUT</text>
        <text x="488" y="56" fontFamily="var(--font-sans)" fontSize="11" letterSpacing="1.4" fill="var(--semantic-success)" fontWeight="600">VALIDATED VALUE TREE</text>

        <SketchCard x={60} y={90}  w={150} h={50} label="bytes"     sub=".surp / .crb" tone="muted"/>
        <SketchCard x={230} y={90} w={160} h={50} label="Decoder"   sub="checks limits" tone="primary"/>
        <SketchCard x={60} y={160} w={330} h={50} label="checksum verification" sub="XXH64 — fail closed" tone="ink"/>
        <SketchCard x={500} y={90} w={160} h={50} label="Value tree" sub="owned"  tone="accent"/>
        <SketchCard x={680} y={90} w={160} h={50} label="SurpValue<'a>" sub="borrowed" tone="accent"/>
        <SketchCard x={500} y={160} w={340} h={50} label="caller code" sub="never sees raw bytes" tone="muted"/>

        <g filter="url(#rough3)" stroke="var(--ink)" strokeWidth="1.1" fill="none" markerEnd="url(#ah)" opacity="0.75">
          <path d="M390 115 C 430 115, 450 115, 500 115" />
        </g>
      </svg>
      <figcaption className="mt-4 text-[13px] text-muted max-w-2xl">
        The decoder is the only bridge between untrusted bytes and caller code.
        Limits and checksums fail closed before any value is constructed.
      </figcaption>
      {children}
    </figure>
  );
}
