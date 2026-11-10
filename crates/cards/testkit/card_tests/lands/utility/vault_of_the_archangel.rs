//! `cards/lands/utility/vault_of_the_archangel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vault of the Archangel: "{T}: Add {C}." / "{2}{W}{B}, {T}: Creatures you control gain deathtouch and lifelink until end of turn."
/// Paid with four basic lands, the activated ability resolves to grant all controlled creatures both keywords.
/// Llanowar Elves gains deathtouch and lifelink, and the land remains tapped.
#[test]
fn vault_of_the_archangel_grants_deathtouch_and_lifelink_to_creatures() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(125, forest())
        .battlefield(
            0,
            &[
                vault_of_the_archangel(),
                plains(),
                swamp(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vault = on_battlefield(&engine, p0, vault_of_the_archangel()).expect("Vault deployed");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("Elves deployed");
    assert!(!keywords(&engine, elves).contains(KeywordSet::DEATHTOUCH));
    assert!(!keywords(&engine, elves).contains(KeywordSet::LIFELINK));

    tap_mana_except(&mut engine, p0, vault);
    activate(&mut engine, p0, vault_of_the_archangel(), 1);

    pass_until(&mut engine, stack_is_empty);

    let kw = keywords(&engine, elves);
    assert!(kw.contains(KeywordSet::DEATHTOUCH));
    assert!(kw.contains(KeywordSet::LIFELINK));
    assert!(is_tapped(&engine, vault));
}
