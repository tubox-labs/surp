import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  Outlet,
  createRootRouteWithContext,
  useRouter,
  HeadContent,
  Scripts,
} from "@tanstack/react-router";
import { useEffect, type ReactNode } from "react";

import appCss from "../styles.css?url";
import { reportError } from "../lib/error-reporting";
import { TopNav, Footer } from "../components/Chrome";
import { CookieBanner } from "../components/CookieBanner";
import { AskAI } from "../components/AskAI";

function NotFoundComponent() {
  return (
    <div className="min-h-[70vh] flex items-center justify-center px-5">
      <div className="text-center max-w-md">
        <div className="eyebrow text-primary">404</div>
        <h1 className="display-lg mt-3 text-ink">This page slipped through</h1>
        <p className="mt-3 text-body">The URL you followed doesn't match a documented page. Try the search (⌘K) or head home.</p>
        <a href="/" className="inline-flex mt-6 items-center gap-2 h-11 px-5 rounded-md bg-ink text-canvas text-[14px] font-medium">Go home</a>
      </div>
    </div>
  );
}

function ErrorComponent({ error, reset }: { error: Error; reset: () => void }) {
  const router = useRouter();
  useEffect(() => { reportError(error, { boundary: "tanstack_root_error_component" }); }, [error]);
  return (
    <div className="min-h-[70vh] flex items-center justify-center px-5">
      <div className="max-w-md text-center">
        <div className="eyebrow text-primary">Something broke</div>
        <h1 className="display-lg mt-3 text-ink">This page didn't render</h1>
        <p className="mt-3 text-body">Try refreshing — if it keeps happening, file an issue on GitHub.</p>
        <div className="mt-6 flex justify-center gap-2">
          <button onClick={() => { router.invalidate(); reset(); }} className="h-11 px-5 rounded-md bg-primary text-on-primary text-[14px] font-medium">Try again</button>
          <a href="/" className="h-11 px-5 rounded-md border border-hairline-strong text-ink text-[14px] inline-flex items-center">Go home</a>
        </div>
      </div>
    </div>
  );
}

export const Route = createRootRouteWithContext<{ queryClient: QueryClient }>()({
  head: () => ({
    meta: [
      { charSet: "utf-8" },
      { name: "viewport", content: "width=device-width, initial-scale=1" },
      { name: "color-scheme", content: "light dark" },
      { name: "theme-color", content: "#f7f7f4" },
      { title: "Surp — A compact, canonical binary serializer" },
      { name: "description", content: "Rust-backed binary serialization with checksums, dedup, optional compression, Python bindings and a CLI. The documentation site for Surp v1 and RFC-001." },
      { name: "author", content: "Tubox Labs" },
      { property: "og:site_name", content: "Surp" },
      { property: "og:title", content: "Surp — A compact, canonical binary serializer" },
      { property: "og:description", content: "Block-framed binary format, XXH64 checksums, optional compression, Rust + Python + CLI." },
      { property: "og:type", content: "website" },
      { name: "twitter:card", content: "summary_large_image" },
      { name: "twitter:title", content: "Surp" },
      { name: "twitter:description", content: "A compact, canonical binary serializer and human-readable alternative to JSON." },
    ],
    links: [
      { rel: "stylesheet", href: appCss },
      { rel: "icon", href: "data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%227%22%20fill%3D%22%23f54e00%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2223%22%20text-anchor%3D%22middle%22%20font-family%3D%22Georgia%22%20font-size%3D%2220%22%20fill%3D%22white%22%3ES%3C%2Ftext%3E%3C%2Fsvg%3E" },
      { rel: "preconnect", href: "https://fonts.googleapis.com" },
      { rel: "preconnect", href: "https://fonts.gstatic.com", crossOrigin: "anonymous" },
      { rel: "stylesheet", href: "https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=Instrument+Serif:ital@0;1&family=JetBrains+Mono:wght@400;500&display=swap" },
    ],
  }),
  shellComponent: RootShell,
  component: RootComponent,
  notFoundComponent: NotFoundComponent,
  errorComponent: ErrorComponent,
});

function RootShell({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <head>
        <HeadContent />
        <script
          dangerouslySetInnerHTML={{
            __html: `try{var t=localStorage.getItem('surp-theme');if(!t){t=matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light';}if(t==='dark')document.documentElement.classList.add('dark');}catch(e){}`,
          }}
        />
      </head>
      <body>
        {children}
        <Scripts />
      </body>
    </html>
  );
}

function RootComponent() {
  const { queryClient } = Route.useRouteContext();
  return (
    <QueryClientProvider client={queryClient}>
      <div className="min-h-screen flex flex-col">
        <TopNav />
        <main className="flex-1">
          <Outlet />
        </main>
        <Footer />
        <CookieBanner />
        <AskAI />
      </div>
    </QueryClientProvider>
  );
}
