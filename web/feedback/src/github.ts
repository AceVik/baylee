// Linking a report to a GitHub issue. The server holds no GitHub token: the
// UI opens GitHub's own "new issue" page with the fields filled in, the
// admin reads it over and submits it there, and then links the number.
//
// What goes into the public issue (owner, 08.10.2026): only a neutral
// technical summary the admin writes themselves, a neutral category and the
// build. Never the player's text, their pseudonym, anything the client
// sent, nor a link back to the report: the service's address and a
// report's id (a UUIDv7, which carries when it arrived) say nothing a
// stranger needs. The way back runs inside the service, where the report
// keeps the issue's number.

import type { Kind } from "./api";

export const REPOSITORY = "https://github.com/AceVik/baylee";

/** The longest summary copied into an issue; GitHub's URL has a limit. */
export const SUMMARY_CHARS = 4000;

/** The neutral categories an issue is filed under, one per report kind. */
export const CATEGORY_TITLES: Record<Kind, string> = {
  bug: "Bug",
  improvement: "Improvement",
  feedback: "Feedback",
  crash: "Crash",
  other: "Report",
};

/** The first line of `text`, cut to `chars` with an ellipsis. */
export function firstLine(text: string, chars: number): string {
  const line = (text.split(/\r?\n/).find((l) => l.trim() !== "") ?? "").trim();
  return line.length > chars ? `${line.slice(0, chars - 1).trimEnd()}…` : line;
}

/** What an admin files: their own words, a category, the build. */
export interface IssueDraft {
  /** The admin's neutral technical summary. */
  summary: string;
  category: Kind;
  /** The build the report came from, as the gateway named it. */
  build: string;
}

/**
 * GitHub's "new issue" page for `draft`, or `null` while the summary is
 * blank: there is nothing to file before the admin has written it. It takes
 * a draft, not a report, so nothing of the report can reach it but the
 * category and the build the caller hands in.
 */
export function newIssueUrl(draft: IssueDraft): string | null {
  const summary = draft.summary.trim();
  if (summary === "") return null;
  const title = `${CATEGORY_TITLES[draft.category]}: ${firstLine(summary, 80)}`;
  const text = summary.length > SUMMARY_CHARS ? `${summary.slice(0, SUMMARY_CHARS)}\n[…]` : summary;
  const body = [text, "", `- Category: ${draft.category}`, `- Build: ${draft.build}`].join("\n");
  const params = new URLSearchParams({ title, body });
  return `${REPOSITORY}/issues/new?${params.toString()}`;
}

/**
 * An issue as an admin types it: `311`, `#311`, or the issue's URL in this
 * repository. `null` for anything else.
 */
export function parseIssue(input: string): number | null {
  const text = input.trim();
  const match =
    /^#?(\d{1,9})$/.exec(text) ??
    /^https:\/\/github\.com\/AceVik\/baylee\/(?:issues|pull)\/(\d{1,9})\/?(?:[?#].*)?$/i.exec(text);
  const digits = match?.[1];
  if (digits === undefined) return null;
  const n = Number(digits);
  return n > 0 && n <= 2_147_483_647 ? n : null;
}
