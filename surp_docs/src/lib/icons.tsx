// Custom inline icon set — hand-drawn ink line feel.
// No icon library. Strokes are intentionally a touch off-axis.
import { type SVGProps } from "react";

const S = (p: SVGProps<SVGSVGElement>) => ({
  width: 18,
  height: 18,
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.6,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
  ...p,
});

export const I = {
  Search: (p: SVGProps<SVGSVGElement>) => (
    <svg {...S(p)}><circle cx="10.5" cy="10.5" r="6.2"/><path d="m20 20-4.6-4.6"/></svg>
  ),
  Menu: (p: SVGProps<SVGSVGElement>) => (
    <svg {...S(p)}><path d="M4 7h16M4 12h16M4 17h11"/></svg>
  ),
  Close: (p: SVGProps<SVGSVGElement>) => (
    <svg {...S(p)}><path d="m6 6 12 12M18 6 6 18"/></svg>
  ),
  Arrow: (p: SVGProps<SVGSVGElement>) => (
    <svg {...S(p)}><path d="M5 12h14M13 6l6 6-6 6"/></svg>
  ),
  ArrowUpRight: (p: SVGProps<SVGSVGElement>) => (
    <svg {...S(p)}><path d="M7 17 17 7M9 7h8v8"/></svg>
  ),
  Sun: (p: SVGProps<SVGSVGElement>) => (
    <svg {...S(p)}><circle cx="12" cy="12" r="3.6"/><path d="M12 3v2.4M12 18.6V21M3 12h2.4M18.6 12H21M5.5 5.5l1.7 1.7M16.8 16.8l1.7 1.7M5.5 18.5l1.7-1.7M16.8 7.2l1.7-1.7"/></svg>
  ),
  Moon: (p: SVGProps<SVGSVGElement>) => (
    <svg {...S(p)}><path d="M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z"/></svg>
  ),
  GitHub: (p: SVGProps<SVGSVGElement>) => (
    <svg {...S(p)}><path d="M9 19c-4 1-4-2-6-2.5M15 22v-3.6a3 3 0 0 0-.9-2.3c3-.3 6.1-1.4 6.1-6.4a4.8 4.8 0 0 0-1.3-3.4 4.4 4.4 0 0 0-.1-3.4s-1.1-.3-3.6 1.4a12 12 0 0 0-6.4 0C6.3 2.6 5.2 2.9 5.2 2.9a4.4 4.4 0 0 0-.1 3.4 4.8 4.8 0 0 0-1.3 3.4c0 4.9 3 6 6 6.4a3 3 0 0 0-.8 2.3V22"/></svg>
  ),
  Copy: (p: SVGProps<SVGSVGElement>) => (
    <svg {...S(p)}><rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h10"/></svg>
  ),
  Rust: (p: SVGProps<SVGSVGElement>) => (
    <svg {...S(p)}><path d="M12 3 7 6v12l5 3 5-3V6z"/><path d="M12 8v8M8 10l8 4M16 10l-8 4"/></svg>
  ),
  Python: (p: SVGProps<SVGSVGElement>) => (
    <svg {...S(p)}><path d="M9 5h6a3 3 0 0 1 3 3v3H9a3 3 0 0 0-3 3v3a3 3 0 0 0 3 3"/><path d="M15 19H9a3 3 0 0 1-3-3v-3h9a3 3 0 0 0 3-3V7a3 3 0 0 0-3-3"/><circle cx="10" cy="8" r=".7" fill="currentColor"/><circle cx="14" cy="16" r=".7" fill="currentColor"/></svg>
  ),
  Terminal: (p: SVGProps<SVGSVGElement>) => (
    <svg {...S(p)}><rect x="3" y="4.5" width="18" height="15" rx="2"/><path d="m7 10 3 2-3 2M13 14h4"/></svg>
  ),
  Book: (p: SVGProps<SVGSVGElement>) => (
    <svg {...S(p)}><path d="M4 4.5A1.5 1.5 0 0 1 5.5 3H19v15.5H6a2 2 0 0 0-2 2zM6 18.5h13"/></svg>
  ),
  Spark: (p: SVGProps<SVGSVGElement>) => (
    <svg {...S(p)}><path d="M12 3v6M12 15v6M3 12h6M15 12h6M5.5 5.5 9 9M15 15l3.5 3.5M5.5 18.5 9 15M15 9l3.5-3.5"/></svg>
  ),
};
