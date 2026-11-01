// Where the newest client is: the releases of the repository the service
// names as its source (`GET /health`), which is the one the gateway and the
// client are built from. GitHub lists the newest first, pre-releases
// included (the project is in beta, so `/releases/latest` would miss them).

/** The releases page of a GitHub repository URL, or `null` for anything else. */
export function releasesUrl(source: string): string | null {
  const trimmed = source.trim().replace(/\/+$/, "");
  return /^https:\/\/github\.com\/[\w.-]+\/[\w.-]+$/.test(trimmed) ? `${trimmed}/releases` : null;
}
