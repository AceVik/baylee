//! The X question of a spell whose cost the board pays beside mana: convoke's
//! creatures (CR 702.51a) and the restricted mana the offer names for the
//! cast (CR 106.6) count toward the cap the client puts on X
//! (`Duel::cap_x`), as the engine counts them.

use super::*;
use baylee_client_core::test_support::{ViewBuilder, token};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::land;
use baylee_core::types::TypeSet;
use baylee_engine::choice::LegalActions;

const ME: PlayerId = PlayerId::new(0);

fn obj(slot: u32) -> ObjectId {
    ObjectId::new(slot, 0)
}

fn forest(slot: u32) -> baylee_view::PublicObject {
    let mut o = token(slot, 0, "Forest", 0, 0);
    o.types = TypeSet::LAND;
    o.power = None;
    o.toughness = None;
    o.subtypes.insert(land::FOREST);
    o
}

fn elf(slot: u32) -> baylee_view::PublicObject {
    let mut o = token(slot, 0, "Elf", 1, 1);
    o.types = TypeSet::CREATURE;
    o.colors = ColorSet::of(Color::Green);
    o
}

/// Carbon b4, attempt 8, q158, as the client would have asked it: Chord of
/// Calling (`{X}{G}{G}{G}`, convoke) beside five Forests and three green
/// creatures pays X = 5; the cap that counted the Forests alone stopped at
/// 2.
#[test]
fn a_convoke_spells_x_is_capped_by_the_lands_and_the_creatures() {
    let chord = baylee_cards::decks::by_name("Chord of Calling").expect("Chord of Calling");
    let view = || {
        let mut view = ViewBuilder::new(2)
            .with_hand(vec![("Chord of Calling", 3, 20)])
            .with_battlefield(
                0,
                [
                    forest(31),
                    forest(32),
                    forest(33),
                    forest(34),
                    forest(35),
                    elf(41),
                    elf(42),
                    elf(43),
                ],
            )
            .build();
        view.hand[0].card.index = chord;
        view
    };
    let priority = Pending::Priority {
        player: ME,
        legal: Box::new(LegalActions {
            can_pass: true,
            castable: vec![obj(20)],
            mana_abilities: (31..=35).map(obj).collect(),
            ..LegalActions::default()
        }),
    };
    let mut duel = Duel::default();
    duel.receive_view(view());
    duel.receive_choice(priority.clone());
    duel.receive_choice(priority);
    let mut casting = view();
    casting.casting = Some(obj(20));
    duel.receive_view(casting);
    duel.receive_choice(Pending::ChooseNumber {
        player: ME,
        min: 0,
        max: u32::MAX,
        reason: baylee_engine::choice::NumberPrompt::X,
    });
    assert_eq!(
        duel.interaction.as_mut().unwrap().set_number(100),
        5,
        "five Forests and three convoking Elves pay X = 5"
    );
}

const BLAZE: u32 = 20;
const ARCANIST: u32 = 41;

fn mountain(slot: u32) -> baylee_view::PublicObject {
    let mut o = token(slot, 0, "Mountain", 0, 0);
    o.types = TypeSet::LAND;
    o.power = None;
    o.toughness = None;
    o.subtypes.insert(land::MOUNTAIN);
    o
}

/// Vodalian Arcanist off the registry: "{T}: Add {C}. Spend this mana only
/// to cast an instant or sorcery spell."
fn arcanist(slot: u32) -> baylee_view::PublicObject {
    let name = "Vodalian Arcanist";
    let index = baylee_cards::decks::by_name(name).expect("Vodalian Arcanist");
    let mut o = baylee_client_core::test_support::printed(slot, 0, name, 0);
    o.card = Some(baylee_view::CardIdentity {
        index,
        print: baylee_core::ids::PrintRef::new(0),
        face: 0,
    });
    o.rules = o.card.map(baylee_view::RulesFace::from);
    o.types = TypeSet::CREATURE;
    o
}

/// The offer Blaze is cast from: two Mountains, Vodalian Arcanist's
/// restricted `{C}` named for `spends`, and restricted mana floating that
/// `floating` names.
fn blaze_offer(
    spends: Vec<baylee_engine::choice::Spend>,
    floating: Vec<(baylee_engine::choice::Spend, [u32; 6])>,
) -> LegalActions {
    LegalActions {
        can_pass: true,
        castable: vec![obj(BLAZE)],
        mana_abilities: vec![obj(31), obj(32)],
        abilities: vec![(obj(ARCANIST), 0)],
        restricted_taps: vec![baylee_engine::choice::RestrictedTaps {
            taps: vec![(obj(ARCANIST), 0)],
            spends,
        }],
        restricted_floating: floating,
        ..LegalActions::default()
    }
}

fn blaze_spend() -> baylee_engine::choice::Spend {
    baylee_engine::choice::Spend::Cast(obj(BLAZE), baylee_engine::choice::CastModeKind::Normal)
}

/// The X the client lets Blaze (`{X}{R}`) be cast at, out of `legal`, with
/// `floating` restricted colourless mana in the pool.
fn blaze_cap(legal: LegalActions, floating: u64) -> u32 {
    let blaze = baylee_cards::decks::by_name("Blaze").expect("Blaze");
    let view = || {
        let mut view = ViewBuilder::new(2)
            .with_hand(vec![("Blaze", 2, BLAZE)])
            .with_battlefield(0, [mountain(31), mountain(32), arcanist(ARCANIST)])
            .build();
        view.hand[0].card.index = blaze;
        view.seats[0].mana_pool.restricted[baylee_core::mana::ManaColor::Colorless.index()] =
            floating;
        view
    };
    let priority = Pending::Priority {
        player: ME,
        legal: Box::new(legal),
    };
    let mut duel = Duel::default();
    duel.receive_view(view());
    duel.receive_choice(priority.clone());
    duel.receive_choice(priority);
    let mut casting = view();
    casting.casting = Some(obj(BLAZE));
    duel.receive_view(casting);
    duel.receive_choice(Pending::ChooseNumber {
        player: ME,
        min: 0,
        max: u32::MAX,
        reason: baylee_engine::choice::NumberPrompt::X,
    });
    duel.interaction.as_mut().unwrap().set_number(100)
}

/// Carbon b7, attempt 22, q214, as the client would have asked it: restricted
/// mana the offer names for this cast pays its X (CR 106.6). Two Mountains
/// pay Blaze at X = 1; Vodalian Arcanist's `{C}`, spendable on a sorcery,
/// makes it 2.
#[test]
fn restricted_mana_the_offer_names_for_the_cast_raises_its_x() {
    assert_eq!(
        blaze_cap(blaze_offer(vec![blaze_spend()], Vec::new()), 0),
        2,
        "the Mountains and the Arcanist's colourless pay X = 2"
    );
}

/// The Arcanist's mana named for some other payment pays nothing here, as it
/// pays nothing when Blaze's cost is paid.
#[test]
fn restricted_mana_named_for_another_payment_pays_no_x() {
    let elsewhere =
        baylee_engine::choice::Spend::Cast(obj(99), baylee_engine::choice::CastModeKind::Normal);
    assert_eq!(
        blaze_cap(blaze_offer(vec![elsewhere], Vec::new()), 0),
        1,
        "the Mountains alone pay X = 1"
    );
}

/// Restricted mana already floating counts where the offer names it for
/// this cast, and not where it names it for nothing.
#[test]
fn restricted_mana_floating_counts_where_the_offer_names_it_for_the_cast() {
    let mut colourless = [0; 6];
    colourless[baylee_core::mana::ManaColor::Colorless.index()] = 1;
    assert_eq!(
        blaze_cap(
            blaze_offer(Vec::new(), vec![(blaze_spend(), colourless)]),
            1
        ),
        2,
        "the floating colourless pays one more X"
    );
    assert_eq!(
        blaze_cap(blaze_offer(Vec::new(), Vec::new()), 1),
        1,
        "floating mana the offer names for no payment pays nothing"
    );
}
