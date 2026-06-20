import { useEffect, useState } from "react";
import { I } from "@/lib/icons";

const KEY = "surp-theme";

function apply(theme: "light" | "dark") {
  const root = document.documentElement;
  root.classList.toggle("dark", theme === "dark");
}

export function ThemeToggle() {
  const [theme, setTheme] = useState<"light" | "dark">("light");
  useEffect(() => {
    const saved = (localStorage.getItem(KEY) as "light" | "dark" | null) ??
      (window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light");
    setTheme(saved);
    apply(saved);
  }, []);
  const toggle = () => {
    const next = theme === "light" ? "dark" : "light";
    setTheme(next);
    apply(next);
    localStorage.setItem(KEY, next);
  };
  return (
    <button
      onClick={toggle}
      aria-label="Toggle color theme"
      className="inline-flex h-9 w-9 items-center justify-center rounded-md border border-hairline text-ink hover:bg-surface-strong transition-colors"
    >
      {theme === "light" ? <I.Moon /> : <I.Sun />}
    </button>
  );
}
