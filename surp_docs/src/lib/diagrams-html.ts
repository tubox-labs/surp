/**
 * Hand-drawn, bespoke SVG diagrams returned as inline HTML strings, so they
 * can be emitted from the markdown renderer when a fence uses the
 * `surp-diagram` language. Every diagram is built from raw SVG primitives —
 * no Mermaid, no icon packs — and styled against the site's semantic tokens
 * via inline `currentColor` / CSS variables so they invert in dark mode.
 *
 * Visual language:
 *  - Warm cream surface with hand-drawn turbulence-filtered strokes
 *  - JetBrains-mono labels, Instrument Serif numerals
 *  - Cursor Orange (`var(--primary)`) used scarcely as accent
 *  - Generous whitespace, no boxy gradients
 */

const TURB_DEFS = `
  <defs>
    <filter id="rough" x="-2%" y="-2%" width="104%" height="104%">
      <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" seed="7"/>
      <feDisplacementMap in="SourceGraphic" scale="1.1"/>
    </filter>
    <filter id="paper" x="0" y="0" width="100%" height="100%">
      <feTurbulence type="fractalNoise" baseFrequency="0.55" numOctaves="2" seed="3"/>
      <feColorMatrix values="0 0 0 0 0.96  0 0 0 0 0.95  0 0 0 0 0.92  0 0 0 0.06 0"/>
    </filter>
    <pattern id="grid" width="22" height="22" patternUnits="userSpaceOnUse">
      <path d="M 22 0 L 0 0 0 22" fill="none" stroke="currentColor" stroke-opacity="0.06" stroke-width="0.7"/>
    </pattern>
    <marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M 0 0 L 10 5 L 0 10 z" fill="currentColor"/>
    </marker>
  </defs>
`;

const wrap = (title: string, eyebrow: string, viewBox: string, body: string, height = 440) => `
<figure class="surp-diagram" role="figure" aria-label="${title}">
  <header class="surp-diagram__head">
    <span class="surp-diagram__eyebrow">${eyebrow}</span>
    <h4 class="surp-diagram__title">${title}</h4>
  </header>
  <div class="surp-diagram__canvas" style="--diagram-h:${height}px">
    <svg viewBox="${viewBox}" xmlns="http://www.w3.org/2000/svg" preserveAspectRatio="xMidYMid meet">
      ${TURB_DEFS}
      <rect width="100%" height="100%" fill="url(#grid)"/>
      ${body}
    </svg>
  </div>
</figure>`;

/* ───────────────── SPEC: file layout ───────────────── */
function fileLayout() {
  const blocks = [
    { y: 40,  type: "0x04", name: "StringDict", note: "prefix-delta entries" },
    { y: 100, type: "0x01", name: "Data · block 0", note: "wire-encoded values" },
    { y: 160, type: "0x01", name: "Data · block 1", note: "wire-encoded values" },
    { y: 220, type: "0x01", name: "Data · block N", note: "wire-encoded values" },
    { y: 280, type: "0x02", name: "Index", note: "optional · offsets" },
    { y: 340, type: "0xFF", name: "Trailer", note: "XXH64 over file" },
  ];
  const body = `
    <g font-family="JetBrains Mono, monospace" fill="currentColor">
      ${blocks.map((b, i) => `
        <g transform="translate(40 ${b.y})" filter="url(#rough)">
          <rect x="0" y="0" width="640" height="46" rx="6" fill="none"
            stroke="currentColor" stroke-width="${b.type === "0xFF" ? 1.6 : 1}"
            stroke-dasharray="${b.type === "0x02" ? "5 3" : "0"}"/>
          <text x="14" y="20" font-size="10.5" fill="var(--muted)" letter-spacing="1.2">BLOCK ${i.toString().padStart(2, "0")}</text>
          <text x="14" y="36" font-size="13.5" font-weight="600">${b.name}</text>
          <text x="220" y="29" font-size="11" fill="var(--muted)">${b.note}</text>
          <g transform="translate(560 12)">
            <rect x="0" y="0" width="64" height="22" rx="3" fill="none" stroke="currentColor" stroke-opacity=".5"/>
            <text x="32" y="15" font-size="11" text-anchor="middle" fill="var(--primary)">${b.type}</text>
          </g>
        </g>
      `).join("")}
      <g stroke="currentColor" stroke-width="0.8" stroke-opacity=".4">
        ${blocks.slice(0, -1).map((b) => `<line x1="360" y1="${b.y + 46}" x2="360" y2="${b.y + 60}" marker-end="url(#arrow)"/>`).join("")}
      </g>
      <g transform="translate(40 400)" font-family="JetBrains Mono, monospace" fill="var(--muted)" font-size="10.5">
        <text x="0" y="0" letter-spacing="1.2">SEQUENTIAL · SELF-CONTAINED · TRAILER-VERIFIED</text>
      </g>
    </g>
  `;
  return wrap("File layout", "Spec · §2", "0 0 720 430", body, 470);
}

/* ───────────────── SPEC: block header strip ───────────────── */
function blockHeader() {
  const fields = [
    { w: 80,  name: "block_type", size: "1 B",   tone: "ink" },
    { w: 110, name: "block_len",  size: "varint", tone: "ink" },
    { w: 90,  name: "comp_type",  size: "1 B",   tone: "ink" },
    { w: 130, name: "checksum",   size: "8 B · XXH64", tone: "primary" },
    { w: 230, name: "payload",    size: "block_len bytes", tone: "ink" },
  ];
  let x = 40;
  const body = `
    <g font-family="JetBrains Mono, monospace" fill="currentColor">
      ${fields.map((f) => {
        const seg = `
          <g transform="translate(${x} 80)" filter="url(#rough)">
            <rect x="0" y="0" width="${f.w}" height="70" rx="5" fill="none" stroke="currentColor" stroke-width="${f.tone === "primary" ? 1.6 : 1}" ${f.tone === "primary" ? `stroke="var(--primary)"` : ""}/>
            <text x="${f.w / 2}" y="32" text-anchor="middle" font-size="12" font-weight="600">${f.name}</text>
            <text x="${f.w / 2}" y="50" text-anchor="middle" font-size="10.5" fill="var(--muted)">${f.size}</text>
          </g>
          <line x1="${x}" y1="170" x2="${x + f.w}" y2="170" stroke="currentColor" stroke-opacity=".25"/>
          <text x="${x + f.w / 2}" y="186" text-anchor="middle" font-size="10" fill="var(--muted)" font-family="JetBrains Mono, monospace">offset → ${x - 40}</text>
        `;
        x += f.w + 8;
        return seg;
      }).join("")}
      <g transform="translate(40 30)">
        <text x="0" y="0" font-size="11" fill="var(--muted)" letter-spacing="1.5">BLOCK HEADER · LITTLE-ENDIAN</text>
      </g>
      <g transform="translate(40 210)" font-size="11" fill="var(--muted)">
        <text x="0" y="0">checksum is computed over the <tspan fill="currentColor">uncompressed</tspan> payload — integrity verified after decompression.</text>
      </g>
    </g>
  `;
  return wrap("Block header anatomy", "Spec · §2.1", "0 0 720 240", body, 280);
}

/* ───────────────── SPEC: object encoding ───────────────── */
function objectEncoding() {
  const body = `
    <g font-family="JetBrains Mono, monospace" fill="currentColor">
      <g transform="translate(40 60)" filter="url(#rough)">
        <rect x="0" y="0" width="56" height="44" rx="4" fill="none" stroke="var(--primary)" stroke-width="1.4"/>
        <text x="28" y="20" text-anchor="middle" font-size="11" fill="var(--primary)">0x06</text>
        <text x="28" y="36" text-anchor="middle" font-size="10" fill="var(--muted)">Start</text>
      </g>
      <g transform="translate(110 60)" filter="url(#rough)">
        <rect x="0" y="0" width="60" height="44" rx="4" fill="none" stroke="currentColor"/>
        <text x="30" y="20" text-anchor="middle" font-size="11">count</text>
        <text x="30" y="36" text-anchor="middle" font-size="10" fill="var(--muted)">varint</text>
      </g>
      ${[0, 1, 2].map((i) => `
        <g transform="translate(${190 + i * 170} 50)" filter="url(#rough)">
          <rect x="0" y="0" width="160" height="66" rx="6" fill="none" stroke="currentColor"/>
          <text x="80" y="22" text-anchor="middle" font-size="11" fill="var(--muted)">field ${i}</text>
          <line x1="14" y1="32" x2="146" y2="32" stroke="currentColor" stroke-opacity=".25"/>
          <text x="80" y="48" text-anchor="middle" font-size="11">key_len · key</text>
          <text x="80" y="62" text-anchor="middle" font-size="10.5" fill="var(--muted)">value (wire-encoded)</text>
        </g>
      `).join("")}
      <g transform="translate(700 60)" filter="url(#rough)">
        <rect x="0" y="0" width="56" height="44" rx="4" fill="none" stroke="var(--primary)" stroke-width="1.4"/>
        <text x="28" y="20" text-anchor="middle" font-size="11" fill="var(--primary)">0x07</text>
        <text x="28" y="36" text-anchor="middle" font-size="10" fill="var(--muted)">End</text>
      </g>
      <text x="40" y="30" font-size="11" fill="var(--muted)" letter-spacing="1.5">OBJECT · BRACKETED STREAM</text>
      <text x="40" y="150" font-size="11" fill="var(--muted)">Fields are emitted in insertion order; keys are UTF-8, never length-prefixed by type.</text>
    </g>
  `;
  return wrap("Object encoding", "Spec · §3.2", "0 0 780 170", body, 200);
}

/* ───────────────── SPEC: array encoding ───────────────── */
function arrayEncoding() {
  const body = `
    <g font-family="JetBrains Mono, monospace" fill="currentColor">
      <g transform="translate(40 60)" filter="url(#rough)">
        <rect x="0" y="0" width="56" height="44" rx="4" fill="none" stroke="var(--primary)" stroke-width="1.4"/>
        <text x="28" y="20" text-anchor="middle" font-size="11" fill="var(--primary)">0x08</text>
        <text x="28" y="36" text-anchor="middle" font-size="10" fill="var(--muted)">Start</text>
      </g>
      <g transform="translate(110 60)" filter="url(#rough)">
        <rect x="0" y="0" width="60" height="44" rx="4" fill="none" stroke="currentColor"/>
        <text x="30" y="20" text-anchor="middle" font-size="11">count</text>
        <text x="30" y="36" text-anchor="middle" font-size="10" fill="var(--muted)">varint</text>
      </g>
      ${[0, 1, 2, 3].map((i) => `
        <g transform="translate(${190 + i * 124} 60)" filter="url(#rough)">
          <rect x="0" y="0" width="112" height="44" rx="5" fill="none" stroke="currentColor"/>
          <text x="56" y="20" text-anchor="middle" font-size="11">value ${i}</text>
          <text x="56" y="36" text-anchor="middle" font-size="10" fill="var(--muted)">wire-encoded</text>
        </g>
      `).join("")}
      <g transform="translate(700 60)" filter="url(#rough)">
        <rect x="0" y="0" width="56" height="44" rx="4" fill="none" stroke="var(--primary)" stroke-width="1.4"/>
        <text x="28" y="20" text-anchor="middle" font-size="11" fill="var(--primary)">0x09</text>
        <text x="28" y="36" text-anchor="middle" font-size="10" fill="var(--muted)">End</text>
      </g>
      <text x="40" y="30" font-size="11" fill="var(--muted)" letter-spacing="1.5">ARRAY · HOMOGENEOUS OR MIXED</text>
    </g>
  `;
  return wrap("Array encoding", "Spec · §3.3", "0 0 780 140", body, 170);
}

/* ───────────────── SPEC: stringdict prefix-delta ───────────────── */
function stringDict() {
  const rows = [
    { prev: "—", curr: "config_cache_host",       pre: 0,  suf: "config_cache_host" },
    { prev: "config_cache_host", curr: "config_cache_port", pre: 13, suf: "port" },
    { prev: "config_cache_port", curr: "config_database_host", pre: 7, suf: "database_host" },
    { prev: "config_database_host", curr: "config_database_port", pre: 16, suf: "port" },
  ];
  const body = `
    <g font-family="JetBrains Mono, monospace" fill="currentColor" font-size="11">
      <text x="40" y="28" font-size="11" fill="var(--muted)" letter-spacing="1.5">STRINGDICT · PREFIX-DELTA</text>
      <g transform="translate(40 50)">
        <text x="0" y="0" fill="var(--muted)">idx</text>
        <text x="44" y="0" fill="var(--muted)">prefix_len</text>
        <text x="140" y="0" fill="var(--muted)">suffix</text>
        <text x="430" y="0" fill="var(--muted)">reconstructed</text>
        <line x1="0" y1="8" x2="700" y2="8" stroke="currentColor" stroke-opacity=".2"/>
      </g>
      ${rows.map((r, i) => `
        <g transform="translate(40 ${78 + i * 44})">
          <text x="0" y="14" fill="var(--primary)">${i}</text>
          <g transform="translate(44 0)">
            <rect x="0" y="-2" width="64" height="22" rx="3" fill="none" stroke="currentColor" stroke-opacity=".35"/>
            <text x="32" y="14" text-anchor="middle">${r.pre}</text>
          </g>
          <g transform="translate(140 0)">
            <rect x="0" y="-2" width="${Math.max(60, r.suf.length * 8)}" height="22" rx="3" fill="none" stroke="var(--primary)" stroke-opacity=".55"/>
            <text x="10" y="14" fill="var(--primary)">${r.suf}</text>
          </g>
          <text x="430" y="14">${r.curr}</text>
        </g>
      `).join("")}
      <text x="40" y="270" font-size="11" fill="var(--muted)">Entries are sorted lexicographically before encoding; each row stores only the byte-delta from the prior key.</text>
    </g>
  `;
  return wrap("StringDict prefix-delta", "Spec · §8.3", "0 0 760 290", body, 320);
}

/* ───────────────── RFC: high-level ───────────────── */
function rfcHighLevel() {
  const body = `
    <g font-family="JetBrains Mono, monospace" fill="currentColor">
      <text x="40" y="30" font-size="11" fill="var(--muted)" letter-spacing="1.5">RFC-001 · HIGH-LEVEL TOPOLOGY</text>

      <g transform="translate(60 80)">
        ${["CTN source", "Rust caller", "Python caller", "CLI"].map((s, i) => `
          <g transform="translate(0 ${i * 56})" filter="url(#rough)">
            <rect x="0" y="0" width="170" height="46" rx="6" fill="none" stroke="currentColor"/>
            <text x="14" y="20" font-size="12" font-weight="600">${s}</text>
            <text x="14" y="36" font-size="10.5" fill="var(--muted)">producer surface</text>
          </g>
        `).join("")}
      </g>

      <g transform="translate(290 130)" filter="url(#rough)">
        <rect x="0" y="0" width="200" height="170" rx="10" fill="none" stroke="var(--primary)" stroke-width="1.6"/>
        <text x="100" y="26" text-anchor="middle" font-size="11" fill="var(--primary)" letter-spacing="1.5">CORE</text>
        <text x="100" y="52" text-anchor="middle" font-size="13.5" font-weight="600">surp-ctn</text>
        <text x="100" y="72" text-anchor="middle" font-size="10.5" fill="var(--muted)">canonical tree normalisation</text>
        <line x1="22" y1="86" x2="178" y2="86" stroke="currentColor" stroke-opacity=".25"/>
        <text x="100" y="106" text-anchor="middle" font-size="13.5" font-weight="600">surp-cbf</text>
        <text x="100" y="126" text-anchor="middle" font-size="10.5" fill="var(--muted)">canonical binary framing</text>
        <line x1="22" y1="138" x2="178" y2="138" stroke="currentColor" stroke-opacity=".25"/>
        <text x="100" y="156" text-anchor="middle" font-size="13.5" font-weight="600">surp-cql</text>
      </g>

      <g transform="translate(550 80)">
        ${[".surp file", "stream sink", "mmap reader", "agent timeline"].map((s, i) => `
          <g transform="translate(0 ${i * 56})" filter="url(#rough)">
            <rect x="0" y="0" width="170" height="46" rx="6" fill="none" stroke="currentColor" stroke-dasharray="${i === 3 ? "4 3" : "0"}"/>
            <text x="14" y="20" font-size="12" font-weight="600">${s}</text>
            <text x="14" y="36" font-size="10.5" fill="var(--muted)">${i === 3 ? "downstream consumer" : "byte sink"}</text>
          </g>
        `).join("")}
      </g>

      <g stroke="currentColor" stroke-opacity=".45" fill="none" marker-end="url(#arrow)">
        ${[0,1,2,3].map(i => `<path d="M 230 ${103 + i*56} C 260 ${103 + i*56}, 280 215, 290 215"/>`).join("")}
        ${[0,1,2,3].map(i => `<path d="M 490 215 C 510 215, 530 ${103 + i*56}, 550 ${103 + i*56}"/>`).join("")}
      </g>

      <text x="40" y="395" font-size="11" fill="var(--muted)">Producers normalise into CTN, frame through CBF, query through CQL — every surface speaks the same canonical tree.</text>
    </g>
  `;
  return wrap("CTN · CBF · CQL — system view", "RFC-001 · high-level", "0 0 760 420", body, 460);
}

/* ───────────────── RFC: low-level ───────────────── */
function rfcLowLevel() {
  const stages = [
    { x: 30,  label: "Parse",        sub: "tokens → AST" },
    { x: 170, label: "Normalise",    sub: "CTN ordering" },
    { x: 310, label: "Stabilise",    sub: "id assignment" },
    { x: 450, label: "Frame",        sub: "CBF blocks" },
    { x: 590, label: "Sign",         sub: "XXH64 trailer" },
  ];
  const body = `
    <g font-family="JetBrains Mono, monospace" fill="currentColor">
      <text x="40" y="30" font-size="11" fill="var(--muted)" letter-spacing="1.5">RFC-001 · ENCODE PIPELINE</text>

      <g transform="translate(40 70)">
        ${stages.map((s, i) => `
          <g transform="translate(${s.x} 0)" filter="url(#rough)">
            <circle cx="44" cy="44" r="36" fill="none" stroke="${i === 4 ? "var(--primary)" : "currentColor"}" stroke-width="${i === 4 ? 1.6 : 1}"/>
            <text x="44" y="42" text-anchor="middle" font-size="12" font-weight="600">${s.label}</text>
            <text x="44" y="58" text-anchor="middle" font-size="10" fill="var(--muted)">${s.sub}</text>
            <text x="44" y="100" text-anchor="middle" font-size="10" fill="var(--muted)">stage ${i + 1}</text>
          </g>
          ${i < stages.length - 1 ? `<line x1="${s.x + 80}" y1="44" x2="${stages[i+1].x + 8}" y2="44" stroke="currentColor" stroke-opacity=".5" marker-end="url(#arrow)"/>` : ""}
        `).join("")}
      </g>

      <g transform="translate(40 200)" font-size="11">
        <text x="0" y="0" fill="var(--muted)" letter-spacing="1.5">INVARIANTS</text>
        ${[
          "key order is byte-lexicographic, not insertion",
          "floats are normalised to canonical f64 (no -0, no NaN payload bits)",
          "integers ride the smallest varint that fits — no padding",
          "string dedup is per-block; never cross-block",
          "trailer covers every byte that preceded it",
        ].map((s, i) => `
          <g transform="translate(0 ${24 + i * 22})">
            <circle cx="4" cy="-4" r="3" fill="var(--primary)"/>
            <text x="18" y="0">${s}</text>
          </g>
        `).join("")}
      </g>
    </g>
  `;
  return wrap("Encode pipeline · low-level", "RFC-001 · low-level", "0 0 760 360", body, 400);
}

/* ───────────────── Rust API architecture ───────────────── */
function rustArchitecture() {
  const crates = [
    { x: 30,  y: 80,  w: 170, h: 60, name: "surp",            sub: "umbrella · re-exports" },
    { x: 30,  y: 170, w: 170, h: 60, name: "surp-derive",     sub: "#[derive(Surp)]" },
    { x: 240, y: 50,  w: 200, h: 90, name: "surp-core",       sub: "encoder · decoder · types", primary: true },
    { x: 240, y: 170, w: 200, h: 60, name: "surp-io",         sub: "Read/Write · MmapReader" },
    { x: 240, y: 260, w: 200, h: 60, name: "surp-compression",sub: "lz4 · zstd · snappy · adaptive" },
    { x: 480, y: 80,  w: 170, h: 60, name: "surp-simd",       sub: "NEON varint pre-scan" },
    { x: 480, y: 170, w: 170, h: 60, name: "surp-bench",      sub: "regression harness" },
    { x: 480, y: 260, w: 170, h: 60, name: "surp-cli",        sub: "inspect · pretty · …" },
  ];
  const edges = [
    [0, 2], [0, 3], [1, 0], [2, 3], [2, 4], [2, 5], [3, 6], [3, 7], [2, 7],
  ];
  const body = `
    <g font-family="JetBrains Mono, monospace" fill="currentColor">
      <text x="40" y="30" font-size="11" fill="var(--muted)" letter-spacing="1.5">RUST · WORKSPACE GRAPH</text>
      ${crates.map((c) => `
        <g transform="translate(${c.x} ${c.y})" filter="url(#rough)">
          <rect x="0" y="0" width="${c.w}" height="${c.h}" rx="8" fill="none"
            stroke="${c.primary ? "var(--primary)" : "currentColor"}"
            stroke-width="${c.primary ? 1.6 : 1}"/>
          <text x="${c.w / 2}" y="${c.h / 2 - 2}" text-anchor="middle" font-size="13" font-weight="600">${c.name}</text>
          <text x="${c.w / 2}" y="${c.h / 2 + 16}" text-anchor="middle" font-size="10.5" fill="var(--muted)">${c.sub}</text>
        </g>
      `).join("")}
      <g stroke="currentColor" stroke-opacity=".4" fill="none" marker-end="url(#arrow)">
        ${edges.map(([a, b]) => {
          const A = crates[a]; const B = crates[b];
          const ax = A.x + A.w / 2; const ay = A.y + A.h / 2;
          const bx = B.x + B.w / 2; const by = B.y + B.h / 2;
          return `<path d="M ${ax} ${ay} Q ${(ax+bx)/2} ${(ay+by)/2 - 24} ${bx} ${by}"/>`;
        }).join("")}
      </g>
      <text x="40" y="360" font-size="11" fill="var(--muted)">Solid edges follow the actual <tspan fill="currentColor">Cargo.toml</tspan> dependency graph; surp-core is the only crate every consumer must touch.</text>
    </g>
  `;
  return wrap("Rust workspace architecture", "Rust API · structure", "0 0 700 380", body, 420);
}

/* ───────────────── Python API architecture ───────────────── */
function pythonArchitecture() {
  const body = `
    <g font-family="JetBrains Mono, monospace" fill="currentColor">
      <text x="40" y="30" font-size="11" fill="var(--muted)" letter-spacing="1.5">PYTHON · BINDING STACK</text>

      <g transform="translate(40 70)" filter="url(#rough)">
        <rect x="0" y="0" width="680" height="58" rx="8" fill="none" stroke="currentColor"/>
        <text x="20" y="24" font-size="13" font-weight="600">Python surface</text>
        <text x="20" y="42" font-size="10.5" fill="var(--muted)">surp.encode · surp.decode · surp.Encoder · surp.Decoder · surp.dumps / loads · context managers</text>
      </g>

      <g transform="translate(120 150)" filter="url(#rough)">
        <rect x="0" y="0" width="240" height="64" rx="8" fill="none" stroke="var(--primary)" stroke-width="1.6"/>
        <text x="120" y="24" text-anchor="middle" font-size="13" font-weight="600" fill="var(--primary)">PyO3 bindings</text>
        <text x="120" y="44" text-anchor="middle" font-size="10.5" fill="var(--muted)">_surp.cpython-*.so · zero-copy buffers</text>
      </g>
      <g transform="translate(400 150)" filter="url(#rough)">
        <rect x="0" y="0" width="240" height="64" rx="8" fill="none" stroke="currentColor"/>
        <text x="120" y="24" text-anchor="middle" font-size="13" font-weight="600">maturin wheels</text>
        <text x="120" y="44" text-anchor="middle" font-size="10.5" fill="var(--muted)">manylinux · macOS arm64 · Windows x64</text>
      </g>

      <g transform="translate(40 240)" filter="url(#rough)">
        <rect x="0" y="0" width="680" height="58" rx="8" fill="none" stroke="currentColor"/>
        <text x="20" y="24" font-size="13" font-weight="600">surp-core (Rust)</text>
        <text x="20" y="42" font-size="10.5" fill="var(--muted)">Encoder / Decoder · varint · checksums · dedup · same code path the Rust API uses</text>
      </g>

      <g stroke="currentColor" stroke-opacity=".45" fill="none" marker-end="url(#arrow)">
        <path d="M 240 128 L 240 150"/>
        <path d="M 520 128 L 520 150"/>
        <path d="M 240 214 L 240 240"/>
        <path d="M 520 214 L 520 240"/>
      </g>

      <text x="40" y="335" font-size="11" fill="var(--muted)">Calling <tspan fill="currentColor">surp.encode(obj)</tspan> bottoms out in the same Rust encoder that <tspan fill="currentColor">cargo run -p surp-cli</tspan> uses — no second implementation.</text>
    </g>
  `;
  return wrap("Python binding stack", "Python API · structure", "0 0 760 360", body, 400);
}

/* ───────────────── registry ───────────────── */
const REGISTRY: Record<string, () => string> = {
  "file-layout": fileLayout,
  "block-header": blockHeader,
  "object-encoding": objectEncoding,
  "array-encoding": arrayEncoding,
  "stringdict": stringDict,
  "rfc-high-level": rfcHighLevel,
  "rfc-low-level": rfcLowLevel,
  "rust-architecture": rustArchitecture,
  "python-architecture": pythonArchitecture,
};

export function diagramHTML(name: string): string | null {
  const fn = REGISTRY[name.trim()];
  return fn ? fn() : null;
}
