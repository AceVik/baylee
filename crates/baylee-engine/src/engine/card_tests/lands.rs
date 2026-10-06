//! Lands, the door `cards/lands/` puts them behind -- and the lands are
//! where most of the engine's mana arithmetic is actually played.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;
use crate::choice::ChoicePrompt;
use baylee_cards_dsl::counters;
use baylee_cards_dsl::{Effect, Find, SearchDest};

mod an_havva_township;
mod ancient_ziggurat;
mod archaeological_dig;
mod artifacts;
mod aysen_abbey;
mod basic;
mod battle;
mod bounce;
mod brotherhood_headquarters;
mod cabal_coffers;
mod cascading_cataracts;
mod castle_sengir;
mod caves;
mod check;
mod corrupted_crossroads;
mod creatures;
mod crowd;
mod crystal_quarry;
mod crystal_vein;
mod cycling;
mod deserts;
mod dual;
mod eclipsed_realms;
mod eldrazi_temple;
mod exotic_orchard;
mod fetch;
mod filter;
mod gaea_s_cradle;
mod gain;
mod gates;
mod gemstone_caverns;
mod gemstone_mine;
mod great_hall_of_the_citadel;
mod grove_of_the_burnwillows;
mod guildmages_forum;
mod henge_of_ramos;
mod horizon;
mod koskun_keep;
mod lake_of_the_dead;
mod legendary;
mod loci;
mod lotus_vale;
mod mana_confluence;
mod manlands;
mod mines;
mod mishra_s_workshop;
mod muraganda_raceway;
mod nephalia_academy;
mod no_untap;
mod opal_palace;
mod pain;
mod pathway;
mod phyrexian_tower;
mod pillar_of_the_paruns;
mod planar_nexus;
mod planets;
mod r_d_s_secret_lair;
mod rainbow_vale;
mod reflecting_pool;
mod refuge;
mod reliquary_tower;
mod restricted;
mod reveal;
mod riftstone_portal;
mod river_of_tears;
mod saddle;
mod school_of_the_unseen;
mod scorched_ruins;
mod scry;
mod secluded_courtyard;
mod secret_base;
mod shimmering_grotto;
mod shock;
mod slow;
mod spheres;
mod storage;
mod study_hall;
mod surveil;
mod tapland;
mod tendo_ice_bridge;
mod the_tabernacle_at_pendrell_vale;
mod tournament_grounds;
mod towers;
mod towns;
mod triome;
mod unclaimed_territory;
mod underdome;
mod undiscovered_paradise;
mod unknown_shores;
mod unlucky;
mod urza_s_power_plant;
mod utility;
mod vesuva;
mod watermarket;
mod white_lotus_hideout;
mod wizards_school;
mod yavimaya_cradle_of_growth;
mod zoetic_cavern;

// oracle_id = "a3da7d5b-2c2b-45fe-b9c5-413b8c8fc0a2"
fn academy_ruins() -> CardIndex {
    card_index("a3da7d5b-2c2b-45fe-b9c5-413b8c8fc0a2")
}

/// Walks to `seat`'s **next** first main phase, across the turn in between.
///
/// Neither of the two walkers already here can do it. `walk_to_own_main`
/// answers "we are there" at once when the game is standing in that very
/// phase, and `pass_until` has no arm for the discard the opponent owes at
/// their own cleanup — seven cards kept plus the draw of their turn is eight,
/// and the walk dies on a question it cannot answer. So the one thing an
/// untap step is needed for, a land that spent its `{T}` last turn, had no
/// road to it. `answer_one` is the shared driver that does have both arms.
#[track_caller]
fn cross_into_the_next_own_main(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let from = engine.state().turn.number;
    for _ in 0..200 {
        if engine.state().turn.number > from
            && matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == seat
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
        {
            return;
        }
        let (player, action) = answer_one(engine).expect("a rest on the way to the next turn");
        engine.apply(player, action).expect("the answer is legal");
    }
    panic!("never reached {seat:?}'s next main phase");
}

// oracle_id = "3644f316-f9a3-46c9-9b1e-747f86cf4ead"
fn buried_ruin() -> CardIndex {
    card_index("3644f316-f9a3-46c9-9b1e-747f86cf4ead")
}

/// Puts a named card out of `seat`'s hand into `seat`'s graveyard, and
/// answers with the object it became.
///
/// [`seed_graveyard`] takes whatever is on top of the library, which is the
/// filler printing and nothing else. A test that has to tell an artifact card
/// in the graveyard from a creature card beside it needs to name both, so it
/// deals them into the opening hand and buries them by name. The id is
/// handed back because an object changes id when it changes zone (CR 400.7),
/// and the id a target list is compared against has to be the one the card
/// has *in the graveyard*.
#[track_caller]
fn bury_from_hand(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
) -> ObjectId {
    let held = in_hand(engine, seat, card).expect("the card starts in hand");
    let buried = engine
        .dev_state_mut(seat)
        .expect("the harness may set boards up")
        .move_object(
            held,
            ZoneLocation::Graveyard(seat),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("the harness moves a card");
    // The offer standing in `pending` was computed before this, and an
    // ability that reads a graveyard is withheld while no graveyard holds
    // what it needs.
    engine.refresh_offer();
    buried
}

// oracle_id = "e996cd67-739c-40f4-b276-0042acf26c71"
/// Dryad Arbor, the pool's one Land Creature, under a name of its own: the
/// fixture of the same card in `instants.rs` is a sibling module's private
/// item and reaches nothing here.
fn the_land_creature() -> CardIndex {
    card_index("e996cd67-739c-40f4-b276-0042acf26c71")
}

// oracle_id = "1861e642-21d5-4232-89f3-b5557f2946c1"
fn phyrexian_tower() -> CardIndex {
    card_index("1861e642-21d5-4232-89f3-b5557f2946c1")
}

// oracle_id = "152e7e91-4eda-4e72-a9fb-bd5cb2e68239"
fn survivors_encampment() -> CardIndex {
    card_index("152e7e91-4eda-4e72-a9fb-bd5cb2e68239")
}

// oracle_id = "e6b77545-de5c-4f4a-b7ea-83498fb33ba8"
fn holdout_settlement() -> CardIndex {
    card_index("e6b77545-de5c-4f4a-b7ea-83498fb33ba8")
}

// oracle_id = "ba11a517-1dbd-4797-9f5e-46ce0f6c77c0"
fn scene_of_the_crime() -> CardIndex {
    card_index("ba11a517-1dbd-4797-9f5e-46ce0f6c77c0")
}

// oracle_id = "d8e2efe0-33a4-4303-9e83-ac42ea5df8cb"
fn vivid_crag() -> CardIndex {
    card_index("d8e2efe0-33a4-4303-9e83-ac42ea5df8cb")
}

// oracle_id = "2da7c49f-cc1e-45d9-9cbf-067e92b0daef"
fn vivid_creek() -> CardIndex {
    card_index("2da7c49f-cc1e-45d9-9cbf-067e92b0daef")
}

// oracle_id = "b7a68899-c0d3-49e0-854b-19268ae9b89d"
fn vivid_grove() -> CardIndex {
    card_index("b7a68899-c0d3-49e0-854b-19268ae9b89d")
}

// oracle_id = "20b32052-f66f-4eb8-b56e-00d531907f19"
fn vivid_marsh() -> CardIndex {
    card_index("20b32052-f66f-4eb8-b56e-00d531907f19")
}

// oracle_id = "dee99df5-628f-4a4e-a203-4dfddc927373"
fn vivid_meadow() -> CardIndex {
    card_index("dee99df5-628f-4a4e-a203-4dfddc927373")
}

// oracle_id = "9e006a4b-8dde-4416-8cb4-8401562d0fd5"
fn tendo_ice_bridge() -> CardIndex {
    card_index("9e006a4b-8dde-4416-8cb4-8401562d0fd5")
}

// oracle_id = "ea53adbe-3f9a-4847-87c7-723ac2789918"
fn mirrodin_s_core() -> CardIndex {
    card_index("ea53adbe-3f9a-4847-87c7-723ac2789918")
}

// oracle_id = "01546b7d-a233-4176-8843-d732074dc5b6"
fn doubling_season() -> CardIndex {
    card_index("01546b7d-a233-4176-8843-d732074dc5b6")
}

// oracle_id = "26259c65-8f4e-42a6-b8a7-f65c36d35c4d"
fn hickory_woodlot() -> CardIndex {
    card_index("26259c65-8f4e-42a6-b8a7-f65c36d35c4d")
}

// oracle_id = "a176924c-78fc-4151-b2b5-1547b1114a40"
fn peat_bog() -> CardIndex {
    card_index("a176924c-78fc-4151-b2b5-1547b1114a40")
}

// oracle_id = "2c38f4c7-1b3f-42b4-a175-edab7acd6cc6"
fn remote_farm() -> CardIndex {
    card_index("2c38f4c7-1b3f-42b4-a175-edab7acd6cc6")
}

// oracle_id = "c8e0a1a5-8188-4677-9d8a-a18eb593343a"
fn sandstone_needle() -> CardIndex {
    card_index("c8e0a1a5-8188-4677-9d8a-a18eb593343a")
}

// oracle_id = "e4e6e796-39ce-4a63-8c61-c7c956d75d78"
fn saprazzan_skerry() -> CardIndex {
    card_index("e4e6e796-39ce-4a63-8c61-c7c956d75d78")
}

// oracle_id = "0c828f10-4775-492f-9224-1e2814ad2cad"
fn gemstone_mine() -> CardIndex {
    card_index("0c828f10-4775-492f-9224-1e2814ad2cad")
}

// oracle_id = "0799df10-b489-4f79-bf98-7a0c500b46a1"
fn fountain_of_cho() -> CardIndex {
    card_index("0799df10-b489-4f79-bf98-7a0c500b46a1")
}

// oracle_id = "136596a0-b179-40be-b42d-c0b992621c95"
fn mage_ring_network() -> CardIndex {
    card_index("136596a0-b179-40be-b42d-c0b992621c95")
}

// oracle_id = "f7dda04a-c9c6-4952-9bbc-87e3c7480347"
fn saprazzan_cove() -> CardIndex {
    card_index("f7dda04a-c9c6-4952-9bbc-87e3c7480347")
}

// oracle_id = "0bbd5a04-c281-4afb-98a1-657b4eca102c"
fn subterranean_hangar() -> CardIndex {
    card_index("0bbd5a04-c281-4afb-98a1-657b4eca102c")
}

// oracle_id = "b02ab3c7-fe4a-443c-b860-ba971d3301b0"
fn mercadian_bazaar() -> CardIndex {
    card_index("b02ab3c7-fe4a-443c-b860-ba971d3301b0")
}

// oracle_id = "ccb2f92e-69c0-415c-81cd-52c384b3b233"
fn rushwood_grove() -> CardIndex {
    card_index("ccb2f92e-69c0-415c-81cd-52c384b3b233")
}

/// Activates a storage land's banking line and lets it resolve.
///
/// `{T}: Put a storage counter on this land` produces no mana, so it is not
/// a mana ability (CR 605.1a) and it uses the stack — the counter is not
/// there until it resolves. That is the reason this is a helper and not a
/// bare `apply`: a test that read the count straight after the press would
/// be reading the board before the ability had done anything, and would
/// then "prove" the storing line broken on every card that has one.
///
/// It also asserts what the press did *not* do. Banking names its own
/// number, so the engine must come straight back to priority — a
/// `ChooseNumber` here would mean the question had attached itself to the
/// permanent rather than to the cost part that announces one.
#[track_caller]
fn store_a_counter(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    land: ObjectId,
    ability_index: u32,
) {
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index,
            },
        )
        .expect("an untapped land may bank a counter");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "banking a counter announces nothing: {:?}",
        engine.pending()
    );
    pass_until(engine, stack_is_empty);
}

/// Presses the storage line and answers `x`, returning the bound the engine
/// offered.
///
/// It is written as one step because the two halves are one decision: the
/// bound is the only thing the question carries, so a test that read it
/// without answering, or answered without reading it, would be asserting
/// half of what happened.
#[track_caller]
fn spend_storage(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    land: ObjectId,
    ability_index: u32,
    x: u32,
) -> (u32, u32) {
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index,
            },
        )
        .expect("the tap is the only part of this cost that can be refused");
    let Pending::ChooseNumber {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!(
            "`Remove any number of storage counters` names no number: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, seat, "the activating player announces the number");
    engine
        .apply(seat, PlayerAction::ChooseNumber(x))
        .expect("a number inside the offered range");
    (min, max)
}

// oracle_id = "2031bc31-81cc-407a-8615-29832f586bbc"
fn calciform_pools() -> CardIndex {
    card_index("2031bc31-81cc-407a-8615-29832f586bbc")
}

// oracle_id = "130a8cf5-1354-4d17-91c8-c073642eb3db"
fn dreadship_reef() -> CardIndex {
    card_index("130a8cf5-1354-4d17-91c8-c073642eb3db")
}

// oracle_id = "6f18ea44-3efa-4a45-abc6-86a0627e40f2"
fn fungal_reaches() -> CardIndex {
    card_index("6f18ea44-3efa-4a45-abc6-86a0627e40f2")
}

// oracle_id = "33587cb2-0fd3-4e4c-bc5e-e7299cc9dab5"
fn molten_slagheap() -> CardIndex {
    card_index("33587cb2-0fd3-4e4c-bc5e-e7299cc9dab5")
}

// oracle_id = "021e4165-2f02-4bd4-86ca-cb7bf4c9e23d"
fn saltcrusted_steppe() -> CardIndex {
    card_index("021e4165-2f02-4bd4-86ca-cb7bf4c9e23d")
}

// oracle_id = "d98b4250-3492-4864-9c4c-42db09b3ccd4"
fn cascading_cataracts() -> CardIndex {
    card_index("d98b4250-3492-4864-9c4c-42db09b3ccd4")
}

fn mystic_gate() -> CardIndex {
    card_index("e9f5feb2-2c1a-46ce-885a-4f378d7d10af")
}

fn fetid_heath() -> CardIndex {
    card_index("42bf259d-4bb9-49c3-b4ec-223dca62f4d6")
}

fn cascade_bluffs() -> CardIndex {
    card_index("f1603384-4361-49c9-98aa-7785fc3504c4")
}

fn sunken_ruins() -> CardIndex {
    card_index("e6415ffb-8b7a-41c3-bedf-0d4112b7b795")
}

fn flooded_grove() -> CardIndex {
    card_index("dc974eb4-72b9-4213-887b-8ee684b93420")
}

fn wooded_bastion() -> CardIndex {
    card_index("61b85077-64aa-4bcc-890d-2d88da9543c0")
}

fn fire_lit_thicket() -> CardIndex {
    card_index("d99a1d9a-7721-4331-bf22-1c6ee0bd825a")
}

fn rugged_prairie() -> CardIndex {
    card_index("8e7641e1-e814-4d5a-9cb3-71ad2f4ceee8")
}

fn graven_cairns() -> CardIndex {
    card_index("5004b84a-33b7-4f6f-b2c2-7086b9087535")
}

fn twilight_mire() -> CardIndex {
    card_index("db623754-e078-4030-ba07-818803c348a8")
}
fn cabal_coffers() -> CardIndex {
    card_index("7358e164-5704-4e78-9b21-6a9bf2a968ce")
}

fn cabal_stronghold() -> CardIndex {
    card_index("066cd584-773c-4623-be53-8f6feda5a26a")
}

fn serra_s_sanctum() -> CardIndex {
    card_index("34187c71-6033-4058-aadc-2bc266f762be")
}

fn tolarian_academy() -> CardIndex {
    card_index("dba4fd31-8931-42dd-bd86-45479c2abf74")
}

fn cloudpost() -> CardIndex {
    card_index("f705c0eb-9c6c-4315-a860-208ed0c5d93e")
}

fn glimmerpost() -> CardIndex {
    card_index("92c9aad6-35ec-425d-be7d-393328992820")
}

#[allow(clippy::too_many_arguments)] // a row of the table above, not a call site
fn one_counted_land(
    seed: u64,
    land_card: CardIndex,
    color: ManaColor,
    mine: &[CardIndex],
    theirs: &[CardIndex],
    generic: usize,
    ability_index: u32,
    want: u16,
) {
    let p0 = PlayerId::new(0);
    let mut seated = mine.to_vec();
    seated.push(land_card);
    // Forests pay the price where the card charges one, and a Forest is none
    // of the five things counted here — which is what lets the count be read
    // off one colour.
    seated.extend(std::iter::repeat_n(forest(), generic));
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &seated)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, land_card).expect("the land is on the table");

    let forests: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == p0 && o.card.is_some_and(|c| c.index == forest()))
        })
        .take(generic)
        .collect();
    for source in forests {
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: 0,
                },
            )
            .expect("a Forest taps for {G}");
    }

    let before = engine.state().players[0].mana_pool.available(color);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index,
            },
        )
        .expect("the price is paid and the land is untapped");
    let after = engine.state().players[0].mana_pool.available(color);
    assert_eq!(
        after - before,
        u32::from(want),
        "the count is the land's own filter, and nothing else on the board"
    );
}

fn baldurs_gate() -> CardIndex {
    card_index("da307ea2-4df7-4d6b-be0f-9dc6ac93db61")
}

fn azorius_guildgate() -> CardIndex {
    card_index("ad1712d8-809f-410c-8b91-ffe6fb8a69a1")
}

fn boros_guildgate() -> CardIndex {
    card_index("73c423b7-cab8-4e69-8070-9edbf96a6c2c")
}

fn one_filter_land(gate_card: CardIndex, basic_card: CardIndex, colors: [ManaColor; 2]) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(919, forest())
        .battlefield(0, &[gate_card, gate_card, basic_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gates: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == gate_card))
        })
        .collect();
    let [gate, other] = gates[..] else {
        panic!("two of the land were seated, found {}", gates.len())
    };
    let basic = on_battlefield(&engine, p0, basic_card).expect("the basic is on the table");

    // The other Gate's own first ability, which is the colorless half of the
    // card: `{T}: Add {C}.`
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: other,
                ability_index: 0,
            },
        )
        .expect("a free tap for {C}");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );

    // {C} is a mana and it is not a *coloured* one, so it pays no half of
    // `{W/U}` (CR 107.4e, CR 202.2: colorless is not a color).
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: gate,
                    ability_index: 1,
                },
            )
            .is_err(),
        "a filter land whose price is generic filters for free; this one \
         charges {{{:?}/{:?}}}",
        colors[0],
        colors[1]
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: basic,
                ability_index: 0,
            },
        )
        .expect("the basic taps for the colour the price names");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: gate,
                ability_index: 1,
            },
        )
        .expect("and that colour is one of the two halves of the hybrid");

    for (i, color) in colors.into_iter().enumerate() {
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!("pick {i} of two: {:?}", engine.pending())
        };
        assert_eq!(
            options,
            colors.to_vec(),
            "pick {i}: the card's own two colours, in its order"
        );
        engine
            .apply(p0, PlayerAction::ChooseColor(color))
            .expect("a colour the engine offered");
    }

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        (
            pool.available(colors[0]),
            pool.available(colors[1]),
            pool.available(ManaColor::Colorless),
        ),
        (1, 1, 1),
        "one mana went in and two came out, one of each — and the {{C}} \
         that could not pay the price is still floating"
    );
}

// oracle_id = "9f12bf9a-6e1a-4377-b4af-e8cabd3ee58a"
fn deserted_temple() -> CardIndex {
    card_index("9f12bf9a-6e1a-4377-b4af-e8cabd3ee58a")
}

// oracle_id = "e43413e4-be17-49af-978a-26210d05f52a"
fn bottomless_vault() -> CardIndex {
    card_index("e43413e4-be17-49af-978a-26210d05f52a")
}

// oracle_id = "4a6625bd-3dd2-45f1-8dc9-034c833fa90c"
fn dwarven_hold() -> CardIndex {
    card_index("4a6625bd-3dd2-45f1-8dc9-034c833fa90c")
}

// oracle_id = "3348df85-e61c-47b5-857d-c79befb38a8a"
fn hollow_trees() -> CardIndex {
    card_index("3348df85-e61c-47b5-857d-c79befb38a8a")
}

// oracle_id = "87a0e0b9-6c2d-47a4-a3ed-7e0ae62fbffc"
fn icatian_store() -> CardIndex {
    card_index("87a0e0b9-6c2d-47a4-a3ed-7e0ae62fbffc")
}

// oracle_id = "48a830f1-8965-4f97-b3d8-ca98eab1ba33"
fn sand_silos() -> CardIndex {
    card_index("48a830f1-8965-4f97-b3d8-ca98eab1ba33")
}

/// Walks until the untap step asks which permanents stay tapped, and hands
/// the question back unanswered.
///
/// `answer_one` answers this one by untapping, which is the right reading
/// for a driver on its way past — and wrong for a test whose subject it is.
/// So the check comes first, before anything is applied.
#[track_caller]
fn walk_to_the_untap_question(engine: &mut Engine<RegistryLookup>) -> (PlayerId, Vec<ObjectId>) {
    for _ in 0..200 {
        if let Pending::ChooseCards {
            player,
            options,
            prompt: ChoicePrompt::LeaveTapped,
            ..
        } = engine.pending().clone()
        {
            return (player, options);
        }
        let (player, action) = answer_one(engine).expect("a rest on the way to the untap step");
        engine.apply(player, action).expect("the answer is legal");
    }
    panic!("the untap step never asked");
}

/// Walks to `seat`'s main phase **of the turn it is already in**.
///
/// [`cross_into_the_next_own_main`] waits for the turn number to change,
/// which is one turn too far from inside the untap step of the turn that
/// matters.
#[track_caller]
fn on_to_this_turn_s_main(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    for _ in 0..200 {
        if matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == seat
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
        {
            return;
        }
        let (player, action) = answer_one(engine).expect("a rest on the way to the main phase");
        engine.apply(player, action).expect("the answer is legal");
    }
    panic!("never reached this turn's main phase");
}

// oracle_id = "977c2f33-b622-4172-9efb-7f523becd32b"
fn blazemire_verge() -> CardIndex {
    card_index("977c2f33-b622-4172-9efb-7f523becd32b")
}
// oracle_id = "b2eb7a64-a307-4a78-a25d-63fb3ae1e237"
fn cryptic_caves() -> CardIndex {
    card_index("b2eb7a64-a307-4a78-a25d-63fb3ae1e237")
}
// oracle_id = "f1e9abfb-c3c8-483e-b446-5c2afc9f6394"
fn floodfarm_verge() -> CardIndex {
    card_index("f1e9abfb-c3c8-483e-b446-5c2afc9f6394")
}
// oracle_id = "d71bda4c-3dee-4398-8fd0-f77d8743b887"
fn gloomlake_verge() -> CardIndex {
    card_index("d71bda4c-3dee-4398-8fd0-f77d8743b887")
}
// oracle_id = "cce328b9-6100-417e-9ddf-808bbe3e3bc5"
fn hushwood_verge() -> CardIndex {
    card_index("cce328b9-6100-417e-9ddf-808bbe3e3bc5")
}
// oracle_id = "d7e1d4eb-1d4e-460e-9304-7db9ab50ccb5"
fn nimbus_maze() -> CardIndex {
    card_index("d7e1d4eb-1d4e-460e-9304-7db9ab50ccb5")
}
// oracle_id = "2550099d-b3e2-4eb6-9f36-0fc412828ca6"
fn rivendell() -> CardIndex {
    card_index("2550099d-b3e2-4eb6-9f36-0fc412828ca6")
}
// oracle_id = "510a6ac5-f098-4145-ac07-771b1b6f7cdf"
fn riverpyre_verge() -> CardIndex {
    card_index("510a6ac5-f098-4145-ac07-771b1b6f7cdf")
}
// oracle_id = "55a519b4-61cb-448a-875b-4d6dbe00580f"
fn spire_of_industry() -> CardIndex {
    card_index("55a519b4-61cb-448a-875b-4d6dbe00580f")
}
// oracle_id = "a202276b-1f1b-4277-95ee-26877a204f5e"
fn sunbillow_verge() -> CardIndex {
    card_index("a202276b-1f1b-4277-95ee-26877a204f5e")
}
// oracle_id = "439de49b-1091-4688-9ffb-80a025df31c2"
fn tainted_field() -> CardIndex {
    card_index("439de49b-1091-4688-9ffb-80a025df31c2")
}
// oracle_id = "0222414f-98b5-458a-a0fd-831a66cd8b07"
fn tainted_isle() -> CardIndex {
    card_index("0222414f-98b5-458a-a0fd-831a66cd8b07")
}
// oracle_id = "b2bae7fc-0668-4b34-9cd6-0d80aea52275"
fn tainted_peak() -> CardIndex {
    card_index("b2bae7fc-0668-4b34-9cd6-0d80aea52275")
}
// oracle_id = "fa6d05a1-3df4-4751-b1a0-8d9693faec73"
fn tainted_wood() -> CardIndex {
    card_index("fa6d05a1-3df4-4751-b1a0-8d9693faec73")
}
// oracle_id = "cfdd5dc6-593e-495a-8cfe-3a56b3c4c7df"
fn temple_of_the_false_god() -> CardIndex {
    card_index("cfdd5dc6-593e-495a-8cfe-3a56b3c4c7df")
}
// oracle_id = "e861bc08-4f0b-4d22-9b85-9d20227fd5b4"
fn thornspire_verge() -> CardIndex {
    card_index("e861bc08-4f0b-4d22-9b85-9d20227fd5b4")
}
// oracle_id = "c6e0574c-3e2b-4c40-b17a-05bce3d49309"
fn wastewood_verge() -> CardIndex {
    card_index("c6e0574c-3e2b-4c40-b17a-05bce3d49309")
}
// oracle_id = "c52eaa87-9251-4a47-83fd-04e582ade612"
fn willowrush_verge() -> CardIndex {
    card_index("c52eaa87-9251-4a47-83fd-04e582ade612")
}

// oracle_id = "a3fb7228-e76b-4e96-a40e-20b5fed75685"
/// Whether printed ability `index` of `card` is on the table right now.
///
/// The mirror of [`activate`], which panics when it is not — and the half
/// this batch of lands actually needs, because "Activate only if …" is a
/// sentence about when the ability is **not** offered.
#[track_caller]
fn offered(engine: &Engine<RegistryLookup>, card: CardIndex, index: u32) -> bool {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    legal.abilities.iter().any(|(id, ai)| {
        *ai == index
            && engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
    })
}

// oracle_id = "bf1341dd-41a3-49f6-87ec-63170dde4324"
fn boseiju_who_endures() -> CardIndex {
    card_index("bf1341dd-41a3-49f6-87ec-63170dde4324")
}

/// Whether the Boseiju in `seat`'s hand offers its channel (ability 1).
fn boseiju_channel_offered(engine: &Engine<RegistryLookup>, seat: PlayerId) -> bool {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let hand = engine.state().zones.list(ZoneLocation::Hand(seat));
    legal.abilities.iter().any(|(source, index)| {
        *index == 1
            && hand.contains(source)
            && engine
                .state()
                .object(*source)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == boseiju_who_endures()))
    })
}

fn cephalid_coliseum() -> CardIndex {
    card_index("c733873e-77db-471f-8061-139db24f7e7c")
}

// oracle_id = "c0adbddc-b070-4c5f-afe0-0474c72a9251"
fn gemstone_caverns() -> CardIndex {
    card_index("c0adbddc-b070-4c5f-afe0-0474c72a9251")
}

// oracle_id = "91d4a5fe-fd6d-4b14-a63f-61b4d0ecd9c4"
fn inventors_fair() -> CardIndex {
    card_index("91d4a5fe-fd6d-4b14-a63f-61b4d0ecd9c4")
}

// oracle_id = "e9b6a394-691c-425a-9307-76d8edc7375e"
fn otawara_soaring_city() -> CardIndex {
    card_index("e9b6a394-691c-425a-9307-76d8edc7375e")
}

/// Walks to the surveil question and reads what it offers.
///
/// Four tests ask it, which is why it is a helper: "When this land enters,
/// surveil 1" is an ordinary trigger, so it uses the stack and every one of
/// them has to pass priority to it first — and a walk written four times is
/// a walk that drifts.
#[track_caller]
fn surveil_offer(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
) -> (Vec<ObjectId>, Vec<ArrangePile>) {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::Arrange {
                prompt: ArrangePrompt::Surveil,
                ..
            }
        )
    });
    let Pending::Arrange {
        player,
        cards,
        piles,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the walk above stops on nothing else");
    };
    assert_eq!(player, seat, "the seat that surveils is the seat asked");
    (cards, piles)
}

fn raucous_theater() -> CardIndex {
    card_index("04e5e84f-8fd4-43ab-8f9d-5b24646f7ae5")
}

// oracle_id = "ac2dd694-d2f1-4025-8400-12332bdc882a"
fn takenuma_abandoned_mire() -> CardIndex {
    card_index("ac2dd694-d2f1-4025-8400-12332bdc882a")
}

// oracle_id = "d2bcff58-7a8a-46ef-b6b3-39501d4c8e6e"
fn thundering_falls() -> CardIndex {
    card_index("d2bcff58-7a8a-46ef-b6b3-39501d4c8e6e")
}

// oracle_id = "08d80efc-9542-4ba2-824c-c8615d8d07f2"
fn undercity_sewers() -> CardIndex {
    card_index("08d80efc-9542-4ba2-824c-c8615d8d07f2")
}

fn idyllic_grange() -> CardIndex {
    card_index("23d349a0-e441-40b8-b634-13e61440a7c8")
}

fn dwarven_mine() -> CardIndex {
    card_index("74ed0bd3-ac31-41a4-8220-d8e7c8c1c437")
}

fn gingerbread_cabin() -> CardIndex {
    card_index("fa98c367-0312-49c6-abef-72e5ead4cc7d")
}

fn uncharted_haven() -> CardIndex {
    card_index("d23c3613-bc5e-4fc5-939c-62a090c53a79")
}

/// Every land in the pool printing "As it enters, choose a color" with no
/// exception, and the colour each one is told to make here.
///
/// One table rather than five tests, because they are one card with five
/// names: the enchantment and the Desert and the snow land differ in their
/// type line and in nothing this is about. The colours are spread across the
/// rows so all five are named at least once.
const ANY_COLOUR_LANDS: [(&str, &str, ManaColor); 5] = [
    (
        "d23c3613-bc5e-4fc5-939c-62a090c53a79",
        "Uncharted Haven",
        ManaColor::White,
    ),
    (
        "e103f422-85c0-43f8-8a2f-8b7863e503fa",
        "Mirage Mesa",
        ManaColor::Blue,
    ),
    (
        "660d44a2-391a-416c-b46c-ddcc3739f527",
        "Valgavoth's Lair",
        ManaColor::Black,
    ),
    (
        "2ac34f3e-822d-4fde-99ca-a31c4d9503fd",
        "Shimmerdrift Vale",
        ManaColor::Red,
    ),
    (
        "b26cfeb0-7bbe-4d93-8eed-e832f175a80c",
        "Crossroads Village",
        ManaColor::Green,
    ),
];

/// The other ten: every land printing "choose a color other than <c>" beside
/// "{T}: Add {c} or one mana of the chosen color".
///
/// Two cycles wearing one rule — five Gates and five Thriving lands — so the
/// row carries the colour the card prints (which is the one it may not be
/// told to make) and a second colour to name, walked around WUBRG so no two
/// rows ask the same pair.
const EXCLUDING_LANDS: [(&str, &str, ManaColor, ManaColor); 10] = [
    (
        "15f1fe23-5af4-4fc4-8cde-2e0bf9f9be0c",
        "Citadel Gate",
        ManaColor::White,
        ManaColor::Blue,
    ),
    (
        "b574c540-9f8a-4fd4-8809-d02c9b099ddc",
        "Sea Gate",
        ManaColor::Blue,
        ManaColor::Black,
    ),
    (
        "dde6bce5-8bbe-4866-b5aa-2c05c7d37241",
        "Black Dragon Gate",
        ManaColor::Black,
        ManaColor::Red,
    ),
    (
        "1999b5ac-21fb-4d99-ad72-58bf507f9a59",
        "Cliffgate",
        ManaColor::Red,
        ManaColor::Green,
    ),
    (
        "dd6e67c0-66a1-49b7-8a86-3cf4b209fd07",
        "Manor Gate",
        ManaColor::Green,
        ManaColor::White,
    ),
    (
        "d1946630-e224-40db-8f0d-388b09622288",
        "Thriving Heath",
        ManaColor::White,
        ManaColor::Black,
    ),
    (
        "69fc70b8-b143-4662-ac95-e2743037239d",
        "Thriving Isle",
        ManaColor::Blue,
        ManaColor::Red,
    ),
    (
        "bff416bb-d193-4c45-b2c1-7c297dbfad08",
        "Thriving Moor",
        ManaColor::Black,
        ManaColor::Green,
    ),
    (
        "91fceb34-0f2d-4392-be27-00dcd765637f",
        "Thriving Bluff",
        ManaColor::Red,
        ManaColor::White,
    ),
    (
        "a8052556-8962-4130-86a8-6fb7b6a324f7",
        "Thriving Grove",
        ManaColor::Green,
        ManaColor::Blue,
    ),
];

fn reflecting_pool() -> CardIndex {
    card_index("67f43ac6-2a58-4b53-b5d7-0330e2a252e2")
}

/// Every land in the pool whose *entry* surveils, minus the three with tests
/// of their own above.
///
/// The Murders at Karlov Manor duals and the three Deserts that follow them
/// print one sentence between them — "When this land enters, surveil 1" —
/// and a table is what says so. The `bin` column walks both answers down the
/// list, because "put it in the graveyard" and "leave it on top" are the two
/// halves of CR 701.25a and a surveil that ignored the answer would satisfy
/// either one alone.
const ENTRY_SURVEIL_LANDS: [(&str, &str, bool); 10] = [
    (
        "b33656ae-3473-4223-845f-f9147f87678b",
        "Commercial District",
        true,
    ),
    (
        "9ea747cf-5d04-4aa7-bdc3-8145860cd1ba",
        "Elegant Parlor",
        false,
    ),
    ("ca4b6689-04ee-4227-9bdc-cb5a9590c745", "Hedge Maze", true),
    (
        "d51831b1-7394-456e-a1de-6787a59f5932",
        "Lush Portico",
        false,
    ),
    (
        "ccfb8b4d-651c-418a-aa19-cb23105b3f2f",
        "Meticulous Archive",
        true,
    ),
    (
        "216a2a92-9ca3-4ca3-8af7-686c13b04290",
        "Shadowy Backstreet",
        false,
    ),
    (
        "840119bf-e60f-4ff7-9c9b-d420d09df545",
        "Underground Mortuary",
        true,
    ),
    (
        "37f924e1-7c25-4f06-88bb-054693a21e5a",
        "Conduit Pylons",
        false,
    ),
    (
        "382d18a2-438e-4ae7-a83f-1658ef1f9b07",
        "Hidden Grotto",
        true,
    ),
    (
        "a3648376-dc8b-409b-b2d1-c29e326a059c",
        "Surveillance Room",
        false,
    ),
];

/// Every land in the pool that surveils for a **cost**, with the number it
/// looks at and the basics that pay for it — one land per mana of the
/// printed cost, and the colours it names.
///
/// The fixture *is* the assertion about the cost. A transcoded `{2}{R}{W}`
/// that came out `{5}` or `{2}{G}{U}` would not be offered over this board
/// at all, and one that came out `{3}` would leave a mana floating, which
/// the test counts. None of that is visible over a board of twenty basics,
/// which is what this table replaced.
const COST_SURVEIL_LANDS: [(&str, &str, u8, &[ManaColor]); 12] = [
    (
        "a32e08fa-bea4-4ba9-a126-9bf0a91f67e2",
        "Fields of Strife",
        1,
        &[
            ManaColor::Red,
            ManaColor::White,
            ManaColor::Red,
            ManaColor::White,
        ],
    ),
    (
        "349ea6c7-6b3e-417f-b082-b712e2b1635b",
        "Forum of Amity",
        1,
        &[
            ManaColor::White,
            ManaColor::Black,
            ManaColor::White,
            ManaColor::Black,
        ],
    ),
    (
        "4eb428ab-f5b0-46ca-98dd-b3466a91ef97",
        "Kishla Village",
        2,
        &[ManaColor::Green; 4],
    ),
    (
        "676141c3-a433-4aba-86fb-729628f96dfa",
        "Ominous Asylum",
        1,
        &[ManaColor::Green; 4],
    ),
    (
        "638ff242-63d5-457d-a7a6-40ad51052e2e",
        "Paradox Gardens",
        1,
        &[
            ManaColor::Green,
            ManaColor::Blue,
            ManaColor::Green,
            ManaColor::Blue,
        ],
    ),
    (
        "1af15c1d-a41c-44cc-9614-d72694dd26e8",
        "Savage Mansion",
        1,
        &[ManaColor::Green; 4],
    ),
    (
        "80f08b47-a237-4efd-8d86-dfe35a816b0e",
        "Sinister Hideout",
        1,
        &[ManaColor::Green; 4],
    ),
    (
        "33a4e73d-d93a-4b6f-88ff-cd53f20d178c",
        "Spectacle Summit",
        1,
        &[
            ManaColor::Blue,
            ManaColor::Red,
            ManaColor::Blue,
            ManaColor::Red,
        ],
    ),
    (
        "6ef30340-a26d-49aa-bc86-0b8aa5252f87",
        "Suburban Sanctuary",
        1,
        &[ManaColor::Green; 4],
    ),
    (
        "595f0eb5-f521-4174-9c48-b89e85ea907c",
        "Titan's Grave",
        1,
        &[
            ManaColor::Black,
            ManaColor::Green,
            ManaColor::Black,
            ManaColor::Green,
        ],
    ),
    (
        "a91f93fd-e428-4a36-b1b3-604b47a34287",
        "Tocasia's Dig Site",
        1,
        &[ManaColor::Green; 3],
    ),
    (
        "98e547de-b963-4ee4-9a08-67bae010734b",
        "University Campus",
        1,
        &[ManaColor::Green; 4],
    ),
];

/// The basic land that taps for one colour.
fn basic_of(color: ManaColor) -> CardIndex {
    let slot = match color {
        ManaColor::White => 0,
        ManaColor::Blue => 1,
        ManaColor::Black => 2,
        ManaColor::Red => 3,
        ManaColor::Green => 4,
        ManaColor::Colorless => unreachable!("no basic taps for colorless"),
    };
    baylee_cards::decks::basic_lands()[slot].expect("the pool has all five basics")
}

/// The ten lands that print "sacrifice it unless you return a land you
/// control to its owner's hand", and the basic each one will take.
///
/// A table rather than ten tests, because ten tests would be one test
/// retyped: what differs between a Karoo and a Rith's Grove is a subtype in
/// a filter, and the rule under all ten is one transcoding. What the table
/// buys is the population — a card added to this family by a later codegen
/// run and *not* added here is caught by
/// [`every_land_that_pays_by_returning_one_is_in_the_table`], which asks the
/// compiled pool rather than this list.
///
/// The Lairs take any land at all (their filter excludes other Lairs, which
/// a Plains is not); the five Karoos each demand their own untapped basic,
/// which is why the payment is named per row and never assumed.
const PAYS_BY_RETURNING_A_LAND: &[(&str, &str)] = &[
    // Karoo cycle: "an untapped <basic> you control".
    ("d4e875d9-2245-470d-aa2f-1dfe66ce2d15", "Plains"), // Karoo
    ("3f347ebf-e0d2-4ae0-ad84-df7a460404e0", "Island"), // Coral Atoll
    ("38ba1956-5505-4a7a-b6af-e75715b1401f", "Mountain"), // Dormant Volcano
    ("ef3b8b0c-cea7-4bae-934c-9c65fd64245d", "Swamp"),  // Everglades
    ("f922f90a-b1a2-4630-9266-40726ca89f74", "Forest"), // Jungle Basin
    // Lair cycle: "a land you control" (that is not another Lair).
    ("e9a7dede-3968-4b0e-a707-419d46a6fec9", "Plains"), // Crosis's Catacombs
    ("19b58ec9-bb88-4193-8ea8-c8f09ceec1ed", "Plains"), // Darigaaz's Caldera
    ("d8b57707-796d-4488-8f91-65bb75bc6281", "Plains"), // Dromar's Cavern
    ("e13289e5-370b-435b-a38e-cf57c3078cec", "Plains"), // Rith's Grove
    ("7b2c7758-2b89-49ff-8838-8dc9880c7209", "Plains"), // Treva's Ruins
];

/// The cards that pay the same way and are **not** lands.
///
/// The transcoder writes `PlayerMayPayCostOr { cost: ReturnToHand(…) }`
/// wherever the reference writes an `UnlessCost$` that returns a permanent,
/// and nothing in that rule is about lands — which is why the sweep below
/// asks the whole pool and not `cards/lands/`. Waterspout Djinn prints the
/// Karoo clause on an **upkeep** trigger, so the driver above, which plays a
/// land and waits for it to arrive, could never reach it.
///
/// A row here carries the same promise a row in the table above does — that
/// some test plays the card — and differs only in which test that is.
const PAYS_BY_RETURNING_A_LAND_ELSEWHERE: &[(&str, &str)] = &[(
    "050dac46-9ba0-4b8a-b61b-1c7ec6f3723a",
    // played by `creatures::a_djinn_that_costs_a_bounce_pays_it_or_is_sacrificed`
    "Waterspout Djinn",
)];

/// The basic the row names, as a handle.
fn basic(name: &str) -> CardIndex {
    match name {
        "Plains" => plains(),
        "Island" => island(),
        "Mountain" => mountain(),
        "Swamp" => swamp(),
        "Forest" => forest(),
        other => panic!("no handle for {other}"),
    }
}

/// Passes priority until the land's enters-trigger has resolved far enough
/// to put its question up, and hands back what it asked.
///
/// `None` is the other legitimate outcome and is recognised by the **land**
/// rather than by the shape of the next question: a trigger that found
/// nothing to ask sacrifices the land and hands priority straight back, so
/// what says "there was no question" is that the land has left the
/// battlefield. Reading it off the pending instead walked on to
/// `ChooseAttackers` and panicked there.
fn reach_the_unless_question(
    engine: &mut Engine<RegistryLookup>,
    land: ObjectId,
) -> Option<(Vec<ObjectId>, ChoicePrompt)> {
    for _ in 0..8 {
        if let Pending::ChooseCards {
            options, prompt, ..
        } = engine.pending().clone()
        {
            return Some((options, prompt));
        }
        if !engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&land)
        {
            return None;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "unexpected while waiting for the trigger: {:?}",
                engine.pending()
            )
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    None
}

/// The Battlebond "crowd" lands: "enters tapped unless you have two or more
/// opponents".
const UNTAPPED_WITH_A_CROWD: &[(&str, &str)] = &[
    (
        "761cb262-f83b-4a99-9345-b773182a7671",
        "Bountiful Promenade",
    ),
    ("819e1765-8325-4e6f-89c1-63ea86de369f", "Luxury Suite"),
    ("bd004c9d-771e-4e63-a97d-a2259c096af8", "Morphic Pool"),
    (
        "d1620449-930a-4895-a143-fd2a0a3c8b17",
        "Rejuvenating Springs",
    ),
    ("672e190d-8ea0-4a2e-b74f-5d35304631e4", "Sea of Clouds"),
    ("cf6d10ed-85c3-48f2-8ba0-2960e03b408b", "Spectator Seating"),
    ("45fe016e-1a09-410c-bbe3-4663ba06c5b7", "Spire Garden"),
    ("e3570ac7-c593-40e3-bbd6-ec3da6d8158d", "Training Center"),
    (
        "7c69f718-acc8-4851-8e5d-0cbaaa86192c",
        "Undergrowth Stadium",
    ),
    ("ebc5ac83-08d4-4d6b-b840-0c4ba71a38ab", "Vault of Champions"),
];

/// The Duskmourn "unlucky" lands: "enters tapped unless a player has 13 or
/// less life".
const UNTAPPED_WHEN_SOMEONE_IS_LOW: &[(&str, &str)] = &[
    (
        "0eec9984-cd11-4a52-9234-469c6a5fb9aa",
        "Abandoned Campground",
    ),
    ("47b6d2ae-d3d7-41eb-9172-2076eb8d028d", "Bleeding Woods"),
    ("6ccca5c2-66c3-495a-8d9e-1a9805569e52", "Etched Cornfield"),
    ("c56cd2ec-5907-4282-9162-d93b7dfd63b5", "Lakeside Shack"),
    ("c2cdefeb-3176-4faf-be54-a62d31f777a5", "Murky Sewer"),
    ("c8c632ab-14ec-44e1-ac00-81d48336320d", "Neglected Manor"),
    (
        "d55f7e20-11c6-44e2-8a21-dca67d3dbc68",
        "Peculiar Lighthouse",
    ),
    ("24a97436-ba61-4ebc-a560-a6c027ccfdf3", "Raucous Carnival"),
    ("8f69bd3a-244e-42d8-bfac-5a426f4b54b4", "Razortrap Gorge"),
    ("3a5b3405-a1e3-4aad-ab4e-1b8db2d1f3a8", "Strangled Cemetery"),
];

/// Whether the land `card` arrives tapped on the board `build` sets up.
fn arrives_tapped(build: impl FnOnce() -> crate::engine::testkit::Duel, card: CardIndex) -> bool {
    let p0 = PlayerId::new(0);
    let mut engine = build().hand(0, &[card]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = play_land(&mut engine, p0, card);
    engine
        .state()
        .object(land)
        .expect("the land arrived")
        .status
        .contains(Status::TAPPED)
}

/// The Ravnica bounce lands: "when this land enters, return a land you
/// control to its owner's hand".
///
/// The other Karoo sentence, and the difference is who is out of pocket.
/// The cycle above charges a bounce as a *price* and sacrifices the land
/// when it goes unpaid; these eleven simply do it, which is why the effect
/// is `ReturnChosenToHand` and not a `CostPart` — there is nothing to
/// decline.
const RETURNS_A_LAND_YOU_CONTROL: &[(&str, &str)] = &[
    ("189fc8f4-17ac-4f1d-82c8-8401445bdaf4", "Azorius Chancery"),
    ("8fa3ac81-3dfe-4565-be99-5554f7597b4b", "Boros Garrison"),
    ("378a1d57-e2f1-4b84-9692-1564602e9e99", "Dimir Aqueduct"),
    ("1b301478-b14f-4ef8-94e6-9647d582eabe", "Golgari Rot Farm"),
    ("657243dd-e479-4f4b-99d2-09b55d833a35", "Gruul Turf"),
    ("ee723c7c-ec9f-4ffb-8f36-cd7637eb1fae", "Guildless Commons"),
    ("1cb9d94a-3039-4f2e-8fcc-6996f9a45f74", "Izzet Boilerworks"),
    ("aa00ae0b-7c0f-427e-8102-ce0e2a6af5df", "Orzhov Basilica"),
    ("0a023964-2905-4928-9c3e-dc63e6ebd218", "Rakdos Carnarium"),
    ("00ef1c55-dea1-4564-bd57-66de86cba4df", "Selesnya Sanctuary"),
    (
        "046f5783-cc7b-416a-8cf6-2bcef9c2cc1a",
        "Simic Growth Chamber",
    ),
];

/// The four lands that charge *mana* for the same escape, and the CR 605.3a
/// window they need.
///
/// One board and two answers per card: the player is asked with an empty
/// pool, makes the mana inside the window, and keeps the land — then the
/// same card, declined, is sacrificed. Rupture Spire charges `{1}` and the
/// other three charge `{1}` as well, so one untapped Forest is the whole
/// price.
const PAYS_WITH_MANA: &[&str] = &[
    "a6543f71-0326-4e1f-b58f-9ce325d5d036", // Gateway Plaza
    "69c63055-ed44-4b32-b591-f3c6c2f3e7d1", // Archway Commons
    "7eadffcb-1e15-44c1-b1db-78c71b8ec1ce", // Rupture Spire
    "98334bfa-c516-4c20-bdc5-9e32e7127adc", // Transguild Promenade
];

/// The two tables above are the whole family, asked of the compiled pool.
///
/// [`PAYS_BY_RETURNING_A_LAND`] and [`PAYS_BY_RETURNING_A_LAND_ELSEWHERE`]
/// claim between them to be every card that escapes an effect by bouncing a
/// land, and a claim about a population is worth what a test says it is: a
/// fifteenth of these written by a later codegen run would otherwise pass
/// every gate in this file while never being played once. The
/// same argument as `lints::every_layer_in_the_pool_is_the_one_its_modifier_derives`
/// and the reason `no_card_claims_a_keyword_the_engine_ignores` is a test.
///
/// Set equality both ways, because the two failures are different repairs: a
/// card the pool has and the table does not is a card to play, and a row
/// naming a card the pool no longer has is a row to delete.
#[test]
fn every_land_that_pays_by_returning_one_is_in_the_table() {
    let mut pool: Vec<&str> = Vec::new();
    for (oracle_id, def) in baylee_cards::generated::ALL {
        let faces = def.faces.iter().map(|f| f.abilities);
        for list in core::iter::once(def.abilities).chain(faces) {
            let dump = format!("{list:?}");
            if dump.match_indices("PlayerMayPayCostOr { ").any(|(at, _)| {
                dump[at..]
                    .split_once("cost: ")
                    .is_some_and(|(_, tail)| tail.starts_with("ReturnToHand("))
            }) {
                pool.push(oracle_id);
            }
        }
    }
    pool.sort_unstable();
    pool.dedup();
    let mut table: Vec<&str> = PAYS_BY_RETURNING_A_LAND
        .iter()
        .chain(PAYS_BY_RETURNING_A_LAND_ELSEWHERE)
        .map(|(id, _)| *id)
        .collect();
    table.sort_unstable();

    assert!(
        pool.len() >= 11,
        "only {} cards in the pool escape a sacrifice by bouncing a land, \
         against the eleven that carried it when this was written — the \
         probe broke",
        pool.len()
    );
    assert_eq!(
        pool, table,
        "the pool and PAYS_BY_RETURNING_A_LAND disagree. A card the table \
         is missing is a card nothing plays; a row the pool is missing names \
         a card that left"
    );
}

/// The eleven lands whose *activation* cost asks the player to name an
/// object, with the one thing each needs on the board to pay it.
///
/// A second family from the same transcoder rule as
/// [`PAYS_BY_RETURNING_A_LAND`] and a different sentence: this is not an
/// escape from an effect but a price on an ability, `{T}, Sacrifice a
/// creature:` and its neighbours. What they share is the part the player
/// answers by naming something, which is what this table plays.
///
/// Two cards feed all of them and are not chosen for convenience. Baleful
/// Strix is an *artifact creature — Bird*, so one card is a legal answer to
/// "a creature", "an artifact" and "a Bird" alike; Gateway Plaza is a
/// *Gate*, which is a land. Ipnu Rivulet needs neither, because it
/// sacrifices a Desert and is one — the row a filter written as "another"
/// would have got wrong.
const PAYS_BY_NAMING_AN_OBJECT: &[(&str, Feed)] = &[
    ("86fb3749-37d6-48a6-8524-71e996850307", Feed::Strix), // High Market
    ("5effaa94-7f87-4485-8959-473d584c5034", Feed::Strix), // Grim Backwoods
    ("ea4d6fcd-21e0-4e9f-b406-a89042998d98", Feed::Strix), // Keldon Necropolis
    ("b6cc062c-eb39-46ee-bd6d-17f1db0ac50d", Feed::Strix), // Phyrexia's Core
    ("4adc39dd-8de1-4298-947c-ff666ec3adeb", Feed::Strix), // Seaside Haven
    ("9abf9a0e-8e7d-406b-a01d-d4870b30134e", Feed::Strix), // The Shire
    ("d3df7128-31dd-4d71-90be-87e2e9ff51b4", Feed::Plaza), // Dust Bowl
    ("e10e84a7-d564-487a-ac64-5a001a45ee90", Feed::Plaza), // Rath's Edge
    ("35922a30-6b84-44dd-a2f0-306554a1ae90", Feed::Plaza), // Heap Gate
    ("c17d799f-adc9-4c41-87cf-b243b5ea3be1", Feed::Itself), // Ipnu Rivulet
    ("850bb6f7-48d3-4d65-9220-b0bec5ee6b64", Feed::HandCard), // Fogwell's Gym
];

/// What a row seats so its land can pay.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Feed {
    /// Baleful Strix: a creature, an artifact and a Bird in one card.
    Strix,
    /// Gateway Plaza: a land, and a Gate.
    Plaza,
    /// A card in hand, for the one that discards.
    HandCard,
    /// The land is its own feed.
    Itself,
}

fn gateway_plaza() -> CardIndex {
    card_index("a6543f71-0326-4e1f-b58f-9ce325d5d036")
}

/// The `Cost` an activated ability charges, both spellings.
///
/// `ActivatedConditional` is the twin six readers across this workspace have
/// already been found matching only half of.
fn activation_cost(ability: &'static AbilityDef) -> Option<&'static baylee_cards_dsl::Cost> {
    match ability {
        AbilityDef::Activated { cost, .. } | AbilityDef::ActivatedConditional { cost, .. } => {
            Some(cost)
        }
        _ => None,
    }
}

/// The ability of `card` whose cost asks the player to name an object.
///
/// Found by reading the compiled `CostPart`s rather than by writing an index
/// into the table: the index is a card file's ability order, which is
/// codegen's to change, and a test pinned to it would start exercising the
/// mana ability the day a card grew a second one.
fn ability_that_asks(engine: &Engine<RegistryLookup>, card: CardIndex) -> Option<(ObjectId, u32)> {
    use baylee_cards_dsl::CostPart;
    let Pending::Priority { legal, .. } = engine.pending() else {
        return None;
    };
    legal.abilities.iter().copied().find(|(id, index)| {
        engine
            .state()
            .object(*id)
            .and_then(|o| o.card)
            .filter(|c| c.index == card)
            .and_then(|c| baylee_cards::by_index(c.index))
            .and_then(|def| def.abilities.get(*index as usize))
            .and_then(activation_cost)
            .is_some_and(|cost| {
                cost.parts.iter().any(|part| {
                    matches!(
                        part,
                        CostPart::Sacrifice(_)
                            | CostPart::Discard(_)
                            | CostPart::TapOther(_)
                            | CostPart::ReturnToHand(_)
                            | CostPart::ExileFromGraveyard(_)
                    )
                })
            })
    })
}

/// No basic land is a legal answer to a price that is not a land.
///
/// An independent reading of the menu, and the reason it is here: reading
/// only `options[0]` let a `cost_wizard::options` with its filter bypassed
/// pass the test above, because the first offer happened to be the right
/// one anyway. Ten basics are on that board precisely so that a menu which
/// ignored its filter would be visibly wrong.
fn no_basic_land_on_the_menu(engine: &Engine<RegistryLookup>, options: &[ObjectId], oracle: &str) {
    for &offered in options {
        let is_basic = engine
            .state()
            .object(offered)
            .and_then(|o| o.card)
            .and_then(|c| baylee_cards::by_index(c.index))
            .is_some_and(|def| {
                def.faces.first().is_some_and(|face| {
                    face.supertypes
                        .contains(baylee_core::types::SupertypeSet::BASIC)
                })
            });
        assert!(
            !is_basic,
            "{oracle} offers a basic land for a price that is not a land"
        );
    }
}

/// What paying looks like, read off the part that was paid.
///
/// A discard and a sacrifice both reach a graveyard and a tap reaches
/// nothing at all, so the outcome is asked of the `CostPart` rather than
/// assumed — and it is asked of the **object**, because a sacrifice that
/// drew its card while leaving the creature on the battlefield is exactly
/// what a test on the ability's effect would let through.
fn the_object_paid(
    engine: &Engine<RegistryLookup>,
    card: CardIndex,
    index: u32,
    paid: ObjectId,
    seat: PlayerId,
    oracle: &str,
) {
    use baylee_cards_dsl::CostPart;
    let part = baylee_cards::by_index(card)
        .and_then(|def| def.abilities.get(index as usize))
        .and_then(activation_cost)
        .and_then(|cost| {
            cost.parts.iter().find(|part| {
                matches!(
                    part,
                    CostPart::Sacrifice(_) | CostPart::Discard(_) | CostPart::TapOther(_)
                )
            })
        })
        .expect("the part this row is about");
    match part {
        CostPart::TapOther(_) => assert!(
            engine
                .state()
                .object(paid)
                .is_some_and(|o| o.status.contains(Status::TAPPED)),
            "{oracle}: what paid is tapped"
        ),
        _ => assert!(
            !engine
                .state()
                .zones
                .list(ZoneLocation::Battlefield)
                .contains(&paid)
                && !engine
                    .state()
                    .zones
                    .list(ZoneLocation::Hand(seat))
                    .contains(&paid),
            "{oracle}: what paid has left the zone it paid from"
        ),
    }
}

/// The eleven lands the transcoder writes out of a `ChangeZone` that reads a
/// library, with the basic each one's own filter accepts and one it refuses.
///
/// The second column fills the library, so there is something to find at
/// all. The third is what
/// [`a_land_that_searches_finds_nothing_outside_its_own_filter`] fills it
/// with instead, and is empty for the six that take any basic land there is
/// — those six have no outside.
const SEARCHES_THE_LIBRARY: &[(&str, &str, &str)] = &[
    // "a basic land card".
    ("861eb7d7-7616-4620-a4fd-4b8c3bf00dd1", "Forest", ""), // Promising Vein
    ("032b8a0d-491a-4a12-ab9f-689010054d5b", "Forest", ""), // Prismatic Vista
    ("619173f4-0403-49cd-9659-2fedd5028a90", "Forest", ""), // Shire Terrace
    ("58eaaa8b-45c6-439b-bdd1-5f4e77a75a8c", "Forest", ""), // Terminal Moraine
    ("6a7f3e1f-6798-4644-b64c-7765f81f0938", "Forest", ""), // Vibrant Cityscape
    ("543e6bb3-a867-43bf-a737-2f5d6d8dc631", "Forest", ""), // Warped Landscape
    // The Panorama cycle: three named basics each, and two it must refuse.
    ("0a1d817d-dce8-4e83-a380-909f7c9eee46", "Forest", "Swamp"), // Bant
    ("6b9cd3d0-4316-4945-b960-12f51052d260", "Plains", "Forest"), // Esper
    ("743f4488-fef1-4f4d-b745-d2de92423e00", "Island", "Plains"), // Grixis
    ("f39f33ac-074d-442d-ae4c-1d694ee315f3", "Swamp", "Plains"), // Jund
    ("71e28800-c42c-48c0-95e5-0296be54a4e8", "Mountain", "Island"), // Naya
    // The two that fetch a basic and sacrifice themselves for nothing else.
    ("a75445d3-1303-4bb5-89ad-26ea93fecd48", "Forest", ""), // Evolving Wilds
    ("1bd3e453-aa21-4ee6-95c2-d6d920ee8e7a", "Forest", ""), // Terramorphic Expanse
    // The fetchlands. Their filter is a *land type* and not `Basic`, so a
    // basic of a named type is found and a basic of any other is refused —
    // which is the same pair of questions the rows above ask, put to a
    // filter written the other way round.
    ("fc0707c7-d504-4ccf-a0d2-3eb6e26e7a57", "Swamp", "Plains"), // Bloodstained Mire
    ("f3c7af78-a77d-4134-82a2-a5ce84285a84", "Plains", "Swamp"), // Flooded Strand
    ("dab520d0-20b4-4273-ba6b-eb07f85ea433", "Plains", "Forest"), // Marsh Flats
    ("09dd85aa-47bc-4713-a9b9-8b52ff2285ed", "Forest", "Plains"), // Misty Rainforest
    ("cb027150-848c-4a66-88ad-e20222304dd8", "Island", "Forest"), // Scalding Tarn
];

/// The ability of `card` that reads a library, and where what it finds goes.
///
/// `finds` is read off the compiled card rather than written into the table
/// beside it, for the same reason [`ability_that_asks`] reads the cost: the
/// table would then be a second opinion about a card, and the card is the
/// one this rule wrote. Both activated spellings, because
/// `ActivatedConditional` is the twin readers keep missing.
fn ability_that_searches(
    engine: &Engine<RegistryLookup>,
    card: CardIndex,
) -> Option<(ObjectId, u32, &'static [Find])> {
    let Pending::Priority { legal, .. } = engine.pending() else {
        return None;
    };
    legal.abilities.iter().copied().find_map(|(id, index)| {
        let def = engine
            .state()
            .object(id)
            .and_then(|o| o.card)
            .filter(|c| c.index == card)
            .and_then(|c| baylee_cards::by_index(c.index))?;
        let effects = match def.abilities.get(index as usize)? {
            AbilityDef::Activated { effects, .. }
            | AbilityDef::ActivatedConditional { effects, .. } => *effects,
            _ => return None,
        };
        effects.iter().find_map(|effect| match effect {
            Effect::SearchLibrary { finds, .. } => Some((id, index, *finds)),
            _ => None,
        })
    })
}

/// Passes priority until the search puts its question up.
///
/// `None` is the other legitimate outcome: a search that matches nothing in
/// the library shuffles and asks nobody (CR 701.23b), so what says "there
/// was no question" is the ability having left the stack. The land is no
/// signal here — it was sacrificed to *pay* for this, one step before the
/// ability ever went on the stack.
fn reach_the_search(engine: &mut Engine<RegistryLookup>) -> Option<(Vec<ObjectId>, u8, u8)> {
    for _ in 0..8 {
        if let Pending::ChooseCards {
            options,
            min,
            max,
            prompt: ChoicePrompt::SearchLibrary,
            ..
        } = engine.pending().clone()
        {
            return Some((options, min, max));
        }
        if engine.state().zones.list(ZoneLocation::Stack).is_empty() {
            return None;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "unexpected while waiting for the search: {:?}",
                engine.pending()
            )
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    // Never a quiet `None`: one ability on an otherwise empty stack resolves
    // in two passes, so eight of them mean the walk lost its way — and the
    // negative test below reads `None` as "the search asked nothing", which
    // an exhausted loop would satisfy without ever reaching the search.
    panic!("the ability never resolved: {:?}", engine.pending())
}

/// Plays the row's land, pays for it, and hands back the engine standing on
/// whatever the search asked — with the land already gone, because every
/// one of these eleven sacrifices itself to pay.
fn a_land_that_searches(
    seed: u64,
    card: CardIndex,
    library: CardIndex,
) -> (Engine<RegistryLookup>, &'static [Find]) {
    use baylee_cards_dsl::CostPart;
    let p0 = PlayerId::new(0);
    // Two of every basic: enough for the dearest of the eleven, `{2}`, and
    // colours the filters have opinions about.
    let mut engine = Duel::new(seed, library)
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                island(),
                island(),
                swamp(),
                swamp(),
                mountain(),
                mountain(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, card);
    // Round the turn: three of these come down tapped, and every one of them
    // spends its own `{T}` as part of the price.
    cross_into_the_next_own_main(&mut engine, p0);
    tap_all_mana_but(&mut engine, p0, Some(card));

    let (source, index, finds) = ability_that_searches(&engine, card)
        .expect("the land offers the ability that reads a library");
    assert_eq!(source, land, "the ability is on the land just played");
    // Read off the compiled cost for the same reason `finds` is read off the
    // compiled effect: five of these rows are fetchlands, whose price is
    // half the printed sentence, and a table that wrote the number beside
    // the card would be a second opinion about it. A row that prints no
    // life asserts that none is paid, which is the arm that would otherwise
    // never fire.
    let owed: u16 = baylee_cards::by_index(card)
        .and_then(|def| def.abilities.get(index as usize))
        .map(|def| match def {
            AbilityDef::Activated { cost, .. } | AbilityDef::ActivatedConditional { cost, .. } => {
                cost.parts
                    .iter()
                    .map(|part| match part {
                        CostPart::PayLife(n) => *n,
                        _ => 0,
                    })
                    .sum()
            }
            _ => 0,
        })
        .expect("the ability that searches is one of the two activated spellings");
    let life_before = engine.state().players[0].life;
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index: index,
            },
        )
        .unwrap();
    assert_eq!(
        life_before - engine.state().players[0].life,
        i32::from(owed),
        "a cost is paid on activation, and life is the half of a fetchland \
         that the search itself cannot show"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&land),
        "the land pays for this with itself, and a cost is paid on activation"
    );
    (engine, finds)
}

/// Every row: the land is played, sacrificed for its own ability, and what
/// the search found is on the battlefield in the state the card prints.
///
/// The count and the tapping are read off the compiled `finds` rather than
/// asserted as constants, so the day a `Find` in one of these cards changes
/// the test follows the card instead of arguing with it. What is fixed here
/// is the sentence around them: the search offers exactly as many as it
/// finds, no fewer (none of the eleven prints "up to"), and every card named
/// arrives.
#[test]
fn a_land_that_searches_puts_what_it_found_onto_the_battlefield() {
    for (i, (oracle, fills, _)) in SEARCHES_THE_LIBRARY.iter().enumerate() {
        let card = card_index(oracle);
        let p0 = PlayerId::new(0);
        let seed = 1020 + u64::try_from(i).expect("eleven rows");
        let (mut engine, finds) = a_land_that_searches(seed, card, basic(fills));

        let (options, min, max) =
            reach_the_search(&mut engine).unwrap_or_else(|| panic!("{oracle} asked nothing"));
        let want = u8::try_from(finds.len()).expect("a handful at most");
        assert_eq!(
            (min, max),
            (want, want),
            "{oracle} prints no \"up to\", so the search is for all {want} of them"
        );
        let chosen: Vec<ObjectId> = options.iter().copied().take(finds.len()).collect();
        assert_eq!(
            chosen.len(),
            finds.len(),
            "{oracle} was offered {} cards out of a library of sixty {fills}",
            options.len()
        );
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: chosen.clone(),
                },
            )
            .unwrap();

        let battlefield = engine.state().zones.list(ZoneLocation::Battlefield).clone();
        for (found, find) in chosen.iter().zip(finds) {
            assert_eq!(
                find.dest,
                SearchDest::Battlefield,
                "{oracle} is one of the eleven that fetch onto the battlefield"
            );
            assert!(
                battlefield.contains(found),
                "{oracle} named a card and it never arrived"
            );
            assert_eq!(
                engine
                    .state()
                    .object(*found)
                    .and_then(|o| o.card)
                    .map(|c| c.index),
                Some(basic(fills)),
                "{oracle} put something other than the {fills} it was handed onto the battlefield"
            );
            assert_eq!(
                entered_tapped(&engine, *found),
                find.tapped,
                "{oracle} prints tapped = {}, and the card arrived the other way",
                find.tapped
            );
        }
    }
}

/// The five Panoramas, over a library of the one basic each of them refuses.
///
/// The independent reading of the filter, and the reason it is a second
/// test: the test above fills the library with sixty cards the filter
/// accepts, so a `SearchLibrary` that ignored its filter entirely would
/// pass it every time. Here nothing matches, the search asks no question at
/// all (CR 701.23b), and the land is still gone — a filter that let the
/// wrong basic through would put a question up instead.
#[test]
fn a_land_that_searches_finds_nothing_outside_its_own_filter() {
    let mut checked = 0;
    for (i, (oracle, _, refuses)) in SEARCHES_THE_LIBRARY.iter().enumerate() {
        if refuses.is_empty() {
            continue;
        }
        checked += 1;
        let card = card_index(oracle);
        let seed = 1040 + u64::try_from(i).expect("eleven rows");
        let (mut engine, _) = a_land_that_searches(seed, card, basic(refuses));
        assert!(
            reach_the_search(&mut engine).is_none(),
            "{oracle} offered a search over a library of sixty {refuses}, which it does not name"
        );
    }
    assert_eq!(
        checked, 10,
        "ten rows name a basic they must refuse — the five Panoramas and the \
         five fetchlands — and this test speaks for all of them. A row whose \
         `refuses` is empty is a land that takes any basic and has nothing to \
         be refused, so it is skipped rather than counted"
    );
}

/// The ten fast lands, on the two boards their sentence divides.
///
/// Every other enters-tapped cycle in this file is a lower bound, and these
/// are the upper one — so the board that turns a slow land on is the board
/// that turns these off, and a test written on a duel's empty table would
/// show only the untapped branch of all of them.
const UNTAPPED_ON_A_SMALL_BOARD: &[(&str, &str)] = &[
    ("5ad94412-6f79-4c5d-bbd4-4ef5779a7b6d", "Blackcleave Cliffs"),
    ("66fa2326-1b5d-41fb-b919-83bf9f383577", "Blooming Marsh"),
    ("88f8f683-738e-48f3-afff-c8f73f1033a2", "Botanical Sanctum"),
    (
        "2d899466-b1eb-4901-b626-1f2fb09b786d",
        "Concealed Courtyard",
    ),
    ("a05f641c-15c9-43dc-ae0d-1ea372fd33d5", "Copperline Gorge"),
    ("a2b48695-f7d7-42ce-a8a0-2a723428542a", "Darkslick Shores"),
    ("3f17c60e-923a-4392-9da8-87d9ded009b7", "Inspiring Vantage"),
    ("94f6c407-e665-4032-be13-a01e40c1f306", "Razorverge Thicket"),
    ("9e7a240d-dc33-47ac-9f17-77fab4c1c340", "Seachrome Coast"),
    ("eb0d8093-5f93-4b25-9384-08f9731bfb28", "Spirebluff Canal"),
];

/// Whether `land` is offered the any-colour ability The World Tree grants.
fn offered_any_colour(engine: &Engine<RegistryLookup>, land: ObjectId) -> bool {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    legal
        .abilities
        .contains(&(land, crate::choice::GRANTED_ABILITY))
}

fn lair_of_the_hydra() -> CardIndex {
    card_index("126e9140-2c05-4c00-8b01-5653456c736a")
}

fn mystic_sanctuary() -> CardIndex {
    card_index("17b60106-a4c7-410a-8ac3-ec8e74e29a7c")
}

fn witch_s_cottage() -> CardIndex {
    card_index("6c8f276e-4e7b-4974-ab02-9356cc0ffb2b")
}

/// The thirteen modal double-faced cards round J added, named rather than
/// counted.
///
/// The sweep below is the test; this list is what keeps it from going
/// quiet. A walk that stopped finding cards would pass every assertion it
/// makes, and a floor sitting between two growing numbers is the fault the
/// land-arrival sweep was repaired for. These are the thirteen a person
/// read against their printing, so the sweep is asked to have reached each
/// of them by name -- which also makes a re-filed or renamed face a
/// failure here rather than a silent shrinking of the population.
/// The spell-fronted land backs whose arrival asks a question, and so the
/// exact set this sweep does not play. Filled from the sweep's own first
/// run rather than from a grep, for the reason the enters-tapped floor is:
/// a list bounded by one instrument and read by another bounds nothing.
const ARRIVALS_THAT_ASK: &[&str] = &[
    "Agadeem, the Undercrypt",
    "Boggart Bog",
    "Emeria, Shattered Skyclave",
    "Fell Mire",
    "Garden of Freyalise",
    "Hydroelectric Laboratory",
    "Mystic Peak",
    "Razorgrass Field",
    "Sea Gate, Reborn",
    "Shatterskull, the Hammer Pass",
    "Soporific Springs",
    "Tanglespan Bridgeworks",
    "Turntimber, Serpentine Wood",
    "Volcanic Fissure",
    "Witch-Blessed Meadow",
];

/// What the sweep reached, less a margin for a card leaving the pool: **35**
/// played on 22.09.2026 over a pool of 2716, beside the fifteen in
/// [`ARRIVALS_THAT_ASK`] that it does not play. Read off the sweep's own run
/// rather than off a grep of the card files, for the reason `validate`'s
/// enters-tapped floor is: a count taken by one instrument bounds nothing
/// about another.
///
/// It reached 55 this morning, and the twenty that left are why this sweep
/// needed a neighbour rather than a tighter assertion. A **transforming**
/// double-faced card is cast as its front face and reaches its back only by
/// turning over (CR 712.8c), so a land on the back of one is not a land drop
/// at all — and `FaceDef::castable_from_hand` defaults to `true`, so
/// twenty-one cards in the pool offered theirs out of hand. Twenty of those
/// are spell-fronted and were counted here; the twenty-first, Havengul
/// Laboratory, has a land front and this sweep never looked at it.
///
/// It could not see any of them and never could: it plays every
/// spell-fronted land back and asserts that what arrives is that face's own
/// types, which was perfectly true of all twenty. What tells a transforming
/// card from a modal one is Scryfall's `layout`, which is a printing and not
/// a rule a card file states, so the guard is in `xtask validate` and this
/// number is what fell out of it.
const FLOOR: usize = 32;

const ROUND_J_MODAL_LANDS: &[fn() -> CardIndex] = &[
    revitalizing_repast,
    vastwood_fortification,
    beyeen_veil,
    jwari_disruption,
    kabira_takedown,
    legion_leadership,
    waterlogged_teachings,
    bala_ged_recovery,
    makindi_stampede,
    song_mad_treachery,
    zof_consumption,
    ondu_inversion,
];

/// Every card whose front face is a spell and whose back is a land it may
/// be played as, played as that land.
///
/// One printed sentence asked of a whole family rather than of one card. A
/// modal double-faced card is a land drop *or* a spell and never both, so
/// what has to hold is that the back face arrives as a **land**, that the
/// front face's card type stayed on the other side of the card, and that it
/// entered tapped exactly when its own face says so. The last of those is
/// the assertion with teeth: the enters-tapped modifier lives on the *back*
/// face, and a reader taking the card's own top-level list would find
/// nothing there and report a clean arrival.
///
/// Asked of the compiled pool rather than of a hand-written list, for the
/// reason `the_only_lands_that_would_count_themselves_are_the_slow_ones`
/// gives -- the pool is what ships, and thirteen of these arrived in one
/// batch from one model, so the question worth asking is whether the
/// *family* holds and not whether those thirteen do.
///
/// It lives in this door although every one of these cards is filed under
/// `instants/` or `sorceries/`: what the test plays is a land drop and what
/// it asserts is how a land arrives.
#[test]
fn every_modal_back_face_land_is_played_as_the_land_it_prints() {
    let mut reached = Vec::new();
    let mut asks = Vec::new();
    for def in baylee_cards::all() {
        let [front, back] = def.faces else { continue };
        if front.types.contains(TypeSet::LAND) || !back.types.contains(TypeSet::LAND) {
            continue;
        }
        // A transforming card's back is not something a player may put onto
        // the battlefield from hand, and this is the one field separating
        // the two layouts: without it a werewolf's night side would be
        // asked for a land drop it never offers.
        if !back.castable_from_hand {
            continue;
        }
        // "As this land enters, you may pay 3 life. If you don't, it enters
        // tapped" is a question asked *before* the land is on the
        // battlefield, and `play_land_face` answers the face choice and
        // nothing else -- so the card is still in hand when it returns and
        // a perfectly good land drop reads as a refusal. Those are named
        // below rather than dropped: what a sweep did not read is the half
        // of its answer that nothing else can recover.
        if !back
            .enter_modifiers
            .iter()
            .all(|m| *m == baylee_cards_dsl::EnterModifier::Tapped)
        {
            asks.push(back.name);
            continue;
        }
        let (engine, land) = play_land_face(def.index, 1)
            .unwrap_or_else(|err| panic!("{}: the land face {err}", back.name));

        let played = types(&engine, land);
        // The whole type line and not "is it a land": a back face that
        // shares a type with its front -- The Myriad Pools is an artifact
        // land behind an artifact -- makes "the front's type is absent" a
        // sentence that is false about a correct card, and equality is the
        // claim that was meant anyway.
        assert_eq!(
            played, back.types,
            "{}: what arrived is not the types its own face prints",
            back.name
        );
        let says_tapped = back
            .enter_modifiers
            .contains(&baylee_cards_dsl::EnterModifier::Tapped);
        assert_eq!(
            is_tapped(&engine, land),
            says_tapped,
            "{}: its own face says enters-tapped = {says_tapped}",
            back.name
        );
        reached.push(def.index);
    }

    asks.sort_unstable();
    assert_eq!(
        asks, ARRIVALS_THAT_ASK,
        "the set of spell-fronted land backs whose arrival asks a question \
         has changed; each of them is a land drop this harness cannot make, \
         so a new one needs a test of its own and a departure needs this \
         list shortened"
    );
    assert!(
        reached.len() >= FLOOR,
        "only {} spell-fronted land backs were played against a floor of \
         {FLOOR}; the walk is not reaching the pool",
        reached.len()
    );
    let missing: Vec<CardIndex> = ROUND_J_MODAL_LANDS
        .iter()
        .map(|handle| handle())
        .filter(|index| !reached.contains(index))
        .collect();
    assert!(
        missing.is_empty(),
        "the sweep never reached {} of the cards it was written for: {missing:?}",
        missing.len()
    );
}

fn inkmoth_nexus() -> CardIndex {
    card_index("675281ff-b81f-4e5e-9f85-9f8cd202b50b")
}

fn lavaclaw_reaches() -> CardIndex {
    card_index("340de307-982a-4b26-9dc0-99113dc766cd")
}

fn mishra_s_foundry() -> CardIndex {
    card_index("b43e9772-6ad4-49c7-9557-b18ee1e4587d")
}

fn mobilized_district() -> CardIndex {
    card_index("eb2094cf-b4be-4f52-8615-e179ef7c741d")
}

fn muraganda_raceway() -> CardIndex {
    card_index("b5fa5651-d714-44d6-867b-be0e3224b7ed")
}

fn nantuko_monastery() -> CardIndex {
    card_index("c6de0ee9-785d-4cd8-8a7f-5bb715763131")
}

fn raging_ravine() -> CardIndex {
    card_index("8d38194e-b607-4ff4-9c19-0e8636d463bf")
}

fn restless_anchorage() -> CardIndex {
    card_index("91320daf-f69c-4350-b0fc-4bb37a6904b1")
}

fn svogthos_the_restless_tomb() -> CardIndex {
    card_index("a34a70b8-02e5-4e8c-a9e7-b21c5a11dddf")
}

fn wandering_fumarole() -> CardIndex {
    card_index("741c51f1-cfbe-4c29-ac8f-ca6bcd2652f9")
}

fn arch_of_orazca() -> CardIndex {
    card_index("3bb518ff-399b-4ce7-b9ad-a1d563dd7792")
}

fn barbarian_ring() -> CardIndex {
    card_index("eeb9377b-72c1-4214-9a66-0f55577c17d1")
}

fn cabal_pit() -> CardIndex {
    card_index("92392467-a22f-4dd7-a0eb-393bef956dc0")
}

fn centaur_garden() -> CardIndex {
    card_index("5cf92fd4-7c0b-4d8e-92f1-53dc2e0476fc")
}

fn evendo_waking_haven() -> CardIndex {
    card_index("83161d59-2520-4741-9328-e2a4a8b5d5bc")
}

fn fortified_beachhead() -> CardIndex {
    card_index("387fe395-e4a0-4fb2-8d6c-88a1a21d2ed8")
}

fn isolated_watchtower() -> CardIndex {
    card_index("3893f320-47fd-49ec-a78c-80bfb607a279")
}

fn lilypad_village() -> CardIndex {
    card_index("5bb06e6f-e3af-4caa-b66d-77248ad46b61")
}

fn planar_nexus() -> CardIndex {
    card_index("26005003-afcb-4c32-a760-be950246ff0f")
}

fn tomb_of_urami() -> CardIndex {
    card_index("f002be6a-e459-49c4-b765-062e30107439")
}

fn uthros_titanic_godcore() -> CardIndex {
    card_index("df08ac72-010f-42f8-beb3-6d645c638e1e")
}

/// Stations Evendo by tapping `creature`, answering the cost's question as
/// a player would.
#[track_caller]
fn station_evendo(engine: &mut Engine<RegistryLookup>, creature: ObjectId) {
    let p0 = PlayerId::new(0);
    activate(engine, p0, evendo_waking_haven(), 1);
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!(
            "station asks which creature pays, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(prompt, ChoicePrompt::CostTap, "a cost, not a target");
    assert!(options.contains(&creature), "another creature you control");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![creature],
            },
        )
        .expect("the creature taps");
}

fn abstergo_entertainment() -> CardIndex {
    card_index("d06a8026-1657-4404-8dff-64e44f1a14f8")
}

fn base_camp() -> CardIndex {
    card_index("41fbf835-baee-4530-9155-e2c1b9045567")
}

fn crucible_of_the_spirit_dragon() -> CardIndex {
    card_index("ecfbebc9-7fc7-474e-8c59-8ede800e082e")
}

fn ishgard_the_holy_see() -> CardIndex {
    card_index("4f4358cb-59df-46d9-be27-69929f5a615c")
}

fn secluded_courtyard() -> CardIndex {
    card_index("79ba18fd-f184-43c1-86df-56ee18ce806c")
}

fn study_hall() -> CardIndex {
    card_index("eb735501-19e7-4910-aa6a-6667fff6f4e5")
}

fn teferi_s_isle() -> CardIndex {
    card_index("ce55657d-d82f-4528-a83e-5cad7de111fd")
}

fn temple_of_the_dragon_queen() -> CardIndex {
    card_index("169a26d2-7bc9-4403-9c92-98d4bd5ca4f3")
}

fn the_monumental_facade() -> CardIndex {
    card_index("63e8d282-d038-4a8c-a0cb-f51ddf87d8ea")
}

fn the_seedcore() -> CardIndex {
    card_index("249fdd3e-376c-4ec2-a612-4353e0e61ee2")
}

fn zanarkand_ancient_metropolis() -> CardIndex {
    card_index("5f2b3ea8-99ee-47a4-8a1c-4b27478d524c")
}

/// Whether `card` carries a link to an exile.
fn linked(engine: &Engine<RegistryLookup>, card: ObjectId) -> bool {
    engine.state().object(card).is_some_and(|o| {
        o.riders
            .iter()
            .any(|r| matches!(r, crate::object::Rider::Linked { .. }))
    })
}

fn accursed_duneyard() -> CardIndex {
    card_index("48edc348-93f6-4dce-9cc4-7244d76b6f4a")
}

fn amonkhet_raceway() -> CardIndex {
    card_index("fe174586-d36b-40f6-babd-1f98e76eec22")
}

fn argoth_sanctum_of_nature() -> CardIndex {
    card_index("62648946-1708-48f4-ba40-c057563ab11b")
}

fn avishkar_raceway() -> CardIndex {
    card_index("e2c35551-1ba5-4424-baf9-821b49bbcc8c")
}

fn bretagard_stronghold() -> CardIndex {
    card_index("c792229b-4a0f-48d5-93e5-60bd4cae9c42")
}

fn castle_doom() -> CardIndex {
    card_index("9bd013df-ad75-4099-940b-1765c58faf26")
}

fn detection_tower() -> CardIndex {
    card_index("93695c16-c441-492d-af12-b57df9739846")
}

fn edgewall_inn() -> CardIndex {
    card_index("4dfd33e8-7e30-493b-8564-f7df5f0257aa")
}

fn eiganjo_seat_of_the_empire() -> CardIndex {
    card_index("7edb3d15-4f70-4ebe-8c5e-caf6a225076d")
}

fn emeria_the_sky_ruin() -> CardIndex {
    card_index("cc999cf2-c99b-4911-8c52-6cc4a99fcc7b")
}

fn eumidian_hatchery() -> CardIndex {
    card_index("5b6d933e-2830-4f5a-b244-a421aa9615dc")
}

fn field_of_ruin() -> CardIndex {
    card_index("f825c98f-a327-440b-8c0d-ebe02e23bfb7")
}

fn field_of_the_dead() -> CardIndex {
    card_index("aa959340-c869-4caa-92c7-572bd8d23eef")
}

fn fomori_vault() -> CardIndex {
    card_index("622a53bd-d894-422c-a606-7126041afa02")
}

fn great_hall_of_starnheim() -> CardIndex {
    card_index("75d8ef26-b745-448b-94b9-3c64668ce171")
}

fn hall_of_oracles() -> CardIndex {
    card_index("0bb896ba-e15e-43a9-9120-e674d7ba003c")
}

fn hall_of_tagsin() -> CardIndex {
    card_index("fd809587-773c-47ec-8676-cefef6d1e38f")
}

fn hanweir_battlements() -> CardIndex {
    card_index("0e735ba6-7fd1-4d12-b20c-21525dc1e2b5")
}

fn haven_of_the_spirit_dragon() -> CardIndex {
    card_index("acc9c16a-5e72-43bd-87e1-56a16aa892f5")
}

fn heart_of_yavimaya() -> CardIndex {
    card_index("6c9a854c-0509-4ed4-9d94-c45b823b65e5")
}

fn hidden_hideout() -> CardIndex {
    card_index("b639e1fe-d099-4cab-a0d0-a1b33c7f31dd")
}

fn ice_floe() -> CardIndex {
    card_index("cfaaead2-09e8-47cb-9e39-8570b8d8de86")
}

fn junktown() -> CardIndex {
    card_index("c470a802-931f-4b63-92fd-ef9cf2e796dd")
}

fn kjeldoran_outpost() -> CardIndex {
    card_index("8b370db5-dfb9-4ea0-9017-bae3e767b041")
}

fn lava_tubes() -> CardIndex {
    card_index("2ffb2647-60bb-4916-b919-cbcf18e5e424")
}

fn littjara_mirrorlake() -> CardIndex {
    card_index("3b577179-10d0-43f1-ac17-9dd2d12c965d")
}

fn magosi_the_waterveil() -> CardIndex {
    card_index("4bdffa67-e6b3-4588-b76e-c11db6f043ca")
}

fn maze_s_end() -> CardIndex {
    card_index("49479778-c4c0-43ba-a7b7-45f00d067462")
}

fn mount_doom() -> CardIndex {
    card_index("995c8dac-fd27-468a-abd4-02372cf0c850")
}

fn mudflat_village() -> CardIndex {
    card_index("aeeab1df-0b8b-4bc4-a5f9-aac413449bec")
}

fn myriad_landscape() -> CardIndex {
    card_index("2549bc57-9ffb-4053-9f10-f2a5f792b845")
}

fn nearby_planet() -> CardIndex {
    card_index("19d34126-8266-4da1-b7ef-67ecfa2dbbee")
}

fn northampton_farm() -> CardIndex {
    card_index("e05f1a43-16ce-4e88-b0ad-2202efb25516")
}

fn nykthos_shrine_to_nyx() -> CardIndex {
    card_index("84dc18f0-8225-4b40-a165-b10321e41769")
}

fn oakhollow_village() -> CardIndex {
    card_index("177b7fe4-8565-4631-b4d9-8b2b4282f3ac")
}

fn oscorp_industries() -> CardIndex {
    card_index("f432eb6a-f1bf-4ce7-b915-1488dccb4bb9")
}

fn petrified_hamlet() -> CardIndex {
    card_index("78a2972c-14f4-41f3-99f9-167948bdd73a")
}

fn port_of_karfell() -> CardIndex {
    card_index("f500a8a4-6135-448c-9116-bd2695a72229")
}

fn river_delta() -> CardIndex {
    card_index("4c6c064c-da61-4c7a-9607-8fb4490ff9a6")
}

fn rockface_village() -> CardIndex {
    card_index("7e103748-3f76-42ce-a063-d0256b2dce2b")
}

fn rustic_clachan() -> CardIndex {
    card_index("cde68428-0033-4ede-92f1-ab91de0a41fb")
}

fn safe_haven() -> CardIndex {
    card_index("8f7f4061-8998-40e9-b980-c63f27e7a5cd")
}

fn scrying_sheets() -> CardIndex {
    card_index("0f98e055-ab61-4314-aae8-9d3c19f66acf")
}

fn secluded_starforge() -> CardIndex {
    card_index("69f55a7c-6ddf-412e-b63b-b395731a1ff2")
}

fn sheltered_valley() -> CardIndex {
    card_index("cd535fa3-6fd8-4227-97fd-3ef07cb0598d")
}

fn skemfar_elderhall() -> CardIndex {
    card_index("70965b80-c8ad-4718-ae20-12a4d8228898")
}

fn sokenzan_crucible_of_defiance() -> CardIndex {
    card_index("c5ee72d5-3a9e-4fe5-8802-3286ee612055")
}

fn soldevi_excavations() -> CardIndex {
    card_index("5baa7abe-5bdf-40ce-9a83-a93b7cae71a3")
}

fn stardew_valley() -> CardIndex {
    card_index("6a4ee425-b3b8-487d-866c-9e2d73682466")
}

fn starlit_sanctum() -> CardIndex {
    card_index("d16298ac-67bd-4f9d-9979-23c1b7e4b359")
}

fn swarmyard() -> CardIndex {
    card_index("4b508087-99da-4eb1-8b12-29162f2ec85d")
}

fn tarnation_vista() -> CardIndex {
    card_index("b0be3f25-edb1-4299-959c-9aad909730ca")
}

fn thawing_glaciers() -> CardIndex {
    card_index("c6792d9f-8b74-43c4-814f-ba4adab2fdea")
}

fn the_great_mound() -> CardIndex {
    card_index("6b78e417-44f8-4ab6-9d3c-e71704fc648e")
}

fn the_lonely_mountain() -> CardIndex {
    card_index("3678c06f-8a33-4a6d-bf20-5b92d5c05a95")
}

fn three_tree_city() -> CardIndex {
    card_index("da3b17a2-e1e1-44e9-b9b1-ae54a92037db")
}

fn timberline_ridge() -> CardIndex {
    card_index("a597d2b7-484b-4cfd-89a6-d166cb1a3420")
}

fn tolaria_west() -> CardIndex {
    card_index("46eefc72-2d9e-4389-8ae1-26d9ee472b5c")
}

fn tomb_fortress() -> CardIndex {
    card_index("986f510c-e2ec-423e-a443-51a169939558")
}

fn tyrite_sanctum() -> CardIndex {
    card_index("38c83332-2d21-450c-ae97-34109b565f59")
}

fn underdark_rift() -> CardIndex {
    card_index("199c7604-6b4b-4cec-b6c2-b6b2918b11c8")
}

fn urban_retreat() -> CardIndex {
    card_index("18290c4b-cff6-4ee5-a487-fe0d5706d3de")
}

fn urza_s_fun_house() -> CardIndex {
    card_index("6538fb05-7cc6-4bb9-9b57-11f4f07b5e59")
}

fn veldt() -> CardIndex {
    card_index("10b9bfb3-c478-45c6-b227-9c66b63bc79b")
}

fn westvale_abbey() -> CardIndex {
    card_index("04eeb9ad-5c59-411b-8809-db8349838588")
}

/// The ten original dual lands, which print **no ability at all** — the
/// parenthesis on the card is reminder text for what CR 305.6 already gives
/// a land with two basic types.
///
/// One row per card, because the thing worth proving is a pair of sentences
/// that only a two-typed land can say. A land with *one* basic type is
/// offered through `LegalActions::mana_abilities`, the CR 305.6 shortcut,
/// and never has to be asked anything; `casting::intrinsic_mana` answers
/// `None` the moment there are two, because the shortcut would have to pick
/// a colour on the player's behalf. So a dual is **absent** from that list
/// and carries the mana as an ordinary printed ability instead — the one
/// `landgen::intrinsic_mana_ability` writes and the transcoder puts back on
/// whatever a script reader produced.
///
/// That absence is the half a test is needed for. A dual whose ability went
/// missing still has its two subtypes, still reads as a correct card from
/// every other side, and simply makes no mana — which is the failure
/// `CLAUDE.md` calls fatal for exactly these cards.
const ORIGINAL_DUALS: &[(&str, ManaColor, ManaColor)] = &[
    (
        "02418479-9455-417f-a6a1-004356faff37",
        ManaColor::White,
        ManaColor::Blue,
    ), // Tundra
    (
        "4b22be3a-8ce1-47d1-b82e-6c3ccfb0548b",
        ManaColor::Blue,
        ManaColor::Black,
    ), // Underground Sea
    (
        "13ff3222-91cb-4796-a34e-899ed817694c",
        ManaColor::Black,
        ManaColor::Red,
    ), // Badlands
    (
        "22e3cf1d-3559-4ce1-954c-8dc815342979",
        ManaColor::Red,
        ManaColor::Green,
    ), // Taiga
    (
        "703243f0-8cb3-420f-958f-5fd4bde30293",
        ManaColor::Green,
        ManaColor::White,
    ), // Savannah
    (
        "c8d95ca8-7d12-4072-aeaf-e20f248c7e39",
        ManaColor::White,
        ManaColor::Black,
    ), // Scrubland
    (
        "c718911c-c955-4eb9-9e16-be4bd49a4e4e",
        ManaColor::Blue,
        ManaColor::Red,
    ), // Volcanic Island
    (
        "b76d1ae6-ad1d-4bac-b4c3-2e03e0e84d9b",
        ManaColor::Black,
        ManaColor::Green,
    ), // Bayou
    (
        "c7a15ca4-085f-4d92-8387-c3711c04c8fa",
        ManaColor::Red,
        ManaColor::White,
    ), // Plateau
    (
        "74b7fe23-5d3a-4092-8d78-7c0eba8f6f73",
        ManaColor::Green,
        ManaColor::Blue,
    ), // Tropical Island
];

fn an_havva_township() -> CardIndex {
    card_index("40ae17be-9998-4ee4-9d95-82a08895405f")
}

fn ancient_den() -> CardIndex {
    card_index("02f16726-f2f6-4943-b71a-93f8e26251d3")
}

fn ancient_ziggurat() -> CardIndex {
    card_index("0baabe39-72ae-47bd-a095-cbf7eb8a6361")
}

fn archaeological_dig() -> CardIndex {
    card_index("cf438848-da86-4db6-b3b8-4dd8570be3b8")
}

fn aysen_abbey() -> CardIndex {
    card_index("6ff85e73-bf7a-4a9c-80ef-6ce76656fab7")
}

fn canopy_vista() -> CardIndex {
    card_index("dcb7e046-f01b-497c-88e5-57794eb30ce5")
}

fn cinder_glade() -> CardIndex {
    card_index("dfac0258-e148-4d7d-8ded-fc2466d9caa6")
}

fn darkmoss_bridge() -> CardIndex {
    card_index("2065cada-4078-41c4-9e06-2460d2a2e8ee")
}

fn darksteel_citadel() -> CardIndex {
    card_index("8dc067bf-f78f-4ac4-b6e7-b305c42cf0bc")
}

fn drossforge_bridge() -> CardIndex {
    card_index("44b83535-fdbf-4307-bf53-ca20470a768d")
}

fn eclipsed_steppe() -> CardIndex {
    card_index("6216635f-8e6e-40a3-9659-ef6352ab92ce")
}

fn goldmire_bridge() -> CardIndex {
    card_index("c9b7ea9c-3bcb-4538-aa25-cdb82a52037e")
}

fn great_furnace() -> CardIndex {
    card_index("f4819061-b0b5-48ab-af7b-6525c3d2eab7")
}

fn mistvault_bridge() -> CardIndex {
    card_index("33ee23bc-6327-4a54-a704-dfd83be36bb5")
}

fn razortide_bridge() -> CardIndex {
    card_index("6cb37ac1-dd11-4a8c-bca5-ef44d828059f")
}

fn rustvale_bridge() -> CardIndex {
    card_index("a3faf70d-c034-4692-9e92-1922029e3852")
}

fn seat_of_the_synod() -> CardIndex {
    card_index("39451b4d-cd7a-40da-b457-cb51b609173f")
}

fn silverbluff_bridge() -> CardIndex {
    card_index("081bfd50-a436-463b-9d2c-5bc8a32b387c")
}

fn slagwoods_bridge() -> CardIndex {
    card_index("e040a8e6-b90c-42d1-a1b1-771d954c61ab")
}

fn snow_covered_forest() -> CardIndex {
    card_index("5f0d3be8-e63e-4ade-ae58-6b0c14f2ce6d")
}

fn snow_covered_island() -> CardIndex {
    card_index("5b2460a5-6ae5-4cad-ba94-1a9e98e6e4c0")
}

fn snow_covered_mountain() -> CardIndex {
    card_index("ca9f660b-e07d-4f42-a46e-abd0ca72510c")
}

fn snow_covered_plains() -> CardIndex {
    card_index("ac8cc74d-e43b-4118-bba0-dfa8b9c04d45")
}

fn snow_covered_swamp() -> CardIndex {
    card_index("d8239a86-7184-4005-ba1e-2dddcd756c47")
}

fn snow_covered_wastes() -> CardIndex {
    card_index("46a07b53-ff58-4bd6-80dd-ded2eb0e29a3")
}

// oracle_id = "29cd8a7c-108a-43d1-af63-f603a27c24f2"
fn tanglepool_bridge() -> CardIndex {
    card_index("29cd8a7c-108a-43d1-af63-f603a27c24f2")
}

fn thornglint_bridge() -> CardIndex {
    card_index("99720c65-be96-4220-8ed4-720660bf6928")
}

fn tree_of_tales() -> CardIndex {
    card_index("8b4aa971-b919-4750-8388-33d4f42c9280")
}

fn vault_of_whispers() -> CardIndex {
    card_index("09496421-74e4-466a-9546-56f2a0c8eef4")
}

fn wastes() -> CardIndex {
    card_index("05d24b0c-904a-46b6-b42a-96a4d91a0dd4")
}

fn barren_moor() -> CardIndex {
    card_index("326ba371-124c-4949-a048-3a0c8962e567")
}

fn blasted_landscape() -> CardIndex {
    card_index("9c8007ac-4b3d-4444-93e9-f583185e5d81")
}

fn canyon_slough() -> CardIndex {
    card_index("2031b17c-0536-446f-a9aa-b46fe79b7ea7")
}

fn castle_sengir() -> CardIndex {
    card_index("c7f0251a-9341-4ff2-8b15-31c06eb4f2e7")
}

fn castle_vantress() -> CardIndex {
    card_index("cdf41cf4-4e77-453d-be5b-0abbbd358934")
}

fn clifftop_retreat() -> CardIndex {
    card_index("d7faa3c8-46cf-46b2-bfa4-89000307cf18")
}

fn coastal_peak() -> CardIndex {
    card_index("0e25faa2-efb0-4ca5-b280-18c38faa860c")
}

// oracle_id = "ca68648f-fe3a-4770-9842-a3dc2310f099"
fn crystal_quarry() -> CardIndex {
    card_index("ca68648f-fe3a-4770-9842-a3dc2310f099")
}

fn crystal_vein() -> CardIndex {
    card_index("616d6013-24f4-4999-9bf3-5b0764e52fa6")
}

fn dragonskull_summit() -> CardIndex {
    card_index("63398c02-6fb1-481d-9d9f-81063532fbc0")
}

fn drifting_meadow() -> CardIndex {
    card_index("c6eb2814-0021-4308-ad44-6c8cc59b0d1c")
}

fn drowned_catacomb() -> CardIndex {
    card_index("819fc966-434e-470f-91e9-a38df974ad17")
}

fn festering_thicket() -> CardIndex {
    card_index("bdb9b2ce-342c-4935-8943-d0c3971b1e38")
}

fn fetid_pools() -> CardIndex {
    card_index("32b03b48-06da-4a74-a7ac-e5ae39a4f428")
}

fn forgotten_cave() -> CardIndex {
    card_index("394c6de5-7957-4a0b-a6b9-ee0c707cd022")
}

fn glittering_massif() -> CardIndex {
    card_index("23732d20-aba6-46cc-89d0-0542629bc6b5")
}

fn hinterland_harbor() -> CardIndex {
    card_index("fb5a3403-7f0b-406c-8c4f-d693be010ca6")
}

// Isolated Chapel — (no cost) — Land
// Oracle: This land enters tapped unless you control a Plains or a Swamp.
// Oracle: {T}: Add {W} or {B}.

fn isolated_chapel() -> CardIndex {
    card_index("7e5d9efe-48a9-434b-bb09-056e0e09cc9a")
}

fn lonely_sandbar() -> CardIndex {
    card_index("765863c8-1be0-4bb1-9e9c-db7701cffde3")
}

fn polluted_mire() -> CardIndex {
    card_index("9809d975-7ef8-4946-9041-607c4e954b13")
}

fn radiant_summit() -> CardIndex {
    card_index("5dd0cc44-4647-4857-ad3b-22494099d08a")
}

fn rootbound_crag() -> CardIndex {
    card_index("9516c4c1-d72d-434f-97e1-6a862434a169")
}

fn scorched_geyser() -> CardIndex {
    card_index("f808b510-907a-4c3c-aea1-efb825c8e13e")
}

fn smoldering_marsh() -> CardIndex {
    card_index("390f1b56-264e-4336-83be-dc1fe79bfdcf")
}

// oracle_id = "0070db93-142b-4d04-afd3-836792dc134b"
fn sodden_verdure() -> CardIndex {
    card_index("0070db93-142b-4d04-afd3-836792dc134b")
}

fn sulfur_falls() -> CardIndex {
    card_index("6a6c5e17-6465-4a1f-9d63-8a3ce2edc522")
}

fn sunpetal_grove() -> CardIndex {
    card_index("402ec768-76fb-474e-ae74-babc90d833c4")
}

fn urza_s_cave() -> CardIndex {
    card_index("4474ecee-0ec3-409b-90df-738d9313fe3c")
}

fn vernal_fen() -> CardIndex {
    card_index("40544d12-0391-4a61-af95-9b8ec01ed8fc")
}

// oracle_id = "c9fe1383-1331-4a58-a45a-3320250221a9"
fn woodland_cemetery() -> CardIndex {
    card_index("c9fe1383-1331-4a58-a45a-3320250221a9")
}

/// The three cycling triple lands. Their mana is the same shape as a dual's
/// one level up, and their second ability is the one thing a land almost
/// never has: an activation from the **hand**.
const CYCLING_TRIOMES: &[(&str, [ManaColor; 3])] = &[
    (
        "6e9ef5ef-6aed-4d3e-a59b-9e3dc8740b1b",
        [ManaColor::White, ManaColor::Blue, ManaColor::Black],
    ), // Raffine's Tower
    (
        "c7fa1dda-9312-4ec8-82cd-a1ba7bc33497",
        [ManaColor::Blue, ManaColor::Red, ManaColor::White],
    ), // Raugrin Triome
    (
        "fdd46004-eaba-4024-8687-39b23dc6a58c",
        [ManaColor::Blue, ManaColor::Black, ManaColor::Green],
    ), // Zagoth Triome
];

/// A module of its own, so the sibling in `creatures.rs` is out of reach.
fn ragavan_nimble_pilferer() -> CardIndex {
    card_index("37108cd4-bbab-4ce3-9ed6-f60e8422e703")
}

/// A module of its own, so the sibling in `creatures.rs` is out of reach.
fn ignoble_hierarch() -> CardIndex {
    card_index("c8de43a3-ebd3-4000-b343-a6ffed11d34d")
}

fn wasteland() -> CardIndex {
    card_index("09a70ae8-3859-4a09-901d-dce063fa3b5f")
}

fn karakas() -> CardIndex {
    card_index("59119143-c0fa-49dd-adf0-e2fd3029c48b")
}

fn temple_garden() -> CardIndex {
    card_index("f413a83d-a40d-434c-b20a-4c707c0527fa")
}

fn watery_grave() -> CardIndex {
    card_index("fc9ec820-4245-4a96-b009-5308a818ca58")
}

fn city_of_brass() -> CardIndex {
    card_index("f25351e3-539b-4bbc-b92d-6480acf4d722")
}

fn reliquary_tower() -> CardIndex {
    card_index("c23e5b80-08d2-4e24-9908-fe2aa4f30f6f")
}

fn volrath_s_stronghold() -> CardIndex {
    card_index("73b8cf90-3c71-4f8b-a29f-61894b7f27c9")
}

fn riptide_laboratory() -> CardIndex {
    card_index("444d50dd-a44a-42db-bbf6-d0978e3bd6a3")
}

fn ana_disciple() -> CardIndex {
    card_index("f543dfcd-015e-48bc-851d-08002d0241fa")
}

/// The two remaining Pathways, played on **both** faces.
///
/// A modal double-faced land is one card that is two lands, and the half
/// that is easy to lose is the front one: a `ChooseCastMode` that quietly
/// defaulted, or a `face_index` that was written and never read, would still
/// let face 1 work and would still look right in a test that only ever asked
/// for the back. So each card is played twice, once per face, and what is
/// asserted each time is the pair `(face_index, colour)` — because either
/// alone can be right while the card is wrong.
const PATHWAYS: &[(&str, ManaColor, ManaColor)] = &[
    (
        "144119bc-7fd1-45c5-9e29-f742e7c255ac",
        ManaColor::Blue,
        ManaColor::Black,
    ), // Clearwater // Murkwater
    (
        "461b3f2f-fcee-4160-abfa-061f8b6a784f",
        ManaColor::White,
        ManaColor::Blue,
    ), // Hengegate // Mistgate
];

fn exotic_orchard() -> CardIndex {
    card_index("27b047e3-0d41-45e2-98e9-9391d7923a1e")
}

fn bad_river() -> CardIndex {
    card_index("259fc423-d078-4668-9e95-8c1a0d2e49ba")
}

// oracle_id = "b76d1ae6-ad1d-4bac-b4c3-2e03e0e84d9b"
fn bayou() -> CardIndex {
    card_index("b76d1ae6-ad1d-4bac-b4c3-2e03e0e84d9b")
}

// oracle_id = "9cbc9f83-8979-42a5-a466-a8d89c8e6de8"
fn bristling_backwoods() -> CardIndex {
    card_index("9cbc9f83-8979-42a5-a466-a8d89c8e6de8")
}

fn creosote_heath() -> CardIndex {
    card_index("c116b787-5f7e-47ef-a694-58709770dd32")
}

// oracle_id = "311f38a6-f68f-4d30-bc4e-62339f1e0d88"
fn desert_of_the_fervent() -> CardIndex {
    card_index("311f38a6-f68f-4d30-bc4e-62339f1e0d88")
}

fn desert_of_the_glorified() -> CardIndex {
    card_index("190e664b-9875-4335-af46-353886ed18ff")
}

fn desert_of_the_mindful() -> CardIndex {
    card_index("508f9e7e-2ff7-4593-b0a9-0612d7b5d646")
}

fn desert_of_the_true() -> CardIndex {
    card_index("2672e0ca-8d5c-449f-8483-35d0e697fbb2")
}

fn festering_gulch() -> CardIndex {
    card_index("9d3b60af-3e38-4d36-95fc-11b31c38f955")
}

// oracle_id = "9043d8d5-b38a-406f-a44c-49f579c644f0"
fn flood_plain() -> CardIndex {
    card_index("9043d8d5-b38a-406f-a44c-49f579c644f0")
}

fn forlorn_flats() -> CardIndex {
    card_index("ebb3e2ff-2214-4e11-88fb-e0fa84288cf1")
}

fn grasslands() -> CardIndex {
    card_index("e80bd454-8bc5-4921-90cf-6ad28a27a88b")
}

fn jagged_barrens() -> CardIndex {
    card_index("64ee02f1-afdb-474b-a893-31538ad7219a")
}

fn lonely_arroyo() -> CardIndex {
    card_index("36508a8a-d1a4-400e-bfda-09436bc4d5d4")
}

fn lush_oasis() -> CardIndex {
    card_index("b6a965eb-cffb-41c1-925a-7cf3e8e2f248")
}

fn painted_bluffs() -> CardIndex {
    card_index("b66deeb5-7371-4f06-b10e-d65165bc07b2")
}

fn plateau() -> CardIndex {
    card_index("c7a15ca4-085f-4d92-8387-c3711c04c8fa")
}

fn rain_slicked_copse() -> CardIndex {
    card_index("633a41f0-3889-49a0-b2fc-9baf59f4ddc2")
}

fn remote_isle() -> CardIndex {
    card_index("24aebda0-315f-4d2f-8bd9-00bbaf5bd76a")
}

fn scattered_groves() -> CardIndex {
    card_index("3c87ea85-ca29-45a7-b5b2-758c62898b0a")
}

fn secluded_steppe() -> CardIndex {
    card_index("8dec6fcf-1254-4b1b-ba23-7a3e492a7241")
}

// oracle_id = "db8d8643-3d0b-4f20-bf53-f4cd26a0e8df"
fn sheltered_thicket() -> CardIndex {
    card_index("db8d8643-3d0b-4f20-bf53-f4cd26a0e8df")
}

fn slippery_karst() -> CardIndex {
    card_index("58bfd9a1-67ce-41d0-be38-f05addd1dd9e")
}

// oracle_id = "be09e83f-6486-4f42-8d2e-416cb95173f9"
fn smoldering_crater() -> CardIndex {
    card_index("be09e83f-6486-4f42-8d2e-416cb95173f9")
}

fn soured_springs() -> CardIndex {
    card_index("e579edd7-4f6c-4f22-a72f-0a20d7a698a2")
}

fn tranquil_thicket() -> CardIndex {
    card_index("9f8fe514-77ed-41b4-a6f3-c6f095bb97be")
}

// oracle_id = "74b7fe23-5d3a-4092-8d78-7c0eba8f6f73"
fn tropical_island() -> CardIndex {
    card_index("74b7fe23-5d3a-4092-8d78-7c0eba8f6f73")
}

fn umbral_expanse() -> CardIndex {
    card_index("5a1dfc60-645d-4bc0-8883-e06ed9c80706")
}

fn volcanic_island() -> CardIndex {
    card_index("c718911c-c955-4eb9-9e16-be4bd49a4e4e")
}

fn a_i_m_labs() -> CardIndex {
    card_index("8417f2d5-93c9-474a-a143-5c4e17097a88")
}

fn akoum_refuge() -> CardIndex {
    card_index("354fecd1-2371-49e3-81c6-7e47728dbb1f")
}

fn asgardian_citadel() -> CardIndex {
    card_index("e3614df6-ce84-4180-ade1-a1751e8e3c9a")
}

fn avengers_hangar() -> CardIndex {
    card_index("10fc2009-4212-47c6-8a97-ab0f45c1f91b")
}

fn birnin_zana_plaza() -> CardIndex {
    card_index("c3172c5d-c745-4422-b651-cd414c8da11b")
}

fn bloodfell_caves() -> CardIndex {
    card_index("64e29bfc-9313-4e8c-808c-bc27f6b018a6")
}

fn blossoming_sands() -> CardIndex {
    card_index("45429b2c-be3b-4b2e-9bab-a059ccbda8cd")
}

// Crypt of the Eternals — (no cost) — Land
// Oracle: When this land enters, you gain 1 life.
// Oracle: {T}: Add {C}.
// Oracle: {1}, {T}: Add {U}, {B}, or {R}.

fn crypt_of_the_eternals() -> CardIndex {
    card_index("cc78776b-822b-4f11-8982-0805a25a9d36")
}

fn darkwater_catacombs() -> CardIndex {
    card_index("4869a530-757f-4364-8d8e-4dc8001f433c")
}

fn desolate_mire() -> CardIndex {
    card_index("3edf9201-265f-4cd9-b27b-8073bf1a4cf2")
}

fn dimension_x() -> CardIndex {
    card_index("8683341a-e836-41e5-9034-4da09eb0ab42")
}

fn dismal_backwater() -> CardIndex {
    card_index("865a2194-fca0-446e-aae3-ca475cd66e00")
}

fn ferrous_lake() -> CardIndex {
    card_index("62c15af0-40e1-407d-b056-7a3d909e3fdb")
}

fn fisk_tower() -> CardIndex {
    card_index("5554db95-9676-4c24-b82a-8173513d7927")
}

fn foot_headquarters() -> CardIndex {
    card_index("063dd25b-d4b4-4e09-acae-8c52dcb40803")
}

fn graypelt_refuge() -> CardIndex {
    card_index("60b36821-0fad-423c-98c4-f64d991719f3")
}

fn hell_s_kitchen() -> CardIndex {
    card_index("1337d89e-a883-4568-a14f-9ca3305cc3df")
}

fn illegitimate_business() -> CardIndex {
    card_index("01ee70d4-6527-45ae-9600-bd3d16b3037d")
}

fn jungle_hollow() -> CardIndex {
    card_index("6de714e1-446d-4fb9-9e3d-bcd3ec6af9ca")
}

fn mossfire_valley() -> CardIndex {
    card_index("23bbd091-f6ff-4514-97aa-42c08164b4eb")
}

fn mountain_valley() -> CardIndex {
    card_index("0b7393aa-d563-45bc-9946-8e7d1729d498")
}

fn overflowing_basin() -> CardIndex {
    card_index("5ac8e01c-b0a7-4855-a122-1cd26b07c4a5")
}

fn rocky_tar_pit() -> CardIndex {
    card_index("8709b5b1-ef9e-45b2-bf4f-ef4c4d613dcd")
}

fn shadowblood_ridge() -> CardIndex {
    card_index("15687ee3-3cdb-4a8f-a726-46b73bceb792")
}

fn skycloud_expanse() -> CardIndex {
    card_index("76f335d0-7f71-4b1a-b60d-73de954cbe2c")
}

fn sungrass_prairie() -> CardIndex {
    card_index("0a28dff0-2bd6-4105-b73a-b6c4735833fd")
}

fn sunscorched_divide() -> CardIndex {
    card_index("8d2b2675-19df-4f40-9e8e-196ec097b91c")
}

fn verdant_catacombs() -> CardIndex {
    card_index("67d60b24-d429-4ded-90d9-06e49f28c396")
}

fn viridescent_bog() -> CardIndex {
    card_index("6bd6d259-1af7-4dff-a79c-48a616d2a36e")
}

fn wooded_foothills() -> CardIndex {
    card_index("6587a463-a108-4854-b6d1-944e89b8c8a4")
}

fn dimir_guildgate() -> CardIndex {
    card_index("52d14717-0cbc-4d7e-b546-54ea91580338")
}

fn fiery_islet() -> CardIndex {
    card_index("026f4a4b-eedd-44e1-9d37-ca4fb8d6db98")
}

fn golgari_guildgate() -> CardIndex {
    card_index("fa2da325-6859-45bb-b185-35526b01bcc1")
}

fn gruul_guildgate() -> CardIndex {
    card_index("d38476e9-2e47-4c0c-8129-483c0bd09ec0")
}

/// Henge of Ramos is a land made of two mana abilities and nothing else:
/// "{T}: Add {C}" and "{2}, {T}: Add one mana of any color." Both halves are
/// played because they differ in exactly the way a mana ability can be
/// misread — the second costs two mana *in addition to* its own tap symbol,
/// so the offer carries it only as long as the pool covers it (CR 601.2h) —
/// and because the question it asks is the whole color wheel without
/// colorless (CR 105.4). The {2} tap is paid with two Forests tapped first,
/// while the Henge is named as the held-back source; the bare {T} is read one
/// turn later, when the untap step has put the land back up and the pool is
/// empty, so that "a colorless and nothing else" is exact.
fn henge_of_ramos() -> CardIndex {
    card_index("829474df-6413-4323-aef6-f878cb0e797c")
}

fn horizon_canopy() -> CardIndex {
    card_index("262a5d83-506c-4781-9bc9-1a2b5d83955c")
}

fn izzet_guildgate() -> CardIndex {
    card_index("bf75a3d1-f184-4b48-a913-21caee1db084")
}

fn jwar_isle_refuge() -> CardIndex {
    card_index("8b96f837-7c32-473a-b5ae-1d66527eaf7b")
}

fn kazandu_refuge() -> CardIndex {
    card_index("03332772-18b0-446a-a7e2-a4bdf38c4f0c")
}

fn los_diablos_missile_base() -> CardIndex {
    card_index("4b17048f-8c7f-4e29-8f56-16db4c2106e6")
}

fn mutant_town() -> CardIndex {
    card_index("f6059731-2e10-4f72-a214-b257f1677e9e")
}

fn nurturing_peatland() -> CardIndex {
    card_index("8ed932ff-986c-4592-ad70-53b3fac80d69")
}

fn orzhov_guildgate() -> CardIndex {
    card_index("57b37df5-fee4-4720-931f-f0cb0a8b338c")
}

fn pym_technologies() -> CardIndex {
    card_index("5a8977fc-0732-4807-b876-01819aedd70b")
}

// oracle_id = "361f534b-39d1-4421-b5a8-d3813c62f86d"
fn rakdos_guildgate() -> CardIndex {
    card_index("361f534b-39d1-4421-b5a8-d3813c62f86d")
}

fn rugged_highlands() -> CardIndex {
    card_index("6c922206-6e68-4dcd-9559-88da1074f2c4")
}

fn scoured_barrens() -> CardIndex {
    card_index("d37f858e-03c8-4594-9b92-cd03699a1591")
}

fn sejiri_refuge() -> CardIndex {
    card_index("73b3e242-075d-4c4d-9b09-6fef1633c348")
}

fn selesnya_guildgate() -> CardIndex {
    card_index("75b235d3-595a-4859-be45-9559d8445db5")
}

fn silent_clearing() -> CardIndex {
    card_index("fd45063f-c83c-431c-9104-f139c497ec0d")
}

fn simic_guildgate() -> CardIndex {
    card_index("e8705df9-6439-4930-91b6-229f818559af")
}

fn stark_industries() -> CardIndex {
    card_index("52401cb4-1bce-4191-9220-0d93ea4108b8")
}

fn subterranean_cavern() -> CardIndex {
    card_index("d49a7525-eca5-48e1-b94b-d618a706fd02")
}

fn sunbaked_canyon() -> CardIndex {
    card_index("f97fd068-b83a-4621-bf8c-cc96e880ce90")
}

fn swiftwater_cliffs() -> CardIndex {
    card_index("2f4ad084-2062-44c0-9975-15f100204531")
}

fn tcri_building() -> CardIndex {
    card_index("4139f40a-873b-4842-96a2-b7875de1c20a")
}

fn thornwood_falls() -> CardIndex {
    card_index("ec96cde2-f1e6-495c-94e2-3e8ae79e556c")
}

fn tranquil_cove() -> CardIndex {
    card_index("5d641bf6-0f93-4189-8dc1-ec7ea446dade")
}

fn waterlogged_grove() -> CardIndex {
    card_index("70fa2eba-565e-4fed-adc9-7f5d9fcbf1fa")
}

// oracle_id = "b0af0c54-2a59-4075-8543-d41ff20c4c87"
fn wind_scarred_crag() -> CardIndex {
    card_index("b0af0c54-2a59-4075-8543-d41ff20c4c87")
}

fn adarkar_wastes() -> CardIndex {
    card_index("d5ad26cc-2bdb-46b7-b8bf-dd099d5fa09b")
}

fn ancient_tomb() -> CardIndex {
    card_index("23467047-6dba-4498-b783-1ebc4f74b8c2")
}

// oracle_id = "6b75b94e-83b7-457e-ac41-7ca90b5a59aa"
fn battlefield_forge() -> CardIndex {
    card_index("6b75b94e-83b7-457e-ac41-7ca90b5a59aa")
}

fn brushland() -> CardIndex {
    card_index("5eb8b497-ec9a-4a89-ad29-1ec3ca82da7c")
}

fn caldera_lake() -> CardIndex {
    card_index("c737d27b-db14-4bd4-8f16-bcbd4401c47b")
}

fn caves_of_koilos() -> CardIndex {
    card_index("33de01e9-ce5a-42d4-afcb-343cd54a6d80")
}

fn celestial_colonnade() -> CardIndex {
    card_index("876ac6f6-74de-4666-84c6-83d81f054723")
}

fn faerie_conclave() -> CardIndex {
    card_index("0c25f6b1-8fb3-4406-9605-0282d2dbbcec")
}

fn forbidding_watchtower() -> CardIndex {
    card_index("cabf7953-0fac-4dbb-b3ae-05e85e02b3fc")
}

fn ghitu_encampment() -> CardIndex {
    card_index("9cc02c16-8cf9-4ac7-9475-231c27121968")
}

fn grand_coliseum() -> CardIndex {
    card_index("1cea9b82-d2e9-4758-8ec8-729fcf4bb7d7")
}

fn hissing_quagmire() -> CardIndex {
    card_index("7fff5224-c002-4a32-86fe-9a4b18d78b50")
}

fn karplusan_forest() -> CardIndex {
    card_index("bd912666-f37f-4767-af6f-9e6d0fcccacf")
}

// oracle_id = "184c5a0d-7654-4421-86e4-7f04bcf49494"
fn koskun_keep() -> CardIndex {
    card_index("184c5a0d-7654-4421-86e4-7f04bcf49494")
}

fn llanowar_wastes() -> CardIndex {
    card_index("32116127-cf96-4a1b-8896-a1ebc087b597")
}

fn lumbering_falls() -> CardIndex {
    card_index("c2878f4e-3044-4d60-8ec1-b325edfca397")
}

fn mana_confluence() -> CardIndex {
    card_index("d0ee5bdc-2b69-4b73-9a20-ffcc18783b29")
}

/// Mishra's Workshop — Land: "{T}: Add {C}{C}{C}. Spend this mana only to
/// cast artifact spells."
///
/// The rider is the whole card, so the board is built so that nothing else can
/// account for the difference: two Workshops make six colourless, and the hand
/// holds Panharmonicon ({4}, an artifact) beside Karn, the Great Creator ({4},
/// a planeswalker). Same cost, same pool, and the only thing separating the
/// two is the printed sentence — an engine that read the mana as ordinary would
/// offer both, and one that never produced it would offer neither. The artifact
/// is then actually cast off that mana, so the offer is not just a claim.
fn mishras_workshop() -> CardIndex {
    card_index("ba284fe6-bb29-455c-8321-9714a0cdc05e")
}

fn needle_spires() -> CardIndex {
    card_index("e7bb8160-0a4b-4e46-b196-7a19fb388d8e")
}

fn pine_barrens() -> CardIndex {
    card_index("603dc263-270c-4a16-9b75-41f36fd7dfde")
}

fn restless_bivouac() -> CardIndex {
    card_index("b3c7b46f-c9ab-40ca-b50b-a4e0d0bd9be8")
}

fn restless_reef() -> CardIndex {
    card_index("df20f85b-5f81-4ee4-8487-55d90109ac36")
}

fn salt_flats() -> CardIndex {
    card_index("7a951bd7-4f7d-44ea-9c2f-fe2f6b2f5289")
}

fn scabland() -> CardIndex {
    card_index("2896f01d-003b-4d68-9d0a-64990ba59cbe")
}

fn shambling_vent() -> CardIndex {
    card_index("4725abd4-06bc-464a-bc5b-c9e0f71ec079")
}

fn shivan_reef() -> CardIndex {
    card_index("0fe16212-66c3-4e45-a641-7391e9b2e304")
}

fn skyshroud_forest() -> CardIndex {
    card_index("117a5fae-7fc4-4e24-b646-5727ea392fa7")
}

fn stirring_wildwood() -> CardIndex {
    card_index("e900f211-dc8b-40f7-a217-1b1462b15c26")
}

fn sulfurous_springs() -> CardIndex {
    card_index("f5c38c01-4a40-469f-91a0-7479daf4e8e7")
}

fn tarnished_citadel() -> CardIndex {
    card_index("66ae2562-68e9-4c77-ba0a-57f8ff37f656")
}

fn bountiful_landscape() -> CardIndex {
    card_index("2eb69a8f-9456-4852-b868-85ae609d3441")
}

fn castle_ardenvale() -> CardIndex {
    card_index("f8f4fc60-725d-46d8-8e8f-e68e00d20589")
}

fn contaminated_landscape() -> CardIndex {
    card_index("28196fd9-00c9-4cd0-b603-0eec8511ec79")
}

fn deceptive_landscape() -> CardIndex {
    card_index("1831fe12-dbe0-437f-8fc8-f01bbb701fe1")
}

fn eroded_canyon() -> CardIndex {
    card_index("852c6520-d148-4923-a312-05a9af821f24")
}

fn foreboding_landscape() -> CardIndex {
    card_index("bcfe1653-e602-4d38-abe7-bfcc7f203f9d")
}

fn gates_of_istfell() -> CardIndex {
    card_index("6f85c26e-3c87-4112-ad1a-8a5708555a93")
}

fn glacial_fortress() -> CardIndex {
    card_index("027dd013-baa7-4111-b3c9-f4d1414e9c45")
}

fn grove_of_the_burnwillows() -> CardIndex {
    card_index("d33c3fbb-8306-4c2d-b0dd-88f12639da94")
}

fn kabira_crossroads() -> CardIndex {
    card_index("b3dbb16f-fa8f-4406-bcf3-e647e4337619")
}

fn lorehold_campus() -> CardIndex {
    card_index("45773715-3f46-4671-b633-bf087e892e26")
}

fn maelstrom_of_the_spirit_dragon() -> CardIndex {
    card_index("49e9fba7-8465-4bbb-95db-73a7e149f494")
}

fn new_benalia() -> CardIndex {
    card_index("6e743fbf-b5b6-4176-a4f2-6933f521f2fe")
}

fn night_market() -> CardIndex {
    card_index("4cdb9f80-d08f-4986-99c8-573166d66082")
}

fn perilous_landscape() -> CardIndex {
    card_index("e2b472dd-047d-47eb-9ebb-df6aa4b52dd4")
}

fn prismari_campus() -> CardIndex {
    card_index("3a3a1b35-ae4d-49d5-ae09-5a1693ad53ce")
}

fn quandrix_campus() -> CardIndex {
    card_index("172f86b6-9580-4eb2-b7dc-2a44277d978b")
}

fn school_of_the_unseen() -> CardIndex {
    card_index("5028dfe8-c505-4643-b493-760b1f19d47f")
}

fn seething_landscape() -> CardIndex {
    card_index("2d8635bd-ed96-4bb1-8718-6962a0eee3d5")
}

fn shattered_landscape() -> CardIndex {
    card_index("7fbad3f2-66f6-4e3a-b9e0-1ddbcd94d42a")
}

fn sheltering_landscape() -> CardIndex {
    card_index("5b932be0-4dac-41b9-9c59-f79e4cecc31a")
}

fn silverquill_campus() -> CardIndex {
    card_index("2b65eb80-6fb7-429f-81f2-2fe125eba634")
}

fn sliver_hive() -> CardIndex {
    card_index("e7286688-ffbe-4d25-ad55-27990f005368")
}

fn temple_of_abandon() -> CardIndex {
    card_index("3baa8e38-ef93-435d-b63e-f781d5bfcc68")
}

fn temple_of_deceit() -> CardIndex {
    card_index("33b9b3bd-33ca-46f3-b8bb-a978bc3d1085")
}

fn temple_of_enlightenment() -> CardIndex {
    card_index("89f43e27-790b-4ca1-8ba7-0882b31e0783")
}

fn tranquil_landscape() -> CardIndex {
    card_index("d4eb65d5-99fd-4daf-b7d3-8ebf99ee9c61")
}

fn twisted_landscape() -> CardIndex {
    card_index("db659cae-2078-423e-a6ed-63898dbab87f")
}

fn underground_river() -> CardIndex {
    card_index("857febd9-cdd7-4f8e-a852-d88084b0cfbc")
}

fn yavimaya_coast() -> CardIndex {
    card_index("40b36bc6-c185-4bda-99e7-0118953c2c97")
}

fn abandoned_outpost() -> CardIndex {
    card_index("5a70ccfa-d12d-4e62-a1a4-f05cda2fd442")
}

fn blood_crypt() -> CardIndex {
    card_index("43985bbc-a0f6-4812-984e-392bc8562633")
}

fn breeding_pool() -> CardIndex {
    card_index("20283c4a-f1f0-42f0-bc08-6da87474426b")
}

fn deathcap_glade() -> CardIndex {
    card_index("f6d24565-5b32-4eff-b2e0-6e2c25516ff0")
}

fn dreamroot_cascade() -> CardIndex {
    card_index("dd8538e6-cd5f-4a88-aff5-eb5e76ce8ddb")
}

fn haunted_ridge() -> CardIndex {
    card_index("e2a37967-4212-4553-9f77-bcb613405807")
}

fn overgrown_farmland() -> CardIndex {
    card_index("709d2f10-1585-48c3-9058-ddd5f62f0452")
}

fn overgrown_tomb() -> CardIndex {
    card_index("975ec9a3-6f20-4177-8211-82526e092538")
}

fn rockfall_vale() -> CardIndex {
    card_index("185c70c1-8403-4ae5-b45d-3679d4ee092a")
}

fn sacred_foundry() -> CardIndex {
    card_index("45181cb8-2090-4471-ba90-e5a8f04d525f")
}

fn shattered_sanctum() -> CardIndex {
    card_index("c854ecb0-cc60-4c48-a9aa-7f2348a7a8c6")
}

fn shimmering_grotto() -> CardIndex {
    card_index("bae49475-fe01-400b-8959-f0dde959577c")
}

fn shipwreck_marsh() -> CardIndex {
    card_index("5f42b67f-87fd-4f98-a0e8-0c8313f4bbc8")
}

fn steam_vents() -> CardIndex {
    card_index("17039058-822d-409f-938c-b727a366ba63")
}

fn stomping_ground() -> CardIndex {
    card_index("16052b52-ade1-406f-a06b-ce7ea607fb63")
}

fn stormcarved_coast() -> CardIndex {
    card_index("4722105b-0085-4bb8-bca1-9de0d3eb5600")
}

fn sundown_pass() -> CardIndex {
    card_index("5ad0b405-cca4-475e-985c-4d7e3599d87e")
}

fn temple_of_epiphany() -> CardIndex {
    card_index("79f94050-d850-41ca-b1db-5ae0cf743f0a")
}

fn temple_of_malady() -> CardIndex {
    card_index("dc55421f-dee8-4263-9df0-2365df5f14bb")
}

fn temple_of_malice() -> CardIndex {
    card_index("7c439c18-31dc-41fe-b03d-3fca06e6fc0b")
}

fn temple_of_mystery() -> CardIndex {
    card_index("7e26f0b7-20e6-46d5-8130-d98c14d6aa29")
}

fn temple_of_plenty() -> CardIndex {
    card_index("e521322b-0e83-458c-8936-7021a80ee279")
}

fn temple_of_silence() -> CardIndex {
    card_index("e6e6fce8-0f6a-4b84-865e-d4e4a4182f9f")
}

fn temple_of_triumph() -> CardIndex {
    card_index("6f0d94d9-64bb-4175-83bc-301e8f79f54f")
}

fn the_autonomous_furnace() -> CardIndex {
    card_index("e2dd05d5-312e-47f1-873e-c0741ee6ef4a")
}

fn the_dross_pits() -> CardIndex {
    card_index("8110fe69-c66c-4e2c-86ee-dcc8dc9a13d1")
}

fn the_fair_basilica() -> CardIndex {
    card_index("8618f8e8-fd86-47e3-905d-e6624b599b9b")
}

fn the_hunter_maze() -> CardIndex {
    card_index("b0ea4975-a944-4585-8711-60f203cffa4a")
}

fn the_surgical_bay() -> CardIndex {
    card_index("db9c155f-b342-41bb-9e1a-50358fb9f40e")
}

fn witherbloom_campus() -> CardIndex {
    card_index("6cd58a88-6434-4c55-bf93-a739b5ed9bc1")
}

fn alpine_meadow() -> CardIndex {
    card_index("8c281ebe-d9a1-48af-b58b-19c55aa4625b")
}

fn ancient_spring() -> CardIndex {
    card_index("2160849a-184f-4a49-8931-fd021e16f7cb")
}

fn arctic_flats() -> CardIndex {
    card_index("841f0a1d-6f84-45fb-81cc-f2d99fa9e9a2")
}

fn arctic_treeline() -> CardIndex {
    card_index("32fb0096-546c-4633-b6e3-ba3f2d9fa49c")
}

fn bog_wreckage() -> CardIndex {
    card_index("bce32541-3bc4-4553-ace2-784a58f164aa")
}

fn boreal_shelf() -> CardIndex {
    card_index("20b549e9-26db-40a5-8bcc-e8de6c559d32")
}

fn cinder_barrens() -> CardIndex {
    card_index("f155a05f-9f5e-4875-a407-103b85ce30ee")
}

fn coastal_tower() -> CardIndex {
    card_index("f0367926-4380-4d22-8cb8-46c2076f102a")
}

fn contaminated_aquifer() -> CardIndex {
    card_index("c27b771d-b5ec-459a-a101-f078cb8d0184")
}

fn crumbling_necropolis() -> CardIndex {
    card_index("7190debf-708b-4f41-9714-0d0a5bd5a74e")
}

fn dwarven_ruins() -> CardIndex {
    card_index("cbef6ad5-717f-4b02-9c62-e5deba407ad1")
}

fn ebon_stronghold() -> CardIndex {
    card_index("4d4b9512-d29d-4b99-9249-d37e1199a1f5")
}

fn elfhame_palace() -> CardIndex {
    card_index("cade8b94-2998-4d23-87bb-9fbdddd19dea")
}

fn forsaken_sanctuary() -> CardIndex {
    card_index("941b0dd1-0df2-48ee-8829-615e9c3177a7")
}

fn foul_orchard() -> CardIndex {
    card_index("ad6a2776-801f-4743-8268-6d654122171e")
}

fn frontier_bivouac() -> CardIndex {
    card_index("e4cf6c2f-0f1e-4980-9ef9-e4eabcae42a9")
}

fn frost_marsh() -> CardIndex {
    card_index("75a49bce-ab48-44d6-906d-6f51a30702ba")
}

fn geothermal_bog() -> CardIndex {
    card_index("e3b67368-1dd6-419b-a95d-7131b1dba23f")
}

fn geothermal_crevice() -> CardIndex {
    card_index("83f9d84a-a011-4fa9-a3a4-51eb39159c54")
}

fn glacial_floodplain() -> CardIndex {
    card_index("5d3563dd-a2c1-463c-a0ed-5ac22388bdbe")
}

fn haunted_mire() -> CardIndex {
    card_index("b0b58a03-462c-4964-97c7-42bc777ec23e")
}

fn havenwood_battleground() -> CardIndex {
    card_index("18cbb47a-85b1-48f6-a024-8c3bbffa0d87")
}

fn highland_forest() -> CardIndex {
    card_index("35137378-6754-4bb1-a38e-5940890ccab1")
}

fn highland_lake() -> CardIndex {
    card_index("c643365a-4255-4d23-adb9-0f8b456f0838")
}

fn highland_weald() -> CardIndex {
    card_index("66d845a2-be4c-41bb-a357-3650461c03ed")
}

fn ice_tunnel() -> CardIndex {
    card_index("40c5d6fe-854a-436d-9f80-13eb5f1f8f68")
}

fn idyllic_beachfront() -> CardIndex {
    card_index("0aea7e5d-40d7-46c7-a8a6-479cb8061e49")
}

fn irrigation_ditch() -> CardIndex {
    card_index("e0c3c87b-83e3-4cff-b05f-78a192e2bba4")
}

fn jungle_shrine() -> CardIndex {
    card_index("2e69537c-c898-4e13-a72d-ce3957a90304")
}

fn meandering_river() -> CardIndex {
    card_index("2d9663be-c466-4191-85e2-a69ce0965432")
}

fn molten_tributary() -> CardIndex {
    card_index("58c592ed-20fc-481b-909b-2315567e5f20")
}

fn mystic_monastery() -> CardIndex {
    card_index("834b8f71-9a45-42ae-9e99-e749fa6fb45e")
}

fn nomad_outpost() -> CardIndex {
    card_index("4619de7e-3d6e-4c6b-8e6e-e24db324839d")
}

fn opulent_palace() -> CardIndex {
    card_index("f9e7e855-1e3b-42d3-91b0-64ba8b5b8982")
}

fn radiant_grove() -> CardIndex {
    card_index("32c91719-f3dd-4cc7-9e32-7d5ccf18f07c")
}

fn ravaged_highlands() -> CardIndex {
    card_index("68919931-b995-4aa0-9d2a-e80eae2f335f")
}

fn rimewood_falls() -> CardIndex {
    card_index("983739cd-0b36-40d9-9a03-7b6aa7ffd0df")
}

fn ruins_of_trokair() -> CardIndex {
    card_index("ad7b610c-e276-4c71-9b53-1ccb6d2dafd2")
}

fn sacred_peaks() -> CardIndex {
    card_index("fb69bc57-f05a-41c2-9b7b-9a9761ef0cd3")
}

fn salt_marsh() -> CardIndex {
    card_index("6a40823f-130c-4416-a987-75e205d3e1dd")
}

fn sandsteppe_citadel() -> CardIndex {
    card_index("544dbabd-cbfc-40da-a5ba-2fea9cddb453")
}

fn savage_lands() -> CardIndex {
    card_index("a3292406-3f49-42d6-a547-e43dd5797f84")
}

fn seafloor_debris() -> CardIndex {
    card_index("eef64810-4187-40c1-8e59-1cb8773ebea3")
}

fn seaside_citadel() -> CardIndex {
    card_index("2ae77795-6a80-498b-bf69-6fd612f601e4")
}

fn shivan_oasis() -> CardIndex {
    card_index("9b64c0f4-b917-4a0d-b033-30ab71aab807")
}

fn snowfield_sinkhole() -> CardIndex {
    card_index("749c2c8e-9588-4e83-b07f-3c37eb63338b")
}

fn stone_quarry() -> CardIndex {
    card_index("90bccf66-58ec-445d-ac96-c6013054a1b4")
}

/// Submerged Boneyard prints two lines: "This land enters tapped" and
/// "{T}: Add {U} or {B}." Both are read off one land played as a real land
/// drop, because a permanent seeded into `starting_battlefield` is placed
/// rather than enters and arrives untapped whatever the card says. The tapped
/// arrival is asserted in the turn it happens — where a `{T}` with nothing to
/// pay it with is therefore not even offered — and the mana line a turn later,
/// after an untap step has stood the land back up. "Or" is the question the
/// ability asks, so the choice is read as exactly the two colours the card
/// prints, with colourless left out (CR 105.4).
fn submerged_boneyard() -> CardIndex {
    card_index("f27d52e6-aab9-4f95-ae46-33d1173bf4fe")
}

fn sulfur_vent() -> CardIndex {
    card_index("bc2906cd-9e0c-4aa8-b656-fb1045d810ac")
}

fn sulfurous_mire() -> CardIndex {
    card_index("77cf536e-246e-4c18-8e04-c904fcad3f40")
}

fn sunlit_marsh() -> CardIndex {
    card_index("a5e5a259-5fa7-4b01-93cb-a2b4aaf80927")
}

fn svyelunite_temple() -> CardIndex {
    card_index("5578cf33-62f5-456c-a58c-f744a25df79b")
}

fn tangled_islet() -> CardIndex {
    card_index("4b1f68a2-b606-4c64-bd44-a9714808316d")
}

fn timber_gorge() -> CardIndex {
    card_index("00b34fab-5a80-4a4d-b6cf-72479197677a")
}

fn timberland_ruins() -> CardIndex {
    card_index("29c7f059-2eeb-40f9-8f50-03eba2d0d5e0")
}

fn tinder_farm() -> CardIndex {
    card_index("af0ad159-8264-4526-9a95-eddd32c0a13f")
}

fn tranquil_expanse() -> CardIndex {
    card_index("a5478263-47c9-447f-9c7a-c77ce0752947")
}

fn tresserhorn_sinks() -> CardIndex {
    card_index("74a6597c-13ea-47bd-adb3-45671897b11b")
}

fn urborg_volcano() -> CardIndex {
    card_index("018129dd-f578-4fc9-b6f8-b13ed0c75a8d")
}

fn volatile_fjord() -> CardIndex {
    card_index("96262c15-9130-409c-8242-71f84be719b7")
}

// ---- Abilities no test had fired (L4 sweep, 2026-10-01) ----

fn unf_tap(engine: &mut Engine<RegistryLookup>, seat: PlayerId, id: ObjectId) {
    engine
        .dev_state_mut(seat)
        .expect("the harness may set boards up")
        .object_mut(id)
        .expect("on the table")
        .status
        .insert(Status::TAPPED);
    engine.refresh_offer();
}

/// Aims the ability at `target` when asked, then pays any sacrifice with
/// `sacrifice`, and lets the stack empty.
#[track_caller]
fn unf_aim_and_pay(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    target: Option<ObjectId>,
    sacrifice: Option<ObjectId>,
) {
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::ChooseTargets { options, .. } => {
                let t = target.expect("a target was asked for");
                assert!(options.contains(&t), "the target is on offer");
                engine
                    .apply(seat, PlayerAction::ChooseObjects { objects: vec![t] })
                    .unwrap();
            }
            Pending::ChooseCards {
                prompt: ChoicePrompt::CostSacrifice,
                options,
                ..
            } => {
                let s = sacrifice.expect("a sacrifice was asked for");
                assert!(options.contains(&s), "the sacrifice is on offer");
                engine
                    .apply(seat, PlayerAction::ChooseObjects { objects: vec![s] })
                    .unwrap();
            }
            _ => break,
        }
    }
    pass_until(engine, stack_is_empty);
}

fn oasis() -> CardIndex {
    card_index("4533ce78-0594-4195-96fb-46cbadd0db69")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

// ---- Abilities no test had fired, second sweep (L4, 2026-10-01) ----

/// Pays the animation (ability 1) of a seeded manland out of the lands
/// beside it, then passes to the declare-attackers question and declares
/// the land plus `with` as attackers at seat 1. Returns the land's object;
/// the engine stands where the attack triggers go on the stack.
#[track_caller]
fn animate_and_attack(
    engine: &mut Engine<RegistryLookup>,
    land_card: CardIndex,
    with: &[ObjectId],
) -> ObjectId {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    reach_main_phase(engine, p0);
    let land = on_battlefield(engine, p0, land_card).expect("the manland is out");
    tap_all_mana_but(engine, p0, Some(land_card));
    activate(engine, p0, land_card, 1);
    pass_until(engine, stack_is_empty);
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on the attack declaration")
    };
    assert!(attackers.contains(&land), "the animated land may attack");
    let mut declared = vec![(land, Defender::Player(p1))];
    declared.extend(with.iter().map(|o| (*o, Defender::Player(p1))));
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: declared,
            },
        )
        .expect("the attackers came from the offer");
    land
}

/// Answers the attack trigger's target question with `object`.
#[track_caller]
fn aim_trigger_at(engine: &mut Engine<RegistryLookup>, object: ObjectId) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on the target question")
    };
    assert!(
        options.contains(&object),
        "the target is on offer: {options:?}"
    );
    engine
        .apply(
            PlayerId::new(0),
            PlayerAction::ChooseObjects {
                objects: vec![object],
            },
        )
        .expect("the target came from the offer");
}

fn restless_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn darksteel_gargoyle() -> CardIndex {
    card_index("73010421-374f-458e-aa88-248ef8ae4f8b")
}

fn lair_ornithopter() -> CardIndex {
    card_index("a3a98bc9-caa0-49b7-951c-fe4e4f54e4ba")
}

fn lair_tortoise() -> CardIndex {
    card_index("2dd50d7f-941f-4deb-a15c-ee2357844c35")
}

/// Answers the question for an activated ability's target with `object`,
/// and asserts that every id in `refused` was not on offer.
#[track_caller]
fn answer_target(engine: &mut Engine<RegistryLookup>, object: ObjectId, refused: &[ObjectId]) {
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("a target question, got {:?}", engine.pending())
    };
    assert!(options.contains(&object), "on offer: {options:?}");
    for r in refused {
        assert!(!options.contains(r), "not a legal target: {options:?}");
    }
    engine
        .apply(
            PlayerId::new(0),
            PlayerAction::ChooseObjects {
                objects: vec![object],
            },
        )
        .expect("the target came from the offer");
}
