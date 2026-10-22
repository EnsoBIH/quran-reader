// Links within the site, below the configured base (`/quran-reader/` on GitHub Pages).

const base = import.meta.env.BASE_URL.replace(/\/$/, "");

/** `path` below the site's base, e.g. `href("download")` → `/quran-reader/download`. */
export function href(path = ""): string {
  return `${base}/${path.replace(/^\//, "")}`;
}

export const REPO_URL = "https://github.com/EnsoBIH/quran-reader";
export const RELEASES_URL = `${REPO_URL}/releases/latest`;
export const ISSUE_URL = `${REPO_URL}/issues/new?template=problem.yml`;
