//! `cards/lands/legendary/barad_dur.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Barad-dûr enters tapped **unless you control a legendary creature**,
/// which is the rarer half of the checkland sentence: the usual one looks at
/// land types, and a reader that matched on a supertype without also
/// requiring the card type would be satisfied by Barad-dûr itself, which is
/// a legendary land sitting right there. So the negative branch has the land
/// alone on an otherwise empty board, and the positive one adds
/// Jin-Gitaxias.
///
/// The file is `Coverage::Partial` for the {X}{X}{B} amass ability, and
/// nothing here reaches it: `Effect::Amass` carries a fixed number where the
/// card prints X, and no `Condition` says a creature died this turn.
#[test]
fn barad_dur_checks_for_a_legendary_creature_and_not_for_itself() {
    let p0 = PlayerId::new(0);

    let mut bare = Duel::new(SEED, forest()).hand(0, &[barad_dur()]).start();
    keep_mulligans(&mut bare);
    assert!(walk_to_own_main(&mut bare, p0), "p0 reaches its own main");
    let tapped = play_land(&mut bare, p0, barad_dur());
    assert!(
        entered_tapped(&bare, tapped),
        "a legendary *land* is not a legendary creature, not even this one"
    );

    let mut held = Duel::new(SEED, forest())
        .battlefield(0, &[jin_gitaxias()])
        .hand(0, &[barad_dur()])
        .start();
    keep_mulligans(&mut held);
    assert!(walk_to_own_main(&mut held, p0), "p0 reaches its own main");
    let untapped = play_land(&mut held, p0, barad_dur());
    assert!(
        !entered_tapped(&held, untapped),
        "Jin-Gitaxias is a legendary creature and turns the clause off"
    );

    activate(&mut held, p0, barad_dur(), 0);
    assert_eq!(
        held.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "{{T}}: Add {{B}}"
    );
}
