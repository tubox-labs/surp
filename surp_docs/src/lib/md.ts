import { marked, type Tokens } from "marked";
import { highlight } from "./highlight";
import { diagramHTML } from "./diagrams-html";

const slug = (s: string) =>
  s
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, "")
    .trim()
    .replace(/\s+/g, "-");

const renderer = new marked.Renderer();

renderer.code = ({ text, lang }: Tokens.Code) => {
  const language = (lang || "").trim() || "text";
  if (language === "surp-diagram") {
    const name = text.trim().split("\n")[0]?.trim() ?? "";
    const html = diagramHTML(name);
    if (html) return html;
  }
  const lines = text.split("\n");
  const numbered = lines
    .map((_, i) => `<span>${i + 1}</span>`)
    .join("\n");
  const highlighted = highlight(text, language);
  const label = language === "text" ? "" : language;
  return `<div class="codeblock" data-lang="${language}">
    <div class="codeblock-head"><span>${label}</span><button class="copy-btn" data-copy>copy</button></div>
    <div class="codeblock-body">
      <pre class="codeblock-lines">${numbered}</pre>
      <pre class="codeblock-code"><code>${highlighted}</code></pre>
    </div>
  </div>`;
};

renderer.heading = ({ tokens, depth, text }: Tokens.Heading) => {
  const inner = marked.parser(
    [{ type: "paragraph", raw: text, tokens } as any],
    { renderer },
  ).replace(/^<p>|<\/p>\s*$/g, "");
  const id = slug(text);
  return `<h${depth} id="${id}"><a href="#${id}" class="anchor" aria-label="Link to ${text}">${inner}</a></h${depth}>`;
};

renderer.link = ({ href, title, tokens }: Tokens.Link) => {
  const inner = marked.parser(
    [{ type: "paragraph", raw: "", tokens } as any],
    { renderer },
  ).replace(/^<p>|<\/p>\s*$/g, "");
  const external = /^https?:/.test(href);
  const t = title ? ` title="${title}"` : "";
  const rel = external ? ' target="_blank" rel="noopener noreferrer"' : "";
  return `<a href="${href}"${t}${rel}>${inner}</a>`;
};

marked.use({ renderer, gfm: true, breaks: false });

export function renderMarkdown(src: string): { html: string; toc: { id: string; text: string; depth: number }[] } {
  // Trim leading whitespace
  const trimmed = src.trim();
  // Strip the leading H1 header (e.g. # Title) if present to prevent duplicate H1 tags on the page
  const cleanSrc = trimmed.startsWith("# ")
    ? trimmed.substring(trimmed.indexOf("\n") + 1).trim()
    : trimmed;

  const tokens = marked.lexer(cleanSrc);
  const toc: { id: string; text: string; depth: number }[] = [];
  for (const t of tokens) {
    if (t.type === "heading" && (t.depth === 2 || t.depth === 3)) {
      toc.push({ id: slug(t.text), text: t.text, depth: t.depth });
    }
  }
  const html = marked.parser(tokens);
  return { html, toc };
}
