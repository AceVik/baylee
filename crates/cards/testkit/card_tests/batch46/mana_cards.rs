//! Five mana cards played through production, spending and linked choices.
use baylee_card_testkit::card_tests::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::*;
use baylee_cards_dsl::{Filter, ManaSource};
use baylee_core::generated::{index, subtypes};
use baylee_core::mana::{ManaCost, ManaPool};
use baylee_engine::event::Cause;
use baylee_engine::zone::ZonePosition;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

mod avengers;
mod beacon;
mod pit;
mod plaza;
mod spending;

fn pool(e: &mut Engine<RegistryLookup>, amounts: &[(ManaColor, u32)]) {
    let mana = &mut e.state.players[0].mana_pool;
    *mana = ManaPool::new();
    for &(color, amount) in amounts {
        mana.add(color, amount);
    }
    e.refresh_offer();
}

fn source(e: &Engine<RegistryLookup>, card: CardIndex) -> ObjectId {
    on_battlefield(e, P0, card).expect("fixture permanent")
}

fn ability(e: &mut Engine<RegistryLookup>, source: ObjectId, ability_index: u32) {
    e.refresh_offer();
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        },
    )
    .unwrap();
}

fn color(e: &mut Engine<RegistryLookup>, color: ManaColor) {
    e.apply(P0, PlayerAction::ChooseColor(color)).unwrap();
}

fn colors(e: &Engine<RegistryLookup>) -> Vec<ManaColor> {
    let Pending::ChooseColor { options, .. } = e.pending() else {
        panic!("{:?}", e.pending());
    };
    options.clone()
}

fn choose(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    let player = e.pending().asked().unwrap();
    e.apply(player, PlayerAction::ChooseObjects { objects })
        .unwrap();
}

fn no_mana_to_cast(e: &Engine<RegistryLookup>, card: ObjectId) {
    assert_eq!(
        baylee_engine::casting::can_cast(e.state(), &RegistryLookup, P0, card),
        Err(baylee_engine::casting::CastError::NotEnoughMana)
    );
}
