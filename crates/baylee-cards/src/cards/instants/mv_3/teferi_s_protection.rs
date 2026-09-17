//! Teferi's Protection — {2}{W} — Instant
//! Oracle: Until your next turn, your life total can't change and you gain protection from everything. All permanents you control phase out. (While they're phased out, they're treated as though they don't exist. They phase in before you untap during your untap step.)
//! Oracle: Exile Teferi's Protection.
//! Set: 2X2 #32 — Double Masters 2022 | Scryfall ID: 483fa1cb-1e35-44f2-a143-98c0f107f5ca | Oracle ID: 0d4ecdb1-ec90-497f-a7a4-1c68092b8757
// PARTIAL — only the printed self-exile is expressible; the life lock, the
// protection and the mass phase-out are listed below as NOT SUPPORTED.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TEFERI_S_PROTECTION,
    oracle_id = "0d4ecdb1-ec90-497f-a7a4-1c68092b8757",
    scryfall_id = "483fa1cb-1e35-44f2-a143-98c0f107f5ca",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Teferi's Protection",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Partial(
        "\"your life total can't change\", \"you gain protection from everything\" and \"all permanents you control phase out\" have no variant; only `Exile Teferi's Protection` is built"
    ),
    abilities = &[spell!(&[Effect::ExileSource])],
);

// NOT SUPPORTED: "Until your next turn, your life total can't change" — the
// nearest variant, `Modifier::CantLoseLife` (Everybody Lives!), stops life
// *loss* and nothing else, and `Duration::UntilYourNextTurn` cannot make it
// the printed "can't change" either: nothing in the vocabulary states that a
// life total may not be raised.
// NOT SUPPORTED: "you gain protection from everything" — every protection in
// the vocabulary is scoped to an *object* (`Modifier::ProtectionFrom` on a
// filter, `Modifier::PlayerHexproof` as a player-only shadow of it), and
// neither is "protection from everything" for a player: no modifier reaches
// a player's damage, targeting, attachment and blocking at once.
// NOT SUPPORTED: "All permanents you control phase out" — `Effect::PhaseOut`
// takes one `TargetSpec`, or the source when it names none, so it can phase
// out this spell and no more; there is no filter-wide phase-out, and no
// `TargetReq` makes "all permanents you control" into a choice.
