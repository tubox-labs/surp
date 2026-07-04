import { Link, useRouterState } from "@tanstack/react-router";
import { useState, type ReactNode } from "react";
import { I } from "@/lib/icons";
import { ThemeToggle } from "./ThemeToggle";
import { SearchPalette } from "./SearchPalette";

export function TopNav() {
  const [open, setOpen] = useState(false);
  const [mobile, setMobile] = useState(false);
  return (
    <header className="sticky top-0 z-30 bg-canvas/85 backdrop-blur-md border-b border-hairline-soft">
      <div className="mx-auto max-w-[1200px] h-16 px-5 sm:px-8 flex items-center gap-6">
        <Link to="/" className="flex items-baseline gap-2 group">
          <span className="text-[22px] font-display tracking-tight text-ink">Surp</span>
          <span className="text-[10px] eyebrow text-primary group-hover:opacity-80">v1.2.0</span>
        </Link>
        <nav className="hidden md:flex items-center gap-7 text-[14px] font-medium text-body ml-3">
          <NavLink to="/docs">Docs</NavLink>
          <NavLink to="/architecture">Architecture</NavLink>
          <NavLink to="/docs/rust-api">API</NavLink>
          <NavLink to="/benchmarks">Benchmarks</NavLink>
          <NavLink to="/examples">Examples</NavLink>
          <NavLink to="/community">Community</NavLink>
          <NavLink to="/changelog">Changelog</NavLink>
        </nav>
        <div className="ml-auto flex items-center gap-2">
          <button
            onClick={() => setOpen(true)}
            className="h-9 hidden sm:flex items-center gap-2 px-3 rounded-md border border-hairline text-muted hover:text-ink hover:bg-surface-strong text-[13px] transition-colors"
          >
            <I.Search width={14} height={14} /> Search
            <kbd className="ml-2 text-[10px] px-1 border border-hairline rounded">⌘K</kbd>
          </button>
          <button onClick={() => setOpen(true)} className="sm:hidden h-9 w-9 inline-flex items-center justify-center rounded-md border border-hairline text-ink" aria-label="Search">
            <I.Search />
          </button>
          <ThemeToggle />
          <a
            href="https://github.com/tubox-labs/surp"
            target="_blank" rel="noopener noreferrer"
            className="h-9 w-9 hidden sm:inline-flex items-center justify-center rounded-md border border-hairline text-ink hover:bg-surface-strong"
            aria-label="GitHub"
          >
            <I.GitHub />
          </a>
          <button
            onClick={() => setMobile((v) => !v)}
            className="md:hidden h-9 w-9 inline-flex items-center justify-center rounded-md border border-hairline text-ink"
            aria-label="Menu"
          >
            {mobile ? <I.Close /> : <I.Menu />}
          </button>
        </div>
      </div>
      {mobile && (
        <div className="md:hidden border-t border-hairline-soft bg-canvas">
          <div className="px-5 py-4 grid gap-2 text-[14px]">
            {[
              ["/docs", "Docs"],
              ["/architecture", "Architecture"],
              ["/docs/rust-api", "API"],
              ["/benchmarks", "Benchmarks"],
              ["/examples", "Examples"],
              ["/community", "Community"],
              ["/changelog", "Changelog"],
            ].map(([to, label]) => (
              <Link key={to} to={to} onClick={() => setMobile(false)} className="py-2 text-ink">
                {label}
              </Link>
            ))}
          </div>
        </div>
      )}
      <SearchPalette open={open} onOpenChange={setOpen} />
    </header>
  );
}

function NavLink({ to, children }: { to: string; children: ReactNode }) {
  const path = useRouterState({ select: (s) => s.location.pathname });
  const active = path === to || (to !== "/" && path.startsWith(to));
  return (
    <Link
      to={to}
      className={`relative transition-colors ${active ? "text-ink" : "text-body hover:text-ink"}`}
    >
      {children}
      {active && <span className="absolute -bottom-[22px] left-0 right-0 h-px bg-primary" />}
    </Link>
  );
}

export function Footer() {
  return (
    <footer className="mt-32 border-t border-hairline">
      <div className="mx-auto max-w-[1200px] px-5 sm:px-8 py-14 grid grid-cols-2 sm:grid-cols-5 gap-8 text-[14px]">
        <div className="col-span-2">
          <div className="text-[22px] font-display text-ink">Surp</div>
          <p className="mt-2 text-body max-w-xs">
            A compact, canonical binary serializer and human-readable alternative to JSON.
            Rust-backed, Python-native, MIT/Apache-2.0.
          </p>
        </div>
        <FooterCol title="Documentation" links={[
          ["/getting-started", "Getting Started"],
          ["/docs", "Docs Home"],
          ["/docs/spec", "v1 Spec"],
          ["/docs/rfc001", "RFC-001"],
        ]} />
        <FooterCol title="Reference" links={[
          ["/docs/rust-api", "Rust API"],
          ["/docs/python-api", "Python API"],
          ["/docs/cli", "CLI"],
          ["/docs/mcp", "MCP"],
        ]} />
        <FooterCol title="Project" links={[
          ["/community", "Community"],
          ["/help", "Help"],
          ["/changelog", "Changelog"],
          ["/security", "Security"],
          ["/privacy", "Privacy"],
          ["/terms", "Terms"],
          ["/cookies", "Cookies"],
        ]} />
      </div>
      <div className="border-t border-hairline-soft">
        <div className="mx-auto max-w-[1200px] px-5 sm:px-8 py-6 flex flex-wrap items-center gap-3 text-[12px] text-muted">
          <span>© {new Date().getFullYear()} Tubox Labs.</span>
          <span>Surp is released under MIT or Apache-2.0.</span>
          <span className="ml-auto">Built with care. No analytics, no trackers.</span>
        </div>
      </div>
    </footer>
  );
}

function FooterCol({ title, links }: { title: string; links: [string, string][] }) {
  return (
    <div>
      <div className="eyebrow mb-3">{title}</div>
      <ul className="space-y-2">
        {links.map(([to, label]) => (
          <li key={to}>
            <Link to={to} className="text-body hover:text-ink transition-colors">{label}</Link>
          </li>
        ))}
      </ul>
    </div>
  );
}
