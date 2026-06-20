import { useEffect, useRef } from "react";
import { Link, useRouterState } from "@tanstack/react-router";
import { renderMarkdown } from "@/lib/md";
import { Sidebar } from "./Sidebar";
import { I } from "@/lib/icons";

export function DocPage({
  source,
  title,
  description,
  crumbs = [],
  edit,
}: {
  source: string;
  title: string;
  description?: string;
  crumbs?: { to?: string; label: string }[];
  edit?: string;
}) {
  const path = useRouterState({ select: (s) => s.location.pathname });
  const { html, toc } = renderMarkdown(source);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // Wire copy buttons inside rendered markdown
    const el = ref.current;
    if (!el) return;
    const handler = (e: Event) => {
      const t = e.target as HTMLElement;
      if (!t.matches("[data-copy]")) return;
      const block = t.closest(".codeblock");
      const code = block?.querySelector(".codeblock-code code")?.textContent ?? "";
      navigator.clipboard.writeText(code).then(() => {
        t.textContent = "copied";
        setTimeout(() => (t.textContent = "copy"), 1400);
      });
    };
    el.addEventListener("click", handler);
    return () => el.removeEventListener("click", handler);
  }, [html]);

  return (
    <div className="mx-auto max-w-[1280px] px-5 sm:px-8 py-10 lg:grid lg:grid-cols-[220px_minmax(0,1fr)_200px] lg:gap-12">
      <aside className="hidden lg:block sticky top-24 self-start max-h-[calc(100vh-7rem)] overflow-y-auto pr-2">
        <Sidebar active={path} />
      </aside>
      <article className="min-w-0">
        <nav className="flex items-center gap-1.5 text-[12px] text-muted mb-5">
          <Link to="/" className="hover:text-ink">Home</Link>
          {crumbs.map((c, i) => (
            <span key={i} className="flex items-center gap-1.5">
              <span className="text-muted-soft">/</span>
              {c.to ? <Link to={c.to} className="hover:text-ink">{c.label}</Link> : <span className="text-ink">{c.label}</span>}
            </span>
          ))}
        </nav>
        <header className="mb-8 rise-in">
          <h1 className="display-lg text-ink">{title}</h1>
          {description && <p className="mt-3 text-[17px] text-body max-w-2xl">{description}</p>}
          <div className="ink-rule mt-6" />
        </header>
        <div
          ref={ref}
          className="prose"
          dangerouslySetInnerHTML={{ __html: html }}
        />
        {edit && (
          <div className="mt-14 pt-6 border-t border-hairline text-sm">
            <a
              href={edit}
              target="_blank" rel="noopener noreferrer"
              className="inline-flex items-center gap-2 text-muted hover:text-ink"
            >
              View source on GitHub <I.ArrowUpRight width={14} height={14} />
            </a>
          </div>
        )}
      </article>
      <aside className="hidden lg:block sticky top-24 self-start max-h-[calc(100vh-7rem)] overflow-y-auto">
        <div className="eyebrow mb-3">On this page</div>
        <ul className="space-y-1.5 text-[13px]">
          {toc.map((h) => (
            <li key={h.id} className={h.depth === 3 ? "pl-3" : ""}>
              <a href={`#${h.id}`} className="text-body hover:text-ink transition-colors block py-0.5">
                {h.text}
              </a>
            </li>
          ))}
        </ul>
      </aside>
    </div>
  );
}
