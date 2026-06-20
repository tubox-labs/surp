import { useEffect, useState } from "react";
import { I } from "@/lib/icons";

const KEY = "surp-cookie-consent";

export function CookieBanner() {
  const [open, setOpen] = useState(false);
  useEffect(() => {
    if (typeof window === "undefined") return;
    if (!localStorage.getItem(KEY)) setOpen(true);
  }, []);
  if (!open) return null;
  const set = (v: "accepted" | "declined") => {
    localStorage.setItem(KEY, v);
    setOpen(false);
  };
  return (
    <div className="fixed bottom-4 left-4 right-4 sm:left-6 sm:right-auto sm:max-w-md z-40 rise-in">
      <div className="rounded-lg border border-hairline bg-surface p-4 shadow-[0_1px_0_var(--hairline)]">
        <div className="flex items-start gap-3">
          <div className="text-[13.5px] text-body leading-relaxed">
            We use a single browser storage key to remember your theme and this preference.
            No analytics, no tracking, no third parties. See the{" "}
            <a href="/cookies" className="text-ink underline underline-offset-2 decoration-hairline-strong">Cookie Policy</a>.
          </div>
          <button onClick={() => set("declined")} aria-label="Dismiss" className="text-muted hover:text-ink">
            <I.Close />
          </button>
        </div>
        <div className="mt-3 flex gap-2">
          <button
            onClick={() => set("accepted")}
            className="h-9 px-4 rounded-md bg-primary text-on-primary text-[13px] font-medium hover:bg-primary-active transition-colors"
          >
            Got it
          </button>
          <button
            onClick={() => set("declined")}
            className="h-9 px-4 rounded-md border border-hairline-strong text-ink text-[13px] hover:bg-surface-strong transition-colors"
          >
            Only essentials
          </button>
        </div>
      </div>
    </div>
  );
}
