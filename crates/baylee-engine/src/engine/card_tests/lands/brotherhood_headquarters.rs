//! `cards/lands/brotherhood_headquarters.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Brotherhood Headquarters is a land and `Coverage::Partial`: both printed
/// mana abilities are built, and what is missing is two thirds of the second
/// one's *spend restriction* — "a spell that has freerunning" and "an ability
/// of an Assassin source" are unreachable, so the mana is restricted to the
/// Assassin-spell half alone, which is narrower than the printing and never
/// wider.
///
/// This leg plays the land as a real land drop, reads the offer to show that
/// **both** routes are there, and taps for the colourless half. The
/// restriction itself is the sibling test below, because both abilities cost
/// `{T}` and one land can only be asked once.
#[test]
fn brotherhood_headquarters_offers_both_printed_mana_abilities() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(813, forest())
        .hand(0, &[brotherhood_headquarters()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, brotherhood_headquarters());
    assert_eq!(
        on_battlefield(&engine, p0, brotherhood_headquarters()),
        Some(land),
        "the land drop put it on the battlefield"
    );
    assert!(
        !is_tapped(&engine, land),
        "nothing on the card makes it enter tapped"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "playing the land itself costs nothing"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("playing a land hands priority back: {:?}", engine.pending())
    };
    let offered = deeds(&legal, &[land]);
    assert!(
        matches!(offered[..], [(0, Deed::Ability(0)), (0, Deed::Ability(1))]),
        "both printed mana abilities are offered — the colourless one and the \
         any-colour one whose mana is restricted: {offered:?}"
    );

    activate(&mut engine, p0, brotherhood_headquarters(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}}, the half that is written"
    );
    assert_eq!(pool.total(), 1, "and that is the whole of it");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "and the seat holds priority again, the colourless half having asked \
         no colour: {:?}",
        engine.pending()
    );
}

/// The restriction is the half worth proving, because restricted mana that
/// landed in the ordinary pool would be a land strictly better than its
/// printing.
///
/// `ManaPool` keeps the two apart — `available(colour)` counts the plain
/// counters and restricted mana lives in its own list — so what says the
/// restriction is real is that the chosen colour is **not** available while
/// the restricted list holds exactly it.
#[test]
fn brotherhood_headquarters_any_colour_mana_lands_restricted() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(814, forest())
        .hand(0, &[brotherhood_headquarters()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, brotherhood_headquarters());
    activate(&mut engine, p0, brotherhood_headquarters(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("\"add one mana of any color\" asks: {:?}", engine.pending())
    };
    assert_eq!(options.len(), 5, "any colour is all five");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.restricted().len(), 1, "one restricted mana was made");
    assert_eq!(
        pool.restricted()[0].color,
        ManaColor::Blue,
        "the colour that was asked for"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "and it is not in the ordinary pool — mana that could pay for anything \
         is the card this land is not"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
