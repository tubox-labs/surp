import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import Fuse from "fuse.js";
import { I } from "@/lib/icons";
import { searchIndex } from "@/lib/search-index";

export function SearchPalette({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
}) {
  const navigate = useNavigate();
  const [q, setQ] = useState("");
  const fuse = useMemo(
    () =>
      new Fuse(searchIndex, {
        keys: [
          { name: "title", weight: 0.6 },
          { name: "section", weight: 0.25 },
          { name: "body", weight: 0.15 },
        ],
        threshold: 0.36,
        ignoreLocation: true,
        minMatchCharLength: 2,
      }),
    [],
  );
  const results = useMemo(() => {
    if (!q.trim()) return searchIndex.slice(0, 8);
    return fuse.search(q).slice(0, 12).map((r) => r.item);
  }, [q, fuse]);
  const [active, setActive] = useState(0);
  useEffect(() => setActive(0), [q]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        onOpenChange(!open);
      } else if (e.key === "/" && !open && !(e.target instanceof HTMLInputElement) && !(e.target instanceof HTMLTextAreaElement)) {
        e.preventDefault();
        onOpenChange(true);
      } else if (e.key === "Escape" && open) {
        onOpenChange(false);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onOpenChange]);

  if (!open) return null;
  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center p-4 sm:p-10 bg-ink/40 backdrop-blur-sm"
      onClick={() => onOpenChange(false)}
    >
      <div
        className="w-full max-w-xl rounded-xl border border-hairline bg-surface overflow-hidden rise-in"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-3 px-4 py-3 border-b border-hairline">
          <I.Search className="text-muted" />
          <input
            autoFocus
            value={q}
            onChange={(e) => setQ(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown") {
                e.preventDefault();
                setActive((a) => Math.min(a + 1, results.length - 1));
              } else if (e.key === "ArrowUp") {
                e.preventDefault();
                setActive((a) => Math.max(0, a - 1));
              } else if (e.key === "Enter" && results[active]) {
                e.preventDefault();
                navigate({ to: results[active].path, hash: results[active].hash });
                onOpenChange(false);
              }
            }}
            placeholder="Search the docs…"
            className="w-full bg-transparent outline-none text-ink placeholder:text-muted-soft text-[15px]"
          />
          <kbd className="text-[11px] text-muted px-1.5 py-0.5 border border-hairline rounded">esc</kbd>
        </div>
        <ul className="max-h-[60vh] overflow-y-auto py-2">
          {results.length === 0 && (
            <li className="px-4 py-8 text-center text-muted text-sm">No matches for "{q}"</li>
          )}
          {results.map((r, i) => (
            <li key={r.path + (r.hash || "")}>
              <button
                onMouseEnter={() => setActive(i)}
                onClick={() => {
                  navigate({ to: r.path, hash: r.hash });
                  onOpenChange(false);
                }}
                className={`w-full text-left px-4 py-2.5 flex items-baseline gap-3 ${
                  active === i ? "bg-canvas-soft" : ""
                }`}
              >
                <span className="text-ink text-[14px] font-medium">{r.title}</span>
                <span className="text-muted text-[12px]">{r.section}</span>
                <I.Arrow className="ml-auto text-muted-soft" />
              </button>
            </li>
          ))}
        </ul>
        <div className="border-t border-hairline px-4 py-2 flex items-center gap-3 text-[11px] text-muted">
          <span><kbd className="px-1 border border-hairline rounded">↑↓</kbd> navigate</span>
          <span><kbd className="px-1 border border-hairline rounded">↵</kbd> open</span>
          <span className="ml-auto">Fuzzy search across {searchIndex.length} pages</span>
        </div>
      </div>
    </div>
  );
}
