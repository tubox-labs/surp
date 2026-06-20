// Tiny, dependency-free syntax tokenizer with VS-Code-like coloring.
// Supports: rust, python, bash/shell, toml, json, ctn, surp (text notation).
// Not a full parser — intentionally regex-driven, defensive, and fast.

type Lang =
  | "rust"
  | "python"
  | "py"
  | "bash"
  | "sh"
  | "shell"
  | "console"
  | "toml"
  | "json"
  | "ctn"
  | "surp"
  | "text"
  | "";

const KW: Record<string, string[]> = {
  rust: "as break const continue crate else enum extern false fn for if impl in let loop match mod move mut pub ref return self Self static struct super trait true type unsafe use where while async await dyn".split(" "),
  python: "False None True and as assert async await break class continue def del elif else except finally for from global if import in is lambda nonlocal not or pass raise return try while with yield match case".split(" "),
  bash: "if then else fi for in do done while case esac function return exit export local readonly source".split(" "),
  toml: [],
  json: ["true", "false", "null"],
  ctn: "let map seq tensor stream sum product symbol enum ref doc".split(" "),
  surp: "true false null inf -inf NaN".split(" "),
};

const esc = (s: string) =>
  s.replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]!);

function normalizeLang(lang: string): keyof typeof KW | "text" {
  const l = (lang || "").toLowerCase();
  if (l === "py") return "python";
  if (l === "sh" || l === "shell" || l === "console" || l === "bash") return "bash";
  if (l === "rs") return "rust";
  if (l in KW) return l as keyof typeof KW;
  return "text";
}

export function highlight(code: string, langRaw: string): string {
  const lang = normalizeLang(langRaw);
  if (lang === "text") return esc(code);

  // Token spec — order matters
  const patterns: Array<[RegExp, string]> = [];

  // Comments
  if (lang === "rust") {
    patterns.push([/\/\/[^\n]*/g, "c"]);
    patterns.push([/\/\*[\s\S]*?\*\//g, "c"]);
  } else if (lang === "python") {
    patterns.push([/#[^\n]*/g, "c"]);
    patterns.push([/"""[\s\S]*?"""|'''[\s\S]*?'''/g, "s"]);
  } else if (lang === "bash") {
    patterns.push([/#[^\n]*/g, "c"]);
  } else if (lang === "ctn" || lang === "surp") {
    patterns.push([/\/\/[^\n]*/g, "c"]);
    patterns.push([/\/\*[\s\S]*?\*\//g, "c"]);
  } else if (lang === "toml") {
    patterns.push([/#[^\n]*/g, "c"]);
  }

  // Strings
  patterns.push([/"(?:\\.|[^"\\\n])*"/g, "s"]);
  patterns.push([/'(?:\\.|[^'\\\n])*'/g, "s"]);
  if (lang === "rust") patterns.push([/b?"(?:\\.|[^"\\])*"/g, "s"]);

  // Numbers
  patterns.push([/\b0[xX][0-9a-fA-F_]+\b/g, "n"]);
  patterns.push([/\b\d[\d_]*(?:\.\d+)?(?:[eE][+-]?\d+)?\b/g, "n"]);

  // Bash: flags & paths
  if (lang === "bash") {
    patterns.push([/(^|\s)(-{1,2}[A-Za-z][\w-]*)/g, "k"]);
    patterns.push([/\$\{?[A-Za-z_][\w]*\}?/g, "t"]);
  }

  // Rust types / Python builtins
  if (lang === "rust") {
    patterns.push([/\b[A-Z][A-Za-z0-9_]*\b/g, "t"]);
    patterns.push([/\b([a-z_][\w]*)\s*\(/g, "f"]);
    patterns.push([/#\[[^\]]*\]/g, "m"]);
    patterns.push([/&'?[a-z_]+/g, "m"]);
  } else if (lang === "python") {
    patterns.push([/\b([a-zA-Z_][\w]*)\s*\(/g, "f"]);
    patterns.push([/@[A-Za-z_][\w]*/g, "m"]);
  }

  // TOML headers + keys
  if (lang === "toml") {
    patterns.push([/^\[[^\]\n]+\]/gm, "t"]);
    patterns.push([/^[A-Za-z_][\w-]*(?=\s*=)/gm, "f"]);
  }

  // Punctuation
  patterns.push([/[{}()\[\];,:]/g, "p"]);

  // Tokenize: collect non-overlapping matches.
  type Hit = { start: number; end: number; cls: string; text: string };
  const hits: Hit[] = [];
  for (const [re, cls] of patterns) {
    let m: RegExpExecArray | null;
    re.lastIndex = 0;
    while ((m = re.exec(code))) {
      // For patterns with leading whitespace capture, use the inner group
      const start = m.index + (m[0].length - (m[m.length - 1]?.length ?? m[0].length));
      const text = m[m.length === 1 ? 0 : m.length - 1] ?? m[0];
      const realStart = code.indexOf(text, m.index);
      const s = realStart >= 0 ? realStart : m.index;
      hits.push({ start: s, end: s + text.length, cls, text });
      if (m[0].length === 0) re.lastIndex++;
    }
  }
  hits.sort((a, b) => a.start - b.start || b.end - a.end);
  const filtered: Hit[] = [];
  let cursor = 0;
  for (const h of hits) {
    if (h.start < cursor) continue;
    filtered.push(h);
    cursor = h.end;
  }

  // Render
  let out = "";
  let i = 0;
  const kws = new Set(KW[lang] || []);
  const emit = (chunk: string) => {
    // Highlight keywords inside plain chunks
    if (!kws.size) return esc(chunk);
    return esc(chunk).replace(
      new RegExp(`\\b(${[...kws].join("|")})\\b`, "g"),
      '<span class="tok-k">$1</span>',
    );
  };
  for (const h of filtered) {
    if (i < h.start) out += emit(code.slice(i, h.start));
    out += `<span class="tok-${h.cls}">${esc(h.text)}</span>`;
    i = h.end;
  }
  if (i < code.length) out += emit(code.slice(i));
  return out;
}
