import { useChat } from "@ai-sdk/react";
import { DefaultChatTransport, type UIMessage } from "ai";
import { useEffect, useMemo, useRef, useState } from "react";
import { marked, type Tokens } from "marked";
import { I } from "@/lib/icons";
import { highlight } from "@/lib/highlight";

const TRANSPORT = new DefaultChatTransport({ api: "/api/chat" });

const SUGGESTIONS: { q: string; tag: string }[] = [
  { q: "What is Surp in one paragraph?", tag: "intro" },
  { q: "Show me a Rust encode/decode example", tag: "rust" },
  { q: "Surp vs JSON vs MessagePack — when do I pick which?", tag: "compare" },
  { q: "Explain the v1 block framing", tag: "spec" },
  { q: "Walk me through the string-dedup win on string_heavy", tag: "bench" },
  { q: "How does the CLI inspect a .surp file?", tag: "cli" },
];

function partsToText(parts: UIMessage["parts"]): string {
  return parts.map((p) => (p.type === "text" ? p.text : "")).join("");
}

/* ── Markdown renderer with custom code blocks via tokenizer ── */
type MdNode = { kind: "html"; html: string } | { kind: "code"; lang: string; code: string };

function tokenize(md: string): MdNode[] {
  const tokens = marked.lexer(md);
  const nodes: MdNode[] = [];
  const walk = (toks: Tokens.Generic[]) => {
    let buf: Tokens.Generic[] = [];
    const flush = () => {
      if (!buf.length) return;
      nodes.push({ kind: "html", html: marked.parser(buf as never) });
      buf = [];
    };
    for (const t of toks) {
      if (t.type === "code") {
        flush();
        nodes.push({ kind: "code", lang: (t as Tokens.Code).lang || "text", code: (t as Tokens.Code).text });
      } else {
        buf.push(t);
      }
    }
    flush();
  };
  walk(tokens as Tokens.Generic[]);
  return nodes;
}

function CodeBlock({ lang, code }: { lang: string; code: string }) {
  const [copied, setCopied] = useState(false);
  const html = useMemo(() => highlight(code, lang), [code, lang]);
  return (
    <div className="ai-code my-2">
      <div className="ai-code-head">
        <span>{lang || "text"}</span>
        <button
          className={`ai-code-copy ${copied ? "copied" : ""}`}
          onClick={() => {
            navigator.clipboard?.writeText(code);
            setCopied(true);
            setTimeout(() => setCopied(false), 1400);
          }}
          aria-label="Copy code"
        >
          {copied ? "Copied" : "Copy"}
        </button>
      </div>
      <pre><code dangerouslySetInnerHTML={{ __html: html }} /></pre>
    </div>
  );
}

function Markdown({ source, streaming }: { source: string; streaming?: boolean }) {
  const nodes = useMemo(() => {
    try { return tokenize(source); } catch { return [{ kind: "html", html: source } as MdNode]; }
  }, [source]);
  return (
    <div className={`ai-md ${streaming ? "ai-caret" : ""}`}>
      {nodes.map((n, i) =>
        n.kind === "code" ? (
          <CodeBlock key={i} lang={n.lang} code={n.code} />
        ) : (
          <div key={i} dangerouslySetInnerHTML={{ __html: n.html }} />
        ),
      )}
    </div>
  );
}

/* ── Component ── */
export function AskAI() {
  const [open, setOpen] = useState(false);
  const [input, setInput] = useState("");
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);

  const { messages, sendMessage, status, error, stop, setMessages } = useChat({
    id: "surp-docs-assistant",
    transport: TRANSPORT,
  });

  useEffect(() => {
    if (open) setTimeout(() => inputRef.current?.focus(), 60);
  }, [open]);

  useEffect(() => {
    const el = scrollRef.current;
    if (el) el.scrollTo({ top: el.scrollHeight, behavior: "smooth" });
  }, [messages, status]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && open) setOpen(false);
      if ((e.metaKey || e.ctrlKey) && e.key === "/") {
        e.preventDefault();
        setOpen((v) => !v);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open]);

  const busy = status === "submitted" || status === "streaming";

  async function submit(text: string) {
    const value = text.trim();
    if (!value || busy) return;
    setInput("");
    await sendMessage({ text: value });
  }

  return (
    <>
      <button
        aria-label="Ask AI about Surp"
        onClick={() => setOpen((v) => !v)}
        style={{ left: "auto" }}
        className="fixed z-40 bottom-5 right-5 sm:bottom-6 sm:right-6 h-12 px-4 inline-flex items-center gap-2 rounded-full bg-ink text-canvas shadow-[0_8px_24px_-8px_rgba(0,0,0,0.45)] hover:translate-y-[-1px] active:translate-y-0 transition-transform"
      >
        <span className="relative inline-flex h-2 w-2">
          <span className="relative inline-flex h-2 w-2 rounded-full bg-primary" />
        </span>
        <span className="text-[13px] font-medium tracking-tight">Ask AI</span>
        <span className="hidden sm:inline text-[10px] font-mono px-1.5 py-0.5 rounded border border-canvas/20 text-canvas/60">⌘/</span>
      </button>

      {open && (
        <div
          style={{ left: "auto", right: "16px" }}
          className="ai-pop fixed z-50 bottom-20 sm:right-6 w-[min(94vw,440px)] h-[min(80vh,680px)] flex flex-col rounded-2xl border border-hairline-strong bg-canvas shadow-[0_24px_80px_-20px_rgba(0,0,0,0.5)] overflow-hidden"

          role="dialog"
          aria-label="Surp docs assistant"
        >
          {/* Header */}
          <div className="relative flex items-center gap-3 px-4 py-3 border-b border-hairline bg-canvas-soft">
            <div className="relative h-9 w-9 rounded-lg bg-ink text-canvas inline-flex items-center justify-center font-display text-[17px] shadow-[inset_0_0_0_1px_rgba(255,255,255,0.06)]">
              S
              <span className="absolute -bottom-0.5 -right-0.5 h-2.5 w-2.5 rounded-full bg-primary border-2 border-canvas-soft" />
            </div>
            <div className="min-w-0">
              <div className="text-[13px] font-semibold text-ink leading-tight">Surp Docs Assistant</div>
              <div className="text-[11px] text-muted leading-tight flex items-center gap-1.5">
                <span className="inline-block h-1.5 w-1.5 rounded-full bg-success" />
                Grounded · v1.2.0 · streaming
              </div>
            </div>
            <div className="ml-auto flex items-center gap-1">
              {messages.length > 0 && (
                <button
                  onClick={() => setMessages([])}
                  className="h-8 px-2 rounded-md inline-flex items-center text-[11px] text-muted hover:text-ink hover:bg-surface-strong"
                  aria-label="Clear conversation"
                >
                  Clear
                </button>
              )}
              <button
                onClick={() => setOpen(false)}
                className="h-8 w-8 rounded-md inline-flex items-center justify-center text-muted hover:text-ink hover:bg-surface-strong"
                aria-label="Close"
              >
                <I.Close />
              </button>
            </div>
          </div>

          {/* Messages */}
          <div ref={scrollRef} className="ai-scroll flex-1 overflow-y-auto px-4 py-4 space-y-4">
            {messages.length === 0 && (
              <div className="ai-msg">
                <div className="rounded-xl border border-hairline bg-surface p-4">
                  <div className="flex items-center gap-2 text-[11px] eyebrow text-primary">
                    <I.Spark width={12} height={12} /> Ask anything about Surp
                  </div>
                  <p className="mt-2 text-[13.5px] text-body leading-relaxed">
                    I'm grounded in the v1 spec, RFC-001, the Rust + Python APIs, the CLI, the MCP server, and the committed benchmarks. I won't invent flags or APIs.
                  </p>
                </div>
                <div className="mt-3 grid gap-1.5">
                  {SUGGESTIONS.map((s, i) => (
                    <button
                      key={s.q}
                      onClick={() => submit(s.q)}
                      style={{ animationDelay: `${i * 40}ms` }}
                      className="ai-msg group text-left text-[12.5px] pl-3 pr-2 py-2 rounded-md border border-hairline hover:border-primary/60 hover:bg-canvas-soft text-body hover:text-ink flex items-center justify-between gap-2 transition-colors"
                    >
                      <span className="flex items-center gap-2 min-w-0">
                        <span className="font-mono text-[9.5px] text-muted-soft uppercase tracking-wider w-12 shrink-0">{s.tag}</span>
                        <span className="truncate">{s.q}</span>
                      </span>
                      <I.Arrow width={11} height={11} />
                    </button>
                  ))}
                </div>
              </div>
            )}

            {messages.map((m, idx) => {
              const text = partsToText(m.parts);
              const isLast = idx === messages.length - 1;
              const isStreaming = isLast && m.role === "assistant" && status === "streaming";
              if (m.role === "user") {
                return (
                  <div key={m.id} className="ai-msg flex justify-end">
                    <div className="max-w-[88%] rounded-2xl rounded-br-md bg-ink text-canvas px-3.5 py-2 text-[13.5px] leading-relaxed whitespace-pre-wrap shadow-[0_2px_8px_-4px_rgba(0,0,0,0.3)]">
                      {text}
                    </div>
                  </div>
                );
              }
              return (
                <div key={m.id} className="ai-msg group">
                  <div className="flex items-center gap-2 mb-1.5">
                    <div className="h-5 w-5 rounded-md bg-primary/10 text-primary inline-flex items-center justify-center text-[10px] font-display">S</div>
                    <span className="text-[10px] eyebrow text-muted">Assistant</span>
                    {text && (
                      <button
                        onClick={() => {
                          navigator.clipboard?.writeText(text);
                          setCopiedId(m.id);
                          setTimeout(() => setCopiedId(null), 1200);
                        }}
                        className="ml-auto opacity-0 group-hover:opacity-100 text-[10px] text-muted hover:text-ink transition-opacity inline-flex items-center gap-1"
                        aria-label="Copy reply"
                      >
                        <I.Copy width={10} height={10} /> {copiedId === m.id ? "Copied" : "Copy"}
                      </button>
                    )}
                  </div>
                  {text ? (
                    <Markdown source={text} streaming={isStreaming} />
                  ) : (
                    <div className="text-[12.5px] ai-shimmer font-medium">Thinking…</div>
                  )}
                </div>
              );
            })}

            {status === "submitted" && (
              <div className="ai-msg flex items-center gap-2 text-[12px] text-muted">
                <span className="inline-flex">
                  <span className="ai-dot" /><span className="ai-dot" /><span className="ai-dot" />
                </span>
                <span className="ai-shimmer font-medium">Reading the spec…</span>
              </div>
            )}

            {error && (
              <div className="ai-msg text-[12px] text-primary border border-primary/30 bg-primary/5 rounded-md px-3 py-2">
                Couldn't reach the model. Check your connection and try again.
              </div>
            )}
          </div>

          {/* Composer */}
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void submit(input);
            }}
            className="border-t border-hairline bg-canvas-soft p-2.5"
          >
            <div className="flex items-end gap-2 rounded-xl border border-hairline bg-canvas px-2.5 py-2 focus-within:border-primary/60 focus-within:shadow-[0_0_0_3px_color-mix(in_oklab,var(--primary)_12%,transparent)] transition-shadow">
              <textarea
                ref={inputRef}
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && !e.shiftKey) {
                    e.preventDefault();
                    void submit(input);
                  }
                }}
                rows={1}
                placeholder="Ask about the spec, the API, the CLI…"
                className="flex-1 resize-none bg-transparent outline-none text-[13.5px] text-ink placeholder:text-muted-soft max-h-40 leading-relaxed"
              />
              {busy ? (
                <button
                  type="button"
                  onClick={() => stop()}
                  className="h-8 w-8 inline-flex items-center justify-center rounded-md bg-surface-strong text-ink hover:bg-canvas-soft border border-hairline"
                  aria-label="Stop"
                >
                  <span className="w-2.5 h-2.5 bg-ink rounded-[2px]" />
                </button>
              ) : (
                <button
                  type="submit"
                  disabled={!input.trim()}
                  className="h-8 w-8 inline-flex items-center justify-center rounded-md bg-primary text-on-primary disabled:opacity-30 disabled:cursor-not-allowed hover:brightness-110 active:scale-95 transition-all"
                  aria-label="Send"
                >
                  <I.Arrow width={14} height={14} />
                </button>
              )}
            </div>
            <div className="mt-1.5 flex items-center justify-between px-1">
              <p className="text-[10.5px] text-muted-soft">
                Grounded in Surp docs · may still be wrong, verify the spec
              </p>
              <p className="text-[10px] text-muted-soft font-mono hidden sm:block">
                ↵ send · ⇧↵ newline
              </p>
            </div>
          </form>
        </div>
      )}
    </>
  );
}
