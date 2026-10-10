// Where a card's picture is, on Scryfall's own image host.
//
// The admin's browser loads it straight from there (`docs/legal.md` §3,
// #270): this service never fetches, stores or passes on a card image, so
// it is neither a proxy nor a mirror of Scryfall's data. The page's CSP
// names `cards.scryfall.io` as the one image host besides itself, and its
// referrer policy is `no-referrer`, so Scryfall sees the card id and
// nothing of the report. The URL is built from the printing's id alone;
// nothing here calls Scryfall's API.

/** The one host the page may load a picture from besides itself. */
export const IMAGE_HOST = "https://cards.scryfall.io";

/** Where a printing's page is. */
export const CARD_HOST = "https://scryfall.com";

const SCRYFALL_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

export type ArtSize = "small" | "normal" | "large";

/** Whether `id` is shaped like a Scryfall id, which is all that may go into a URL. */
export function isScryfallId(id: string): boolean {
  return SCRYFALL_ID.test(id);
}

/**
 * The picture of printing `id`, as the gateway's `/art` names it too
 * (`crates/baylee-gateway/src/art.rs`): `/{size}/{face}/{a}/{b}/{id}.jpg`,
 * `a` and `b` the id's first two characters. `null` for an id that is not
 * one.
 */
export function imageUrl(id: string, face: "front" | "back" = "front", size: ArtSize = "normal"): string | null {
  if (!isScryfallId(id)) return null;
  return `${IMAGE_HOST}/${size}/${face}/${id[0] ?? ""}/${id[1] ?? ""}/${id}.jpg`;
}

/** The printing's page on Scryfall; `null` for an id that is not one. */
export function cardUrl(id: string): string | null {
  return isScryfallId(id) ? `${CARD_HOST}/card/${id}` : null;
}

/** A set's page on Scryfall; `null` for a code that is not one. */
export function setUrl(code: string): string | null {
  return /^[a-z0-9]{2,6}$/i.test(code) ? `${CARD_HOST}/sets/${code.toLowerCase()}` : null;
}
