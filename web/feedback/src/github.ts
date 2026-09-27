// Linking a report to a GitHub issue. The server holds no GitHub token: the
// UI opens GitHub's own "new issue" page with the fields filled in, the
// admin reads it over and submits it there, and then links the number.

import type { Summary } from "./api";

export const REPOSITORY = "https://github.com/AceVik/baylee";

/** The longest report text copied into an issue; GitHub's URL has a limit. */
export const TEXT_CHARS = 4000;

const KIND_TITLES: Record<Summary["kind"], string> = {
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

/**
 * GitHub's "new issue" page for `report`, prefilled. The body quotes the
 * player's text and names the build and the report; it leaves out the
 * reporter's pseudonym and everything the client sent, which stay here.
 */
export function newIssueUrl(report: Summary, origin: string): string {
  const title = `${KIND_TITLES[report.kind]}: ${firstLine(report.text, 80) || "(no text)"}`;
  const text =
    report.text.length > TEXT_CHARS ? `${report.text.slice(0, TEXT_CHARS)}\n[…]` : report.text;
  const quoted = text
    .split(/\r?\n/)
    .map((line) => `> ${line}`)
    .join("\n");
  const body = [
    quoted,
    "",
    `- Report: ${origin}/r/${report.id}`,
    `- Kind: ${report.kind}`,
    `- Build: ${report.gateway_version}`,
    `- Gateway: ${report.gateway}`,
    `- Received: ${report.created_at}`,
    `- Game record: ${report.has_record ? (report.record_complete ? "complete" : "partial") : "none"}`,
  ].join("\n");
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
