export function reportError(error: unknown, context: Record<string, unknown> = {}) {
  if (typeof window === "undefined") {
    console.error("[Server Error Handled]", error, context);
    return;
  }
  console.error("[Client Error Handled]", error, {
    route: window.location.pathname,
    ...context,
  });
}
