import type { ReaderView } from "@quran-reader/types";

// Links out of the app. The Tauri side opens only these (`LINKS` in src-tauri/src/lib.rs).

export const REPO_URL = "https://github.com/EnsoBIH/quran-reader";
export const RELEASES_URL = `${REPO_URL}/releases/latest`;

export const SOURCE_URLS = {
  tanzil: "https://tanzil.net/",
  amiri: "https://github.com/aliftype/amiri/",
  gutenberg: "https://www.gutenberg.org/ebooks/16955/",
} as const;

export interface ProblemContext {
  /** App version, e.g. `1.0.0`; null in the browser preview. */
  version: string | null;
  /** The verse being read or the selected one, e.g. `2:255`. */
  verse: string | null;
  /** Shown translation, if any. */
  translation: string | null;
  view: ReaderView;
  theme: string;
  /** `navigator.userAgent`. */
  userAgent: string;
}

/** The OS as a reporter would name it, from the webview's user agent. */
export function osName(userAgent: string): string {
  if (/Windows/.test(userAgent)) return "Windows";
  if (/Macintosh|Mac OS X/.test(userAgent)) return "macOS";
  if (/Linux/.test(userAgent)) return "Linux";
  return "";
}

/**
 * A new GitHub issue with the "Report a problem" form (.github/ISSUE_TEMPLATE/problem.yml),
 * its fields filled in from the app. Nothing is sent: the browser opens the form and the
 * user decides what to submit.
 */
export function problemUrl(ctx: ProblemContext): string {
  const params = new URLSearchParams({ template: "problem.yml" });
  if (ctx.verse) params.set("verse", ctx.verse);
  if (ctx.version) params.set("version", ctx.version);
  const os = osName(ctx.userAgent);
  if (os) params.set("os", os);
  const setup = [
    `view: ${ctx.view}`,
    ctx.view !== "arabic" && ctx.translation ? `translation: ${ctx.translation}` : null,
    `theme: ${ctx.theme}`,
  ];
  params.set("setup", setup.filter(Boolean).join(", "));
  return `${REPO_URL}/issues/new?${params}`;
}
