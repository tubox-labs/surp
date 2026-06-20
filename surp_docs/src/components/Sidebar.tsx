import { Link } from "@tanstack/react-router";

export const docsNav = [
  {
    label: "Start here",
    items: [
      { to: "/getting-started", label: "Getting Started" },
      { to: "/docs", label: "Documentation home" },
      { to: "/examples", label: "Examples" },
    ],
  },
  {
    label: "Architecture",
    items: [
      { to: "/architecture", label: "Architecture overview" },
      { to: "/docs/spec", label: "v1 binary spec" },
      { to: "/docs/rfc001", label: "RFC-001 (CTN / CBF / CQL)" },
    ],
  },
  {
    label: "Reference",
    items: [
      { to: "/docs/rust-api", label: "Rust API" },
      { to: "/docs/python-api", label: "Python API" },
      { to: "/docs/cli", label: "CLI reference" },
      { to: "/docs/mcp", label: "MCP server" },
    ],
  },
  {
    label: "Project",
    items: [
      { to: "/changelog", label: "Changelog" },
      { to: "/community", label: "Community" },
      { to: "/help", label: "Help" },
      { to: "/security", label: "Security & risks" },
    ],
  },
] as const;

export function Sidebar({ active }: { active?: string }) {
  return (
    <nav className="text-[14px]">
      {docsNav.map((g) => (
        <div key={g.label} className="mb-7">
          <div className="eyebrow mb-3">{g.label}</div>
          <ul className="space-y-[6px]">
            {g.items.map((it) => {
              const isActive = active === it.to;
              return (
                <li key={it.to}>
                  <Link
                    to={it.to}
                    className={`block py-[3px] pl-3 -ml-3 border-l transition-colors ${
                      isActive
                        ? "text-ink border-primary font-medium"
                        : "text-body border-transparent hover:text-ink hover:border-hairline-strong"
                    }`}
                  >
                    {it.label}
                  </Link>
                </li>
              );
            })}
          </ul>
        </div>
      ))}
    </nav>
  );
}
