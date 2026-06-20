import { createFileRoute, Link } from "@tanstack/react-router";
import { I } from "@/lib/icons";

export const Route = createFileRoute("/community")({
  head: () => ({
    meta: [
      { title: "Community — Surp" },
      { name: "description", content: "Where Surp conversations happen: GitHub discussions, issues, pull requests, and the contribution loop." },
      { property: "og:title", content: "Community — Surp" },
      { property: "og:url", content: "/community" },
    ],
    links: [{ rel: "canonical", href: "/community" }],
  }),
  component: Community,
});

function Community() {
  return (
    <div className="mx-auto max-w-[1000px] px-5 sm:px-8 py-16">
      <div className="eyebrow mb-3">Community</div>
      <h1 className="display-lg text-ink max-w-2xl">Small, considered, and welcoming.</h1>
      <p className="mt-4 text-body text-[17px] max-w-2xl">
        Surp is developed in the open at{" "}
        <a className="text-ink underline underline-offset-2 decoration-hairline-strong hover:decoration-primary" href="https://github.com/tubox-labs/surp" target="_blank" rel="noopener noreferrer">tubox-labs/surp</a>.
        The repository is the authoritative source of truth for issues, discussions, and contributions.
      </p>
      <div className="ink-rule my-10" />
      <div className="grid sm:grid-cols-2 gap-px bg-hairline border border-hairline rounded-xl overflow-hidden">
        {[
          { t: "File an issue", d: "Bug reports, feature requests, format clarifications — open an issue with a minimal reproduction.", to: "https://github.com/tubox-labs/surp/issues" },
          { t: "Open a discussion", d: "Design questions and RFC-001 feedback are easier to thread through GitHub Discussions.", to: "https://github.com/tubox-labs/surp/discussions" },
          { t: "Send a pull request", d: "Read the local development loop in the README. Run cargo fmt, cargo test --workspace --all-features, and cargo clippy -D warnings before pushing.", to: "https://github.com/tubox-labs/surp/blob/main/README.md#local-development" },
          { t: "Report a vulnerability", d: "Follow the policy in SECURITY.md — please do not file public issues for security reports.", to: "https://github.com/tubox-labs/surp/blob/main/SECURITY.md" },
        ].map((c) => (
          <a key={c.t} href={c.to} target="_blank" rel="noopener noreferrer"
            className="group bg-surface p-7 hover:bg-canvas-soft transition-colors flex items-start justify-between gap-4">
            <div>
              <h3 className="display-md text-ink">{c.t}</h3>
              <p className="mt-2 text-body text-[14.5px]">{c.d}</p>
            </div>
            <I.ArrowUpRight className="text-muted-soft group-hover:text-primary shrink-0 mt-2" />
          </a>
        ))}
      </div>

      <section className="mt-16">
        <div className="eyebrow text-primary mb-3">Contribution loop</div>
        <h2 className="display-md text-ink">The shape of a healthy patch</h2>
        <ol className="mt-6 grid gap-4 list-none">
          {[
            "Open an issue first if the change touches the v1 wire format, public Rust API, or Python public surface.",
            "Match the existing style — the codebase uses cargo fmt for Rust and ruff/pyright conventions for Python.",
            "Add or update tests in the affected crate. Workspace tests must pass with --all-features.",
            "Update CHANGELOG.md under the [Unreleased] section using Keep a Changelog conventions.",
            "Reference the issue in your pull request and describe the user-visible impact.",
          ].map((step, i) => (
            <li key={i} className="grid grid-cols-[28px_1fr] gap-4 py-3 border-t border-hairline">
              <span className="text-primary font-display text-[22px] leading-none">{(i+1).toString().padStart(2, "0")}</span>
              <span className="text-body text-[15px] leading-relaxed">{step}</span>
            </li>
          ))}
        </ol>
        <div className="mt-8">
          <Link to="/help" className="inline-flex items-center gap-2 text-ink hover:text-primary text-[14px]">Need help first? Read the help page <I.Arrow width={14} height={14} /></Link>
        </div>
      </section>
    </div>
  );
}
