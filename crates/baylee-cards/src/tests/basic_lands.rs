use super::every_card_file;

/// "Basic land card" is the Basic *supertype* (CR 205.4a), and spelling it
/// as the five basic land subtypes is a rules error, not a style choice:
/// Breeding Pool has the Forest subtype and is not a basic land, so a
/// Rampant Growth written that way fetches shocklands.
///
/// Naming a few of the subtypes is normal and correct — a fetchland asks
/// for "a Plains or Island card", a checkland looks for one you control.
/// Two generated cards were written the wrong way, past `clippy`, the card
/// tests and a review pass, and nothing here noticed until they were read
/// by hand.
///
/// This said "naming all five at once never means anything but basic" and
/// forbade the shape outright, which held until the pool reached the one
/// card it is not true of. Spoils of Victory prints *"a Plains, Island,
/// Swamp, Mountain, or Forest card"* — Wizards' older wording for a land
/// with a basic land type, which a Breeding Pool satisfies and a
/// `SupertypeSet::BASIC` filter would wrongly refuse. So the question is
/// asked of the **printing** and not of the filter: five subtypes are
/// right where the card names five types, and wrong where it says "basic
/// land". The card's own `//! Oracle:` header is that printing, and
/// `xtask validate` is what holds it to Scryfall.
///
/// Measured over the reference corpus: 369 scripts search for
/// `Land.Basic` and exactly **one** spells the five names — this card.
#[test]
fn no_card_spells_basic_land_as_the_five_basic_subtypes() {
    const SUBTYPES: [&str; 5] = ["PLAINS", "ISLAND", "SWAMP", "MOUNTAIN", "FOREST"];
    let mut offenders = Vec::new();
    for (name, text) in every_card_file() {
        let names_all_five = SUBTYPES
            .iter()
            .all(|s| text.contains(&format!("HasSubtype(land::{s})")))
            || SUBTYPES
                .iter()
                .all(|s| text.contains(&format!("HasSubtype(subtypes::land::{s})")));
        let printed: String = text
            .lines()
            .filter(|l| l.starts_with("//! Oracle:"))
            .collect::<Vec<_>>()
            .join(" ")
            .to_uppercase();
        // Boseiju, Who Endures prints the modern wording of the same
        // thing Spoils of Victory lists: "a land card with a basic land
        // type" (CR 205.3i), which a Breeding Pool satisfies too.
        let prints_all_five = SUBTYPES.iter().all(|s| printed.contains(s))
            || printed.contains("WITH A BASIC LAND TYPE");
        if names_all_five && !prints_all_five {
            offenders.push(name);
        }
    }
    assert!(
        offenders.is_empty(),
        "these files spell \"basic land\" as five subtypes and print no \
         such list — use `Filter::HasSupertype(SupertypeSet::BASIC)`: \
         {offenders:?}"
    );
    // The rule is only a rule if the other half of it is reachable: the
    // card that legitimately writes the five has to exist, or this has
    // quietly become a test of nothing.
    let named: Vec<String> = every_card_file()
        .into_iter()
        .filter(|(_, text)| {
            SUBTYPES
                .iter()
                .all(|s| text.contains(&format!("HasSubtype(subtypes::land::{s})")))
        })
        .map(|(name, _)| name)
        .collect();
    assert!(
        !named.is_empty(),
        "no card in the pool writes the five subtypes at all, so the \
         printing half of this test is unexercised"
    );
}

