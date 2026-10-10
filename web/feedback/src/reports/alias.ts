// A reporter's pseudonym as something an admin can tell apart at a glance.
//
// The pseudonym is an HMAC of the account id under a key only the gateway
// holds (`docs/feedback.md` §"The gateway"): the service cannot read the
// account from it, and that is the point. This UI does not ask the gateway
// to turn it back into a name either, because the service's promise is
// that it never knows who wrote a report (`docs/privacy.md`). What it does
// instead is derive a stable alias and a hue from the hash itself, so the
// same reporter reads the same way in every row and a second one reads
// differently, and nothing more about them can be read from it than from
// the hex.

const ADJECTIVES = [
  "Amber", "Bold", "Brisk", "Calm", "Clever", "Cool", "Crisp", "Dapper",
  "Deep", "Eager", "Fair", "Fleet", "Fond", "Frank", "Gentle", "Glad",
  "Grand", "Hardy", "Hollow", "Humble", "Jolly", "Keen", "Kind", "Late",
  "Light", "Lively", "Loyal", "Lucky", "Merry", "Mild", "Misty", "Neat",
  "Noble", "Odd", "Pale", "Plain", "Proud", "Quick", "Quiet", "Rapid",
  "Rare", "Ready", "Rosy", "Royal", "Rusty", "Sharp", "Shy", "Silent",
  "Sly", "Soft", "Solid", "Spry", "Steady", "Still", "Stout", "Swift",
  "Tidy", "True", "Vivid", "Warm", "Wild", "Wise", "Witty", "Young",
] as const;

const NOUNS = [
  "Badger", "Bear", "Beetle", "Bison", "Crane", "Crow", "Deer", "Dove",
  "Eagle", "Eel", "Elk", "Falcon", "Ferret", "Finch", "Fox", "Frog",
  "Gull", "Hare", "Hawk", "Heron", "Hornet", "Ibis", "Jay", "Koi",
  "Lark", "Lemur", "Lion", "Llama", "Lynx", "Mole", "Moose", "Moth",
  "Newt", "Orca", "Otter", "Owl", "Panda", "Pike", "Puma", "Quail",
  "Raven", "Robin", "Seal", "Shrew", "Snail", "Sparrow", "Stag", "Stork",
  "Swan", "Swift", "Tapir", "Tern", "Toad", "Trout", "Viper", "Vole",
  "Wasp", "Whale", "Wolf", "Wren", "Yak", "Zebra", "Carp", "Crab",
] as const;

/** FNV-1a over the UTF-16 code units: for a pseudonym that is not hex. */
function fnv(text: string): number {
  let hash = 0x811c9dc5;
  for (let i = 0; i < text.length; i += 1) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash >>> 0;
}

/** 32 bits of `pseudonym`: its first eight hex digits, else a hash of it. */
function bits(pseudonym: string): number {
  const hex = /^[0-9a-f]{8}/i.exec(pseudonym)?.[0];
  return hex === undefined ? fnv(pseudonym) : Number.parseInt(hex, 16) >>> 0;
}

export interface Alias {
  /** Two words, such as `Quiet Heron`. */
  name: string;
  /** The first four characters of the pseudonym, which tell two reporters of one name apart. */
  short: string;
  /** A hue in degrees, for a dot beside the name. */
  hue: number;
}

/** The alias of `pseudonym`, the same every time. */
export function alias(pseudonym: string): Alias {
  const n = bits(pseudonym);
  const adjective = ADJECTIVES[n % ADJECTIVES.length] ?? "Plain";
  const noun = NOUNS[Math.floor(n / 64) % NOUNS.length] ?? "Wren";
  return {
    name: `${adjective} ${noun}`,
    short: pseudonym.slice(0, 4),
    hue: Math.floor(n / 4096) % 360,
  };
}
