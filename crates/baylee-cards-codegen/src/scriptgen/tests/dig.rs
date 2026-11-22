//! Unit tests for reading `Dig`: each shape the reader writes, on made-up
//! scripts, and each shape it refuses with the reason it names.

use super::*;

fn ability(script: &str) -> String {
    let body = read(script);
    assert_eq!(body.abilities.len(), 1, "{:?}", body.abilities);
    body.abilities[0].clone()
}

fn reason(script: &str) -> String {
    let parsed = parse(script);
    assert!(
        transcode(&parsed, &cats(), None).is_none(),
        "silently accepted: {script}"
    );
    refusal_reason(&parsed, &cats(), None).unwrap_or_default()
}

#[test]
fn a_pick_into_the_hand_puts_the_rest_on_the_bottom() {
    // Sleight of Hand: the reference's defaults are one card, to the hand,
    // the rest on the bottom in the player's order.
    assert_eq!(
        ability(
            "Name:Pick\nManaCost:U\nTypes:Sorcery\n\
             A:SP$ Dig | DigNum$ 2 | NoReveal$ True"
        ),
        "spell!(&[Effect::LookAtTopPick { count: Amount::Fixed(2), pick: 1, random: false }])"
    );
    assert!(
        ability(
            "Name:Pick two\nManaCost:2 U\nTypes:Sorcery\n\
             A:SP$ Dig | DigNum$ 7 | ChangeNum$ 2 | RestRandomOrder$ True"
        )
        .contains("Effect::LookAtTopPick { count: Amount::Fixed(7), pick: 2, random: true }")
    );
    // Worldly Counsel: the domain count, read as it resolves.
    assert!(
        ability(
            "Name:Domain pick\nManaCost:1 U\nTypes:Instant\n\
             A:SP$ Dig | DigNum$ X\nSVar:X:Count$Domain"
        )
        .contains("count: Amount::BasicLandTypesAmong(&Filter::YOUR_LAND), pick: 1")
    );
    assert!(
        ability(
            "Name:X pick\nManaCost:X U\nTypes:Instant\n\
             A:SP$ Dig | DigNum$ X\nSVar:X:Count$xPaid"
        )
        .contains("count: Amount::X, pick: 1")
    );
    // Diabolic Vision: the rest back on top, in the player's order.
    assert!(
        ability(
            "Name:Pick and reorder\nManaCost:U B\nTypes:Sorcery\n\
             A:SP$ Dig | DigNum$ 5 | ChangeNum$ 1 | LibraryPosition2$ 0"
        )
        .contains("Effect::LookAtTopPickThenReorder { count: Amount::Fixed(5), pick: 1 }")
    );
    // An enters trigger looks at its controller's library too.
    assert!(
        ability(
            "Name:Familiar\nManaCost:2 U\nTypes:Creature Wizard\nPT:1/2\n\
             T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Any | Destination$ Battlefield | Execute$ FamiliarLook\n\
             SVar:FamiliarLook:DB$ Dig | DigNum$ 3"
        )
        .contains("Effect::LookAtTopPick { count: Amount::Fixed(3), pick: 1, random: false }")
    );
}

#[test]
fn a_dig_after_a_targeted_line_still_digs_in_your_library() {
    // Prophetic Bolt: the damage line's target belongs to the damage, and
    // the dig after it is "look at the top four cards of your library".
    let line = ability(
        "Name:Bolt and pick\nManaCost:3 U R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SubAbility$ LookThree\n\
         SVar:LookThree:DB$ Dig | DigNum$ 3",
    );
    assert!(
        line.contains(
            "Effect::DealDamage { amount: Amount::Fixed(3), target: TargetSpec::AnyTarget }, \
             Effect::LookAtTopPick { count: Amount::Fixed(3), pick: 1, random: false }"
        ),
        "{line}"
    );
    assert!(line.contains("targets = Some(TargetReq::one(TargetSpec::AnyTarget))"));
}

#[test]
fn a_look_may_take_one_matching_card() {
    let line = ability(
        "Name:Look for a creature\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 4 | ChangeNum$ 1 | ChangeValid$ Creature | ForceRevealToController$ True | Optional$ True | RestRandomOrder$ True",
    );
    assert!(
        line.contains(
            "Effect::LookAtTopFind { found: Find::HAND, count: 4, filter: &Filter::CREATURE, random: true }"
        ),
        "{line}"
    );
    let line = ability(
        "Name:Look for a land\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 6 | ChangeValid$ Land | Optional$ True | DestinationZone$ Battlefield | Tapped$ True",
    );
    assert!(
        line.contains(
            "Effect::LookAtTopFind { found: Find::BATTLEFIELD_TAPPED, count: 6, filter: &Filter::LAND, random: false }"
        ),
        "{line}"
    );
}

#[test]
fn a_reveal_moves_every_matching_card() {
    // Goblin Ringleader and the tribal enters triggers.
    let body = read(
        "Name:Ringleader\nManaCost:3 R\nTypes:Creature Goblin\nPT:2/2\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Any | Destination$ Battlefield | Execute$ RevealFive\n\
         SVar:RevealFive:DB$ Dig | DigNum$ 5 | Reveal$ True | ChangeNum$ All | ChangeValid$ Goblin",
    );
    assert!(
        body.abilities[0].contains("Effect::RevealTopAllMatching { count: 5, filter: &"),
        "{:?}",
        body.abilities
    );
    assert!(body.abilities[0].contains("found: Find::HAND"));
    assert!(
        body.statics.contains("subtypes::creature::GOBLIN"),
        "{}",
        body.statics
    );
}

#[test]
fn a_revealed_top_card_is_sorted_by_the_filter() {
    // Zoologist and Call of the Wild: one card, here if it matches and
    // there if it does not.
    assert!(
        ability(
            "Name:Sort\nManaCost:3 G\nTypes:Creature Wizard\nPT:2/3\n\
             A:AB$ Dig | Cost$ 3 G T | DigNum$ 1 | Reveal$ True | ChangeNum$ All | ChangeValid$ Creature | DestinationZone$ Battlefield | DestinationZone2$ Graveyard"
        )
        .contains(
            "Effect::RevealTopAndSort { filter: &Filter::CREATURE, matched: SearchDest::Battlefield, otherwise: SearchDest::Graveyard }"
        )
    );
}

#[test]
fn a_reveal_find_reads_one_card_whether_or_not_the_line_says_one() {
    let unsaid = ability(
        "Name:Find\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 4 | Reveal$ True | ChangeValid$ Creature | DestinationZone2$ Graveyard | Optional$ True",
    );
    let said = ability(
        "Name:Find\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 4 | Reveal$ True | ChangeNum$ 1 | ChangeValid$ Creature | DestinationZone2$ Graveyard | Optional$ True",
    );
    assert_eq!(unsaid, said);
    assert!(said.contains(
        "Effect::RevealTopFind { found: Find::HAND, count: 4, filter: &Filter::CREATURE, otherwise: SearchDest::Graveyard }"
    ));
}

#[test]
fn every_card_exiled_is_the_top_of_a_library_exiled() {
    assert!(
        ability(
            "Name:Exile yours\nManaCost:B\nTypes:Sorcery\n\
             A:SP$ Dig | Defined$ You | DigNum$ 10 | ChangeNum$ All | DestinationZone$ Exile"
        )
        .contains("Effect::ExileTop { who: PlayerRel::You, count: Amount::Fixed(10) }")
    );
    // Raven Guild Master: the player the damage was dealt to.
    assert!(
        ability(
            "Name:Exile theirs\nManaCost:1 U U\nTypes:Creature Wizard\nPT:1/1\n\
             T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | Execute$ ExileSeven | TriggerZones$ Battlefield\n\
             SVar:ExileSeven:DB$ Dig | Defined$ TriggeredTarget | DigNum$ 7 | ChangeNum$ All | DestinationZone$ Exile"
        )
        .contains("Effect::ExileTop { who: PlayerRel::DamagedPlayer, count: Amount::Fixed(7) }")
    );
    let line = ability(
        "Name:Exile target's\nManaCost:U\nTypes:Sorcery\n\
         A:SP$ Dig | ValidTgts$ Player | DigNum$ 3 | ChangeNum$ All | DestinationZone$ Exile",
    );
    assert!(
        line.contains("Effect::ExileTop { who: PlayerRel::Chosen, count: Amount::Fixed(3) }"),
        "{line}"
    );
}

const REFUSED_DIG_SHAPES: &[(&str, &str)] = &[
    ("DigNum$ 3 | Reveal$ True", "a revealed `Dig` pick"),
    ("DigNum$ 3 | Optional$ True", "an optional `Dig` pick"),
    (
        "DigNum$ 3 | ValidTgts$ Player",
        "a `Dig` of a targeted player's library",
    ),
    (
        "DigNum$ 5 | Defined$ Player | Reveal$ True | ChangeNum$ All | ChangeValid$ Land | DestinationZone$ Battlefield",
        "a `Dig` of every player's library",
    ),
    (
        "DigNum$ 4 | ChangeValid$ Creature | NoReveal$ True | Optional$ True",
        "a `Dig` that moves a matching card unrevealed",
    ),
    (
        "DigNum$ 4 | ChangeValid$ Creature",
        "a `Dig` that looks and moves 1 matching to the hand, the rest on the bottom",
    ),
    (
        "DigNum$ 3 | ChangeNum$ All | ChangeValid$ Land | DestinationZone$ Exile",
        "a `Dig` that exiles some of the cards",
    ),
    (
        "DigNum$ 3 | DestinationZone$ Exile",
        "a `Dig` that exiles the cards it picks",
    ),
    (
        "DigNum$ 2 | ChangeNum$ 1 | DestinationZone$ Graveyard",
        "a `Dig` destination `Graveyard`",
    ),
    (
        "DigNum$ 4 | ChangeValid$ Creature | Optional$ True | Tapped$ True",
        "`Tapped$` on a `Dig` to the hand or exile",
    ),
    (
        "DigNum$ 3 | DestinationZone$ Library | LibraryPosition$ 0 | DestinationZone2$ Exile",
        "a `Dig` destination `Library`",
    ),
    (
        "DigNum$ 4 | Reveal$ True | ForceRevealToController$ True | ChangeValid$ Creature | Optional$ True",
        "a `Dig` revealed to its controller twice",
    ),
    (
        "DigNum$ 2 | ChangeNum$ X",
        "a `Dig` moving `ChangeNum$ X` cards",
    ),
    (
        "DigNum$ 2 | ChangeNum$ 0",
        "a `Dig` moving `ChangeNum$ 0` cards",
    ),
    (
        "DigNum$ 2 | Optional$ False",
        "`Optional$ False` on a `Dig`",
    ),
    ("DigNum$ 2 | Reveal$ False", "`Reveal$ False` on a `Dig`"),
    ("ChangeNum$ 1", "a `Dig` with no `DigNum$`"),
    // "Put the rest back on top in a random order" has no effect here.
    (
        "DigNum$ 3 | LibraryPosition2$ 0 | RestRandomOrder$ True",
        "a `Dig` whose rest goes back on top in a random order",
    ),
    (
        "DigNum$ 4 | ChangeValid$ Creature | Optional$ True | LibraryPosition2$ 0 | RestRandomOrder$ True",
        "a `Dig` that looks and moves up to 1 matching to the hand, the rest on top in a random order",
    ),
    (
        "DigNum$ 3 | DestinationZone2$ Hand",
        "a `Dig` remainder put into `Hand` at position `-1`",
    ),
    // An exile on a line whose target is an object names no library.
    (
        "DigNum$ 3 | ValidTgts$ Creature | ChangeNum$ All | DestinationZone$ Exile",
        "a `Dig` exile on a line that targets an object",
    ),
];

#[test]
fn every_other_dig_is_refused_by_name() {
    let script =
        |params: &str| format!("Name:Dig\nManaCost:U\nTypes:Sorcery\nA:SP$ Dig | {params}");
    for &(params, expected) in REFUSED_DIG_SHAPES {
        let text = script(params);
        assert!(
            reason(&text).contains(expected),
            "{params}: expected `{expected}`, got `{}`",
            reason(&text)
        );
    }
    // Refused by a knob nothing reads.
    for params in [
        "DigNum$ 5 | Reveal$ True | ChangeNum$ 1 | ChangeValid$ Creature | DestinationZone$ Battlefield | DestinationZone2$ Graveyard | Choser$ Player.Opponent",
        "DigNum$ 3 | Defined$ You | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True",
        "DigNum$ 3 | ChangeNum$ 1 | SkipReorder$ True | LibraryPosition2$ 0",
    ] {
        assert!(refused(&script(params)), "silently accepted: {params}");
    }
}

#[test]
fn the_shapes_no_card_above_spells_out() {
    // Every matching card onto the battlefield, tapped or not.
    let line = ability(
        "Name:Lands down\nManaCost:3 G\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 5 | Reveal$ True | ChangeNum$ All | ChangeValid$ Land | DestinationZone$ Battlefield | Tapped$ True",
    );
    assert!(
        line.contains(
            "Effect::RevealTopAllMatching { count: 5, filter: &Filter::LAND, found: Find::BATTLEFIELD_TAPPED }"
        ),
        "{line}"
    );
    let line = ability(
        "Name:Creatures down\nManaCost:5 G\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 4 | Reveal$ True | ChangeNum$ All | ChangeValid$ Creature | DestinationZone$ Battlefield",
    );
    assert!(line.contains("found: Find::BATTLEFIELD }"), "{line}");
    // A revealed find whose rest is exiled.
    let line = ability(
        "Name:Find and exile\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 3 | Reveal$ True | ChangeValid$ Instant | Optional$ True | DestinationZone2$ Exile",
    );
    assert!(
        line.contains(
            "Effect::RevealTopFind { found: Find::HAND, count: 3, filter: &DIG1, otherwise: SearchDest::Exile }"
        ),
        "{line}"
    );
    // Each player's library, and each opponent's.
    for (defined, who) in [
        ("Player", "PlayerRel::EachPlayer"),
        ("Opponent", "PlayerRel::Opponent"),
    ] {
        let line = ability(&format!(
            "Name:Exile each\nManaCost:2 B\nTypes:Sorcery\n\
             A:SP$ Dig | Defined$ {defined} | DigNum$ 2 | ChangeNum$ All | DestinationZone$ Exile"
        ));
        assert!(
            line.contains(&format!(
                "Effect::ExileTop {{ who: {who}, count: Amount::Fixed(2) }}"
            )),
            "{defined}: {line}"
        );
    }
}

#[test]
fn wave18b_dig_library_position_and_dig_until_subability() {
    // Redundant `LibraryPosition$ -1` alongside `DestinationZone$ Hand/Battlefield`
    // and bottom remainder (the Forge default) lowers cleanly.
    let hand_with_pos = ability(
        "Name:LookWithPos\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 3 | ChangeNum$ 1 | Optional$ True | ChangeValid$ Permanent | \
         DestinationZone$ Hand | DestinationZone2$ Library | LibraryPosition$ -1 | RestRandomOrder$ True",
    );
    assert!(
        hand_with_pos.contains("Effect::LookAtTopFind { found: Find::HAND, count: 3,"),
        "{hand_with_pos}"
    );

    // `DestinationZone$ Library | LibraryPosition$ 0` + `ChangeValid$` -> `LookAtTopFind { found: Find::TOP_OF_LIBRARY }`.
    let top_find = ability(
        "Name:AtlasLook\nManaCost:2\nTypes:Artifact\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDig\n\
         SVar:TrigDig:DB$ Dig | DigNum$ 4 | Optional$ True | ChangeNum$ 1 | ChangeValid$ Land | \
         DestinationZone$ Library | LibraryPosition$ 0 | RestRandomOrder$ True",
    );
    assert!(
        top_find.contains(
            "Effect::LookAtTopFind { found: Find::TOP_OF_LIBRARY, count: 4, filter: &Filter::LAND, random: true }"
        ),
        "{top_find}"
    );

    // `DestinationZone$ Library | LibraryPosition$ 0` + `DigNum$ 1` -> `LookAtTopOf`.
    let peek_top = ability(
        "Name:SpyIsle\nTypes:Land\n\
         A:AB$ Dig | Cost$ U T | ValidTgts$ Player | DigNum$ 1 | DestinationZone$ Library | LibraryPosition$ 0",
    );
    assert!(
        peek_top.contains("Effect::LookAtTopOf { player: PlayerRel::Chosen, count: 1 }"),
        "{peek_top}"
    );

    // Refusals for invalid `Dig.LibraryPosition`.
    for bad in [
        "Name:BadPosZeroHand\nManaCost:U\nTypes:Sorcery\nA:SP$ Dig | DigNum$ 3 | DestinationZone$ Hand | LibraryPosition$ 0",
        "Name:BadPosNegGy\nManaCost:U\nTypes:Sorcery\nA:SP$ Dig | DigNum$ 3 | DestinationZone$ Hand | DestinationZone2$ Graveyard | LibraryPosition$ -1",
        "Name:BadPosNegLib\nManaCost:U\nTypes:Sorcery\nA:SP$ Dig | DigNum$ 1 | DestinationZone$ Library | LibraryPosition$ -1 | Optional$ True | LibraryPosition2$ 0",
        "Name:BadPeekTwo\nManaCost:U\nTypes:Sorcery\nA:SP$ Dig | DigNum$ 2 | DestinationZone$ Library | LibraryPosition$ 0",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }

    // `DigUntil` with `Card.NamedCard` + `SubAbility$ DBCleanup` (`ClearNamedCard$ True`) -> `RevealUntilNamed`.
    let named_dig = ability(
        "Name:NameSeeker\nManaCost:1 B\nTypes:Creature Wizard\nPT:1/1\n\
         A:AB$ NameCard | Cost$ 1 B T | Defined$ You | SubAbility$ DBDig\n\
         SVar:DBDig:DB$ DigUntil | Valid$ Card.NamedCard | FoundDestination$ Hand | RevealedDestination$ Exile | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearNamedCard$ True",
    );
    assert!(
        named_dig.contains("Effect::ChooseCardName")
            && named_dig.contains(
                "Effect::RevealUntilNamed { player: PlayerRel::You, matched: SearchDest::Hand, otherwise: SearchDest::Exile }"
            ),
        "{named_dig}"
    );

    // Goblin Charbelcher shape: `DigUntil | Valid$ Land | RememberRevealed$ True | SubAbility$ DBBelch` -> `RevealUntilLandDealDamage`.
    let belcher = ability(
        "Name:LandCannon\nManaCost:4\nTypes:Artifact\n\
         A:AB$ DigUntil | Cost$ 3 T | Valid$ Land | ValidDescription$ land | RememberRevealed$ True | \
         RevealedDestination$ Library | RevealedLibraryPosition$ -1 | SubAbility$ DBBelch\n\
         SVar:DBBelch:DB$ DealDamage | ValidTgts$ Any | NumDmg$ X | ConditionCheckSVar$ Mtn | ConditionSVarCompare$ EQ0 | SubAbility$ DBDouble\n\
         SVar:DBDouble:DB$ DealDamage | Defined$ Targeted | NumDmg$ DoubleX | ConditionCheckSVar$ Mtn | ConditionSVarCompare$ EQ1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Valid Card.nonLand\n\
         SVar:DoubleX:Remembered$Valid Card.nonLand/Times.2\n\
         SVar:Mtn:Remembered$Valid Card.Mountain",
    );
    assert!(
        belcher.contains("Effect::RevealUntilLandDealDamage { target: TargetSpec::AnyTarget }")
            && belcher.contains("target = Some(TargetSpec::AnyTarget)"),
        "{belcher}"
    );

    // Refusal: `DigUntil` with `RememberFound$ True` -> `Play` with dynamic `PlayCost$`.
    assert!(refused(
        "Name:BadDigPlay\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ DigUntil | Valid$ Card.nonLand | FoundDestination$ Exile | RevealedDestination$ Exile | RememberFound$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | PlayCost$ PayLife<ConvertedManaCost>"
    ));
}

#[test]
fn wave19b_dig_peek_rest_any_and_face_down_exile() {
    // Unfiltered Dig with rest to graveyard or exile -> PeekAndReveal.
    let rest_gy = ability(
        "Name:VaultSift\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 7 | ChangeNum$ 2 | DestinationZone2$ Graveyard",
    );
    assert!(
        rest_gy.contains(
            "Effect::PeekAndReveal { library: PlayerRel::You, count: Amount::Fixed(7), \
             reveal_all: false, reveal_chosen: false, filter: &Filter::Any, min_pick: 2, \
             max_pick: 2, picked: PeekPickDest::Hand, rest: PeekRestDest::Graveyard, if_picked: &[] }"
        ),
        "{rest_gy}"
    );
    let rest_exile = ability(
        "Name:VaultBanish\nManaCost:2 U U\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 5 | DestinationZone2$ Exile",
    );
    assert!(
        rest_exile.contains(
            "min_pick: 1, max_pick: 1, picked: PeekPickDest::Hand, rest: PeekRestDest::Exile"
        ),
        "{rest_exile}"
    );

    // Filtered Dig: ChangeNum$ All + Graveyard rest, multi-pick optional, ChangeNum$ Any + X + cmcLEX.
    let all_gy = ability(
        "Name:PathSift\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 4 | Reveal$ True | ChangeNum$ All | ChangeValid$ Land | DestinationZone2$ Graveyard",
    );
    assert!(
        all_gy.contains(
            "Effect::PeekAndReveal { library: PlayerRel::You, count: Amount::Fixed(4), \
             reveal_all: true, reveal_chosen: false, filter: &Filter::LAND, min_pick: u8::MAX, \
             max_pick: u8::MAX, picked: PeekPickDest::Hand, rest: PeekRestDest::Graveyard, if_picked: &[] }"
        ),
        "{all_gy}"
    );
    let two_spells = ability(
        "Name:ArcanePuzzle\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 5 | Reveal$ True | ChangeNum$ 2 | ChangeValid$ Instant,Sorcery | DestinationZone2$ Graveyard | Optional$ True",
    );
    assert!(
        two_spells.contains("reveal_all: true, reveal_chosen: false")
            && two_spells.contains("min_pick: 0, max_pick: 2, picked: PeekPickDest::Hand, rest: PeekRestDest::Graveyard"),
        "{two_spells}"
    );
    let any_x = ability(
        "Name:VowOfGrove\nManaCost:X G G\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ X | Reveal$ True | ChangeNum$ Any | ChangeValid$ Land,Permanent.Legendary+cmcLEX | DestinationZone$ Battlefield | DestinationZone2$ Graveyard\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        any_x.contains("count: Amount::X, reveal_all: true, reveal_chosen: false")
            && any_x.contains("min_pick: 0, max_pick: u8::MAX, picked: PeekPickDest::Battlefield { tapped: false }, rest: PeekRestDest::Graveyard"),
        "{any_x}"
    );

    // Targeted player `ChangeNum$ Any` top peek -> Graveyard / Exile, rest back on top in any order.
    let dimir_spy = ability(
        "Name:MindMachinations\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ Dig | ValidTgts$ Player | DigNum$ 3 | ChangeNum$ Any | DestinationZone$ Exile | LibraryPosition2$ 0",
    );
    assert!(
        dimir_spy.contains(
            "Effect::PeekAndReveal { library: PlayerRel::Chosen, count: Amount::Fixed(3), \
             reveal_all: false, reveal_chosen: false, filter: &Filter::Any, min_pick: 0, \
             max_pick: u8::MAX, picked: PeekPickDest::Exile, rest: PeekRestDest::TopAnyOrder, if_picked: &[] }"
        ),
        "{dimir_spy}"
    );

    // `Dig.ExileFaceDown`: permanent `ExileTopFaceDownLinked` & `LookAtTopExileFaceDown`.
    let safe = ability(
        "Name:IronVault\nManaCost:3\nTypes:Artifact\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ Dig | Defined$ You | DigNum$ 2 | ChangeNum$ All | DestinationZone$ Exile | ExileFaceDown$ True | NoReveal$ True",
    );
    assert!(
        safe.contains("Effect::ExileTopFaceDownLinked, Effect::ExileTopFaceDownLinked"),
        "{safe}"
    );

    let scout_pw = ability(
        "Name:WildScout\nManaCost:2 G\nTypes:Legendary Planeswalker Scout\nLoyalty:4\n\
         A:AB$ Dig | Cost$ SubCounter<2/LOYALTY> | Planeswalker$ True | Defined$ You | DigNum$ 3 | \
         DestinationZone$ Exile | ExileFaceDown$ True | WithMayLook$ True | RememberChanged$ True | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | RememberObjects$ Remembered | StaticAbilities$ STPlay | Duration$ Permanent | ForgetOnMoved$ Exile | SubAbility$ DBCleanup\n\
         SVar:STPlay:Mode$ Continuous | MayPlay$ True | Affected$ Creature.IsRemembered | AffectedZone$ Exile | Secondary$ You may look at it and cast it if it's a creature.\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        scout_pw
            .contains("Effect::LookAtTopExileFaceDown { count: 3, cast_types: TypeSet::CREATURE }"),
        "{scout_pw}"
    );

    // Refusals: face-down exile on spell / opponent / without matching static / with look-at Oracle,
    // multi-card `TopOfLibrary`, undefined `DigNum$ X`.
    for bad in [
        "Name:BadSpellFaceDown\nManaCost:U\nTypes:Sorcery\nA:SP$ Dig | Defined$ You | DigNum$ 2 | ChangeNum$ All | DestinationZone$ Exile | ExileFaceDown$ True | NoReveal$ True",
        "Name:BadOppFaceDown\nManaCost:2 B\nTypes:Artifact\nA:AB$ Dig | Cost$ T | ValidTgts$ Opponent | DigNum$ 2 | ChangeNum$ All | DestinationZone$ Exile | ExileFaceDown$ True",
        "Name:BadLookFaceDown\nManaCost:2\nTypes:Artifact\nA:AB$ Dig | Cost$ T | Defined$ You | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile | ExileFaceDown$ True | WithMayLook$ True",
        "Name:BadOracleLookFaceDown\nManaCost:2\nTypes:Artifact\nA:AB$ Dig | Cost$ T | Defined$ You | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile | ExileFaceDown$ True\nOracle:Look at the top card of your library, then exile it face down.",
        "Name:BadMultiTop\nManaCost:1 W\nTypes:Sorcery\nA:SP$ Dig | DigNum$ 4 | ChangeNum$ Any | ChangeValid$ Soldier | DestinationZone$ Library | LibraryPosition$ 0",
        "Name:BadUndefX\nManaCost:1 G\nTypes:Sorcery\nA:SP$ Dig | DigNum$ X | Reveal$ True | ChangeValid$ Creature | DestinationZone2$ Graveyard | Optional$ True",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave20b_dig_remember_changed_exiled_and_revealed_mana_value() {
    // Dig | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True -> ThisWay::Exiled.
    let dig_exile_loss = read(
        "Name:ExileTopLoss\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 3 | ChangeNum$ All | DestinationZone$ Exile \
         | RememberChanged$ True | SubAbility$ DBLose\n\
         SVar:DBLose:DB$ LoseLife | Defined$ Opponent | LifeAmount$ X | IsCurse$ True | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Valid Creature",
    );
    assert!(
        dig_exile_loss.abilities[0]
            .contains("Effect::ExileTop { who: PlayerRel::You, count: Amount::Fixed(3) }")
            && dig_exile_loss.abilities[0].contains(
                "Effect::LoseLife { amount: Amount::ThisWay(ThisWay::Exiled(&Filter::CREATURE)), \
                 target: PlayerRel::Opponent }"
            ),
        "{:?}",
        dig_exile_loss.abilities
    );

    // Dig | Reveal$ True | ChangeNum$ All | DestinationZone$ Hand | RememberChanged$ True
    // -> RevealTopAllMatching + ThisWay::RevealedManaValue (Dark Confidant / Yuriko shape).
    let bob = read(
        "Name:BobReveal\nManaCost:1 B\nTypes:Creature Wizard\nPT:2/1\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigDig\n\
         SVar:TrigDig:DB$ Dig | DigNum$ 1 | Reveal$ True | ChangeNum$ All | ChangeValid$ Card \
         | DestinationZone$ Hand | RememberChanged$ True | SubAbility$ DBLose\n\
         SVar:DBLose:DB$ LoseLife | Defined$ Player.Opponent | LifeAmount$ X | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost",
    );
    assert!(
        bob.abilities[0].contains(
            "Effect::RevealTopAllMatching { count: 1, filter: &Filter::Any, found: Find::HAND }"
        ) && bob.abilities[0].contains(
            "Effect::LoseLife { amount: Amount::ThisWay(ThisWay::RevealedManaValue), \
             target: PlayerRel::Opponent }"
        ),
        "{:?}",
        bob.abilities
    );

    // Refusals: Dig exile followed by DB$ Play (until 11b merges) and unrevealed Hand Dig.
    for bad in [
        "Name:BadDigPlay\nManaCost:3 R\nTypes:Sorcery\nA:SP$ Dig | DigNum$ 4 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ P\nSVar:P:DB$ Play | Defined$ Remembered",
        "Name:BadUnrevealedDigHand\nManaCost:1 B\nTypes:Sorcery\nA:SP$ Dig | DigNum$ 1 | ChangeNum$ All | ChangeValid$ Card | DestinationZone$ Hand | RememberChanged$ True | SubAbility$ L\nSVar:L:DB$ LoseLife | LifeAmount$ X\nSVar:X:Remembered$CardManaCost",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave23a_dig_until_standalone_reveal_until_then() {
    // Multi-hit `Amount$ 4` revealing into graveyard (`SameAsRest`).
    let mill_four = ability(
        "Name:GraveDredger\nManaCost:1 U B\nTypes:Sorcery\n\
         A:SP$ DigUntil | ValidTgts$ Opponent | Amount$ 4 | Valid$ Land | \
         RevealedDestination$ Graveyard",
    );
    assert!(
        mill_four.contains(
            "Effect::RevealUntilThen { library: PlayerRel::Chosen, filter: &Filter::LAND, \
             count: Amount::Fixed(4), max_revealed: None, exile_while_revealing: false, \
             found: RevealUntilFound::SameAsRest, rest: RevealUntilRest::Graveyard, then: &[] }"
        ),
        "{mill_four}"
    );

    // Dynamic `Amount$ X`, `RestDestination$` alias, `FoundDestination$ Hand`.
    let dyn_amount = ability(
        "Name:TideSurveyor\nManaCost:3 U\nTypes:Sorcery\n\
         A:SP$ DigUntil | Amount$ X | Valid$ Card.nonLand | FoundDestination$ Hand | \
         RestDestination$ Library | RevealedLibraryPosition$ -1 | RevealRandomOrder$ True\n\
         SVar:X:Count$Valid Creature.YouCtrl",
    );
    assert!(
        dyn_amount.contains(
            "count: Amount::CountOf { filter: &Filter::YOUR_CREATURE, zone: ZoneSel::Battlefield }"
        ) && dyn_amount.contains(
            "found: RevealUntilFound::Hand, rest: RevealUntilRest::BottomRandom, then: &[]"
        ),
        "{dyn_amount}"
    );

    // `MaxRevealed$ X`, `Tapped$ True`, `GainControl$ True` vs library owner's control.
    let steal_tapped = ability(
        "Name:VaultRaider\nManaCost:X 2 B\nTypes:Sorcery\n\
         A:SP$ DigUntil | ValidTgts$ Opponent | MaxRevealed$ X | Valid$ Creature | \
         FoundDestination$ Battlefield | Tapped$ True | GainControl$ True | \
         RevealedDestination$ Graveyard\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        steal_tapped.contains(
            "max_revealed: Some(Amount::X), exile_while_revealing: false, \
             found: RevealUntilFound::Battlefield { tapped: true, under_your_control: true }, \
             rest: RevealUntilRest::Graveyard"
        ),
        "{steal_tapped}"
    );
    let owner_ctrl = ability(
        "Name:SharedOath\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ DigUntil | ValidTgts$ Opponent | Valid$ Creature | \
         FoundDestination$ Battlefield | RevealedDestination$ Library | \
         RevealedLibraryPosition$ -1",
    );
    assert!(
        owner_ctrl.contains(
            "found: RevealUntilFound::Battlefield { tapped: false, under_your_control: false }, \
             rest: RevealUntilRest::BottomChosenOrder"
        ),
        "{owner_ctrl}"
    );
}

#[test]
fn wave23a_dig_until_positions_optional_and_multiseat() {
    // `FoundLibraryPosition$ 0` (top) and `-1` (same as bottom rest).
    let top_found = ability(
        "Name:TopFinder\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ DigUntil | Valid$ Instant | FoundDestination$ Library | \
         FoundLibraryPosition$ 0 | RevealedDestination$ Graveyard",
    );
    assert!(
        top_found
            .contains("found: RevealUntilFound::TopOfLibrary, rest: RevealUntilRest::Graveyard"),
        "{top_found}"
    );
    let bottom_found = ability(
        "Name:BottomFinder\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ DigUntil | Valid$ Instant | FoundDestination$ Library | \
         FoundLibraryPosition$ -1 | RevealedDestination$ Library | \
         RevealedLibraryPosition$ -1 | RevealRandomOrder$ True",
    );
    assert!(
        bottom_found
            .contains("found: RevealUntilFound::SameAsRest, rest: RevealUntilRest::BottomRandom"),
        "{bottom_found}"
    );

    // `RevealOptional$ True` wraps in `Effect::MayDo`.
    let opt_reveal = ability(
        "Name:CoyOracle\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ DigUntil | RevealOptional$ True | Valid$ Land | FoundDestination$ Hand | \
         RevealedDestination$ Library | RevealedLibraryPosition$ -1 | RevealRandomOrder$ True",
    );
    assert!(
        opt_reveal.contains("Effect::MayDo { effects: &[Effect::RevealUntilThen {"),
        "{opt_reveal}"
    );

    // `OptionalFoundMove$ True` with `Shuffle$ True` (non-Aura) -> `StayInLibrary` + `MayDo(MoveAndModify)` + `ShuffleLibrary`.
    let opt_move = ability(
        "Name:ShrineSpirit\nManaCost:3 G\nTypes:Creature Spirit\nPT:3/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDig\n\
         SVar:TrigDig:DB$ DigUntil | Valid$ Artifact | FoundDestination$ Battlefield | \
         OptionalFoundMove$ True | RevealedDestination$ Library | Shuffle$ True",
    );
    assert!(
        opt_move.contains(
            "found: RevealUntilFound::StayInLibrary, rest: RevealUntilRest::StayInLibrary"
        ) && opt_move.contains("Effect::MayDo { effects: &[Effect::MoveAndModify {")
            && opt_move.contains("duration: Duration::Indefinitely")
            && opt_move.contains("Effect::ShuffleLibrary { who: PlayerRel::You }"),
        "{opt_move}"
    );

    // Multi-seat `Defined$ Opponent` wraps single-seat `RevealUntilThen` in `Effect::RepeatEach`.
    let each_opp = ability(
        "Name:GroupMill\nManaCost:2 U B\nTypes:Sorcery\n\
         A:SP$ DigUntil | Defined$ Opponent | Amount$ 2 | Valid$ Land | \
         RevealedDestination$ Graveyard",
    );
    assert!(
        each_opp.contains(
            "Effect::RepeatEach { domain: RepeatDomain::EachOpponent, effects: &[Effect::RevealUntilThen { \
             library: PlayerRel::EventPlayer,"
        ),
        "{each_opp}"
    );
}

#[test]
fn wave23a_dig_until_remember_found_and_revealed_chains() {
    // Hoisted `ValidTgts$` + `DealDamage` equal to found card's mana value.
    let mv_burn = ability(
        "Name:ArcaneSpark\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ DigUntil | Valid$ Card.nonLand | RevealedDestination$ Library | \
         RevealedLibraryPosition$ -1 | RevealRandomOrder$ True | RememberFound$ True | \
         SubAbility$ DBDmg\n\
         SVar:DBDmg:DB$ DealDamage | ValidTgts$ Any | NumDmg$ X | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost",
    );
    assert!(
        mv_burn.contains("targets = Some(TargetReq::one(TargetSpec::AnyTarget))")
            && mv_burn.contains(
                "then: &[Effect::DealDamage { amount: Amount::EventManaValue, \
                 target: TargetSpec::AnyTarget }]"
            ),
        "{mv_burn}"
    );

    // Hoisted `ValidTgts$ Creature` + `+X/-X` (`Amount::Negated(&Amount::EventManaValue)`).
    let mv_mut = ability(
        "Name:WildShift\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ DigUntil | Valid$ Card.nonLand | RevealedDestination$ Library | \
         RevealedLibraryPosition$ -1 | RevealRandomOrder$ True | RememberFound$ True | \
         SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | ValidTgts$ Creature | NumAtt$ +X | NumDef$ -X | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost",
    );
    assert!(
        mv_mut.contains(
            "Effect::PumpTarget { power: Amount::EventManaValue, \
             toughness: Amount::Negated(&Amount::EventManaValue)"
        ),
        "{mv_mut}"
    );

    // Untargeted `Pump | Defined$ Self` uses `PumpTargetSlot { slot: TargetSlot::This, .. }`.
    let self_pump = ability(
        "Name:ChaosTinker\nManaCost:2 R\nTypes:Creature Goblin\nPT:2/2\n\
         A:AB$ DigUntil | Cost$ 2 R | Valid$ Card.nonLand | RevealedDestination$ Library | \
         RevealedLibraryPosition$ -1 | RevealRandomOrder$ True | RememberFound$ True | \
         SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | Defined$ Self | NumAtt$ +X | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost",
    );
    assert!(
        self_pump.contains(
            "Effect::PumpTargetSlot { slot: TargetSlot::This, power: Amount::EventManaValue, \
             toughness: Amount::Fixed(0), keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }"
        ),
        "{self_pump}"
    );

    // `MaxRevealed$ X` + `Sacrifice` gated by `Remembered$Amount == 1` -> `Effect::SacrificeSelf`.
    let dissect = ability(
        "Name:MindProbe\nManaCost:4\nTypes:Artifact\n\
         A:AB$ DigUntil | Cost$ X T | ValidTgts$ Opponent | MaxRevealed$ X | Valid$ Artifact | \
         FoundDestination$ Battlefield | GainControl$ True | RevealedDestination$ Graveyard | \
         RememberFound$ True | SubAbility$ DBSac\n\
         SVar:DBSac:DB$ Sacrifice | ConditionCheckSVar$ Found | ConditionSVarCompare$ EQ1 | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$xPaid\n\
         SVar:Found:Remembered$Amount",
    );
    assert!(
        dissect.contains("max_revealed: Some(Amount::X)")
            && dissect.contains("then: &[Effect::SacrificeSelf]"),
        "{dissect}"
    );

    // `ImmediateTrigger` reading `Remembered$CardManaCost` -> `Effect::ReflexiveTrigger`.
    let blast = ability(
        "Name:FluxVolley\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ DigUntil | Valid$ Card.nonLand | RevealedDestination$ Library | \
         RevealedLibraryPosition$ -1 | RevealRandomOrder$ True | RememberFound$ True | \
         SubAbility$ DBTrig\n\
         SVar:DBTrig:DB$ ImmediateTrigger | ConditionDefined$ Remembered | \
         ConditionPresent$ Card.nonLand | Execute$ TrigDmg | SubAbility$ DBClean\n\
         SVar:TrigDmg:DB$ DealDamage | ValidTgts$ Any | NumDmg$ X\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost",
    );
    assert!(
        blast.contains(
            "Effect::ReflexiveTrigger { when: ReflexiveWhen::Always, once: true, \
             condition: None, targets: Some(TargetReq::one(TargetSpec::AnyTarget)), \
             remember: TriggerRemember::EventObject, \
             effects: &[Effect::DealDamage { amount: Amount::EventManaValue, \
             target: TargetSpec::AnyTarget }] }"
        ),
        "{blast}"
    );
}

#[test]
fn wave23a_dig_until_play_and_exile_permission_chains() {
    // `Play` + Exile->Hand uncast fallback.
    let storm_play = ability(
        "Name:StormRift\nManaCost:4 R\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDig\n\
         SVar:TrigDig:DB$ DigUntil | Valid$ Card.nonLand | FoundDestination$ Exile | \
         RevealedDestination$ Library | RevealedLibraryPosition$ -1 | RevealRandomOrder$ True | \
         RememberFound$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | WithoutManaCost$ True | Optional$ True | \
         ValidSA$ Spell.cmcLE6 | SubAbility$ DBHand\n\
         SVar:DBHand:DB$ ChangeZone | Defined$ Remembered | Origin$ Exile | Destination$ Hand | \
         SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        storm_play.contains("Effect::Play { source: PlaySource::Subject(TargetSpec::EventObject)")
            && storm_play.contains("cost: PlayCostMode::WithoutPayingManaCost, cast_only: true")
            && storm_play.contains("Effect::ReturnToHand { target: TargetSpec::EventObject }"),
        "{storm_play}"
    );

    // `NoMoveRevealed$ True` -> `Play` -> `Shuffle`.
    let shift_play = ability(
        "Name:SpellWeave\nManaCost:3 U\nTypes:Instant\n\
         A:SP$ DigUntil | Valid$ Instant | NoMoveRevealed$ True | RememberFound$ True | \
         SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | WithoutManaCost$ True | Optional$ True | \
         SubAbility$ DBShuf\n\
         SVar:DBShuf:DB$ Shuffle | Defined$ You | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        shift_play.contains(
            "found: RevealUntilFound::StayInLibrary, rest: RevealUntilRest::StayInLibrary"
        ) && shift_play
            .contains("Effect::Play { source: PlaySource::Subject(TargetSpec::EventObject)")
            && shift_play.contains("Effect::ShuffleLibrary { who: PlayerRel::You }"),
        "{shift_play}"
    );

    // `FoundDestination$ Exile` -> `DB$ Effect` (`ThisTurn` on spell, `UntilHostLeavesPlay` on permanent).
    let stolen = ability(
        "Name:LiftedSpoils\nManaCost:3 U\nTypes:Sorcery\n\
         A:SP$ DigUntil | ValidTgts$ Opponent | Valid$ Card.nonLand | FoundDestination$ Exile | \
         RevealedDestination$ Library | RevealedLibraryPosition$ -1 | RevealRandomOrder$ True | \
         RememberFound$ True | SubAbility$ DBEff\n\
         SVar:DBEff:DB$ Effect | RememberObjects$ Remembered | StaticAbilities$ STPlay | \
         ForgetOnMoved$ Exile | SubAbility$ DBClean\n\
         SVar:STPlay:Mode$ Continuous | Affected$ Card.IsRemembered | AffectedZone$ Exile | \
         MayPlay$ True | MayPlayWithoutManaCost$ True\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        stolen.contains("found: RevealUntilFound::ExileLinked")
            && stolen.contains(
                "Effect::GrantLinkedExilePermission { \
                 permission: ExilePermission::play(PlayWindow::ThisTurn).without_paying_mana_cost() }"
            ),
        "{stolen}"
    );

    let host_play = ability(
        "Name:TimelessChronicle\nManaCost:3 R W\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigDig\n\
         SVar:TrigDig:DB$ DigUntil | Defined$ You | Valid$ Card.Legendary | \
         FoundDestination$ Exile | RevealedDestination$ Exile | RememberRevealed$ True | \
         ImprintFound$ True | SubAbility$ DBEff\n\
         SVar:DBEff:DB$ Effect | StaticAbilities$ STPlay | RememberObjects$ Imprinted | \
         Duration$ UntilHostLeavesPlay | ForgetOnMoved$ Exile | SubAbility$ DBRest\n\
         SVar:STPlay:Mode$ Continuous | Affected$ Card.IsRemembered | AffectedZone$ Exile | \
         MayPlay$ True\n\
         SVar:DBRest:DB$ ChangeZoneAll | ChangeType$ Card.IsRemembered+!IsImprinted | \
         Origin$ Exile | Destination$ Library | LibraryPosition$ -1 | RandomOrder$ True | \
         SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True | ClearImprinted$ True",
    );
    assert!(
        host_play.contains("exile_while_revealing: true, found: RevealUntilFound::ExileLinked, rest: RevealUntilRest::BottomRandom")
            && host_play.contains(
                "Effect::GrantLinkedExilePermission { \
                 permission: ExilePermission::play(PlayWindow::WhileSourceOnBattlefield) }"
            ),
        "{host_play}"
    );
}

#[test]
fn wave23a_dig_until_exile_move_revealed_count_and_repeat_composition() {
    // `exile_while_revealing` -> `ChangeZone` Exile->Battlefield + `ChangeZoneAll` rest to bottom.
    let starrix = ability(
        "Name:ExileSurge\nManaCost:4 G\nTypes:Sorcery\n\
         A:SP$ DigUntil | Amount$ 2 | Valid$ Permanent | FoundDestination$ Exile | \
         RevealedDestination$ Exile | RememberRevealed$ True | ImprintFound$ True | \
         SubAbility$ DBPut\n\
         SVar:DBPut:DB$ ChangeZone | Defined$ Imprinted | Origin$ Exile | Destination$ Battlefield | \
         SubAbility$ DBRest\n\
         SVar:DBRest:DB$ ChangeZoneAll | ChangeType$ Card.IsRemembered+!IsImprinted | \
         Origin$ Exile | Destination$ Library | LibraryPosition$ -1 | RandomOrder$ True | \
         SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True | ClearImprinted$ True",
    );
    assert!(
        starrix.contains(
            "exile_while_revealing: true, \
             found: RevealUntilFound::Battlefield { tapped: false, under_your_control: true }, \
             rest: RevealUntilRest::BottomRandom, then: &[]"
        ),
        "{starrix}"
    );

    // `RememberFound$ True` -> unconditional trailing `Clash` (outside `then`).
    let recross = ability(
        "Name:PathClash\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ DigUntil | Valid$ Land | FoundDestination$ Battlefield | \
         RevealedDestination$ Library | RevealedLibraryPosition$ -1 | RevealRandomOrder$ True | \
         RememberFound$ True | SubAbility$ DBClash\n\
         SVar:DBClash:DB$ Clash | WinSubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(recross.contains("then: &[] }, Effect::Clash"), "{recross}");

    // `RememberRevealed$ True` -> `DealDamage` reading `Remembered$Amount` -> `ThisWay::Revealed`.
    let madcap = ability(
        "Name:AetherGamble\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ DigUntil | Valid$ Artifact | FoundDestination$ Battlefield | \
         RevealedDestination$ Library | RevealedLibraryPosition$ -1 | RevealRandomOrder$ True | \
         RememberRevealed$ True | SubAbility$ DBDmg\n\
         SVar:DBDmg:DB$ DealDamage | Defined$ You | NumDmg$ X | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        madcap.contains("then: &[] }, Effect::DealDamage { amount: Amount::ThisWay(ThisWay::Revealed(&Filter::Any)), target: TargetSpec::Player(PlayerRel::You) }"),
        "{madcap}"
    );

    // Composition: multi-hit `RevealUntilThen` -> `RepeatEachObject { only_found: true }`.
    let composed = ability(
        "Name:DoubleCastDig\nManaCost:5 R\nTypes:Sorcery\n\
         A:SP$ DigUntil | Amount$ 2 | Valid$ Card.nonLand | FoundDestination$ Exile | \
         RevealedDestination$ Library | RevealedLibraryPosition$ -1 | RevealRandomOrder$ True | \
         RememberFound$ True | SubAbility$ DBRep\n\
         SVar:DBRep:DB$ RepeatEach | RepeatCards$ Card.IsRemembered | Zone$ Exile | \
         RepeatSubAbility$ DBPlay | SubAbility$ DBClean\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | WithoutManaCost$ True | Optional$ True\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        composed.contains("count: Amount::Fixed(2)")
            && composed.contains(
                "Effect::RepeatEachObject { zone: ZoneSel::ExileAll, filter: &Filter::Any, \
                 only_found: true, then: &[Effect::Play {"
            ),
        "{composed}"
    );
}

#[test]
fn wave23a_repeat_each_object_and_two_piles() {
    // Non-battlefield `Zone$` variants: ExileAll, LinkedExile, GraveyardYou/Opponents/Chosen/All,
    // HandYou/Chosen/EachOpponent, LibraryYou/Chosen.
    let exile_all = ability(
        "Name:ExileSweeper\nManaCost:3 U\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatCards$ Card | Zone$ Exile | RepeatSubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | WithoutManaCost$ True | Optional$ True",
    );
    assert!(
        exile_all.contains("Effect::RepeatEachObject { zone: ZoneSel::ExileAll,"),
        "{exile_all}"
    );

    let linked_exile = ability(
        "Name:VaultRelease\nManaCost:3\nTypes:Artifact\n\
         A:AB$ RepeatEach | Cost$ T | RepeatCards$ Card.ExiledWithSource | Zone$ Exile | RepeatSubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | WithoutManaCost$ True | Optional$ True",
    );
    assert!(
        linked_exile.contains("Effect::RepeatEachObject { zone: ZoneSel::LinkedExile,"),
        "{linked_exile}"
    );

    let gy_you = ability(
        "Name:CryptSweep\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatCards$ Creature.YouOwn | Zone$ Graveyard | RepeatSubAbility$ DBRet\n\
         SVar:DBRet:DB$ ChangeZone | Defined$ Remembered | Origin$ Graveyard | Destination$ Hand",
    );
    assert!(
        gy_you.contains("Effect::RepeatEachObject { zone: ZoneSel::GraveyardYou,"),
        "{gy_you}"
    );

    let gy_chosen = ability(
        "Name:TombPurge\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ RepeatEach | ValidTgts$ Player | RepeatCards$ Artifact.TargetedPlayerOwn | \
         Zone$ Graveyard | RepeatSubAbility$ DBRet\n\
         SVar:DBRet:DB$ ChangeZone | Defined$ Remembered | Origin$ Graveyard | Destination$ Library | LibraryPosition$ -1",
    );
    assert!(
        gy_chosen
            .contains("Effect::RepeatEachObject { zone: ZoneSel::GraveyardOf(PlayerRel::Chosen),"),
        "{gy_chosen}"
    );

    let hand_opp = ability(
        "Name:ThoughtRake\nManaCost:3 B\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatCards$ Card.OppOwn | Zone$ Hand | RepeatSubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | Optional$ True",
    );
    assert!(
        hand_opp
            .contains("Effect::RepeatEachObject { zone: ZoneSel::HandOf(PlayerRel::EachOpponent),"),
        "{hand_opp}"
    );

    let lib_you = ability(
        "Name:LibrarySift\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatCards$ Card.YouOwn | Zone$ Library | RepeatSubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | WithoutManaCost$ True | Optional$ True",
    );
    assert!(
        lib_you.contains("Effect::RepeatEachObject { zone: ZoneSel::LibraryYou,"),
        "{lib_you}"
    );
}

#[test]
fn wave23a_repeat_each_object_copy_damage_nested_and_two_piles() {
    // `UseImprinted$ True` + `CopyPermanent | RememberTokens$ True` + `DelayedTrigger` (`EndCombat` Exile).
    let mirror_copy = ability(
        "Name:CombatMirage\nManaCost:4 R\nTypes:Creature Monk\nPT:3/3\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigRep\n\
         SVar:TrigRep:DB$ RepeatEach | RepeatCards$ Creature.YouCtrl+modified | UseImprinted$ True | \
         RepeatSubAbility$ DBCopy | SubAbility$ DBDel\n\
         SVar:DBCopy:DB$ CopyPermanent | Defined$ Imprinted | TokenTapped$ True | \
         TokenAttacking$ True | RememberTokens$ True\n\
         SVar:DBDel:DB$ DelayedTrigger | Mode$ Phase | Phase$ EndCombat | \
         RememberObjects$ Remembered | Execute$ DBExile | SubAbility$ DBClean\n\
         SVar:DBExile:DB$ ChangeZone | Origin$ Battlefield | Destination$ Exile | \
         Defined$ DelayTriggerRemembered\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True | ClearImprinted$ True",
    );
    assert!(
        mirror_copy.contains("Effect::RepeatEachObject { zone: ZoneSel::Battlefield, filter: &Filter::YOUR_MODIFIED_CREATURE")
            && mirror_copy.contains("Effect::CreateTokenCopy { target: TargetSpec::EventObject, controller: PlayerRel::You")
            && mirror_copy.contains("tapped: true, attacking: true, haste_until_eot: false, cleanup: CopyCleanup::ExileAtEndOfCombat }"),
        "{mirror_copy}"
    );

    // `CopyPermanent` with `Controller$ RememberedController` -> `PlayerRel::EventPlayer`.
    let parallel_copy = ability(
        "Name:TokenEcho\nManaCost:3 G G\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatCards$ Creature.token | RepeatSubAbility$ DBCopy\n\
         SVar:DBCopy:DB$ CopyPermanent | Defined$ Remembered | Controller$ RememberedController",
    );
    assert!(
        parallel_copy.contains(
            "Effect::CreateTokenCopy { target: TargetSpec::EventObject, \
             controller: PlayerRel::EventPlayer,"
        ),
        "{parallel_copy}"
    );

    // `DamageMap$ True` + `DealDamage | DamageSource$ Remembered` -> `EventObjectDealsDamage`.
    let charm_dmg = ability(
        "Name:ReboundHex\nManaCost:B R\nTypes:Instant\n\
         A:SP$ RepeatEach | RepeatCards$ Creature | DamageMap$ True | RepeatSubAbility$ DBDmg\n\
         SVar:DBDmg:DB$ DealDamage | Defined$ RememberedController | DamageSource$ Remembered | NumDmg$ 1",
    );
    assert!(
        charm_dmg.contains(
            "Effect::RepeatEachObject { zone: ZoneSel::Battlefield, filter: &Filter::CREATURE, \
             only_found: false, then: &[Effect::EventObjectDealsDamage { amount: Amount::Fixed(1), \
             target: TargetSpec::Player(PlayerRel::EventPlayer) }] }"
        ),
        "{charm_dmg}"
    );

    // `ShowCurrentCard$ Remembered` UI prose stripped cleanly.
    let cut_tethers = ability(
        "Name:TetherSnap\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatCards$ Creature | RepeatSubAbility$ DBBounce\n\
         SVar:DBBounce:DB$ ChangeZone | Defined$ Remembered | Origin$ Battlefield | \
         Destination$ Hand | ShowCurrentCard$ Remembered | UnlessCost$ 2 | \
         UnlessPayer$ RememberedController",
    );
    assert!(
        cut_tethers.contains(
            "Effect::RepeatEachObject { zone: ZoneSel::Battlefield, filter: &Filter::CREATURE"
        ),
        "{cut_tethers}"
    );

    // Nested `RepeatPlayers$ Player` -> `RepeatCards$ Creature.RememberedPlayerCtrl`.
    let fade_tax = ability(
        "Name:FadingTax\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatPlayers$ Player | RepeatSubAbility$ DBEach\n\
         SVar:DBEach:DB$ RepeatEach | RepeatCards$ Creature.RememberedPlayerCtrl | \
         RepeatSubAbility$ DBSac\n\
         SVar:DBSac:DB$ Sacrifice | Defined$ Player.IsRemembered | SacValid$ Permanent | \
         UnlessCost$ 1 | UnlessPayer$ Player.IsRemembered",
    );
    assert!(
        fade_tax.contains(
            "Effect::RepeatEachObject { zone: ZoneSel::Battlefield, filter: &Filter::CREATURE, only_found: false,"
        ),
        "{fade_tax}"
    );

    // `PeekAndReveal` -> `TwoPiles` (*Fact or Fiction* shape) -> `Effect::RevealAndSeparate { count }`.
    let fof = ability(
        "Name:TwinPiles\nManaCost:3 U\nTypes:Instant\n\
         A:SP$ PeekAndReveal | NoPeek$ True | RememberRevealed$ True | PeekAmount$ 5 | \
         SubAbility$ DBTwo\n\
         SVar:DBTwo:DB$ TwoPiles | Defined$ You | DefinedCards$ Remembered | \
         Separator$ Opponent | Chooser$ You | ChosenPile$ DBHand | UnchosenPile$ DBGrave | \
         SubAbility$ DBClean\n\
         SVar:DBHand:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Hand\n\
         SVar:DBGrave:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Graveyard\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        fof.contains("Effect::RevealAndSeparate { count: 5 }"),
        "{fof}"
    );
}

#[test]
fn wave23a_refusal_edge_cases() {
    for bad in [
        // Planechase / non-Paper `DigZone$ PlanarDeck`.
        "Name:BadPlanarDig\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ DigUntil | DigZone$ PlanarDeck | Valid$ Plane | FoundDestination$ Command",
        // Unmodelled `MinTotalCMC$`, `Attacking$`, `AttachedTo$`.
        "Name:BadMinCmc\nManaCost:4 R\nTypes:Sorcery\n\
         A:SP$ DigUntil | Valid$ Card | MinTotalCMC$ 5 | FoundDestination$ Exile | RevealedDestination$ Exile",
        "Name:BadAttackingDig\nManaCost:4 R\nTypes:Sorcery\n\
         A:SP$ DigUntil | Valid$ Creature | FoundDestination$ Battlefield | Attacking$ True | RevealedDestination$ Library | RevealedLibraryPosition$ -1",
        "Name:BadAttachedDig\nManaCost:3 W\nTypes:Sorcery\n\
         A:SP$ DigUntil | Valid$ Aura | FoundDestination$ Battlefield | AttachedTo$ Self | RevealedDestination$ Library | RevealedLibraryPosition$ -1",
        // `Optional$ True` on non-controller library.
        "Name:BadOppOptional\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ DigUntil | ValidTgts$ Opponent | Optional$ True | Valid$ Land | RevealedDestination$ Graveyard",
        // `OptionalFoundMove$ True` with bottom-random rest or Aura.
        "Name:BadOptBottom\nManaCost:3 G\nTypes:Sorcery\n\
         A:SP$ DigUntil | Valid$ Creature | FoundDestination$ Battlefield | OptionalFoundMove$ True | \
         RevealedDestination$ Library | RevealedLibraryPosition$ -1 | RevealRandomOrder$ True",
        "Name:BadOptAura\nManaCost:3 W\nTypes:Sorcery\n\
         A:SP$ DigUntil | Valid$ Aura | FoundDestination$ Battlefield | OptionalFoundMove$ True | \
         RevealedDestination$ Library | Shuffle$ True",
        // `RememberFound$ + RememberRevealed$` -> `Play` -> `ChangeZoneAll` rest to bottom
        // (uncast found card must randomize with rest).
        "Name:BadChaosWand\nManaCost:4\nTypes:Artifact\n\
         A:AB$ DigUntil | Cost$ 4 T | ValidTgts$ Opponent | Valid$ Instant,Sorcery | \
         FoundDestination$ Exile | RevealedDestination$ Exile | RememberFound$ True | \
         RememberRevealed$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | WithoutManaCost$ True | Optional$ True | SubAbility$ DBRest\n\
         SVar:DBRest:DB$ ChangeZoneAll | ChangeType$ Card.IsRemembered | Origin$ Exile | Destination$ Library | LibraryPosition$ -1 | RandomOrder$ True",
        // `DB$ Effect` `ThisTurn` permission on a permanent (would refresh prior turns' linked cards).
        "Name:BadPermThisTurn\nManaCost:3 R\nTypes:Artifact\n\
         A:AB$ DigUntil | Cost$ 2 T | Valid$ Card.nonLand | FoundDestination$ Exile | \
         RevealedDestination$ Library | RevealedLibraryPosition$ -1 | RevealRandomOrder$ True | \
         RememberFound$ True | SubAbility$ DBEff\n\
         SVar:DBEff:DB$ Effect | RememberObjects$ Remembered | StaticAbilities$ STPlay | ForgetOnMoved$ Exile | SubAbility$ DBClean\n\
         SVar:STPlay:Mode$ Continuous | Affected$ Card.IsRemembered | AffectedZone$ Exile | MayPlay$ True\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True",
        // Multi-seat `DigUntil` with `RememberFound$ True` -> `Play` (pooled multi-seat cast).
        "Name:BadMultiSeatPlay\nManaCost:5 R\nTypes:Sorcery\n\
         A:SP$ DigUntil | Defined$ Opponent | Valid$ Card.nonLand | FoundDestination$ Exile | \
         RevealedDestination$ Library | RevealedLibraryPosition$ -1 | RevealRandomOrder$ True | \
         RememberFound$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | WithoutManaCost$ True | Optional$ True",
        // `FoundLibraryPosition$ -1` with non-bottom rest.
        "Name:BadFoundBottomGyRest\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ DigUntil | Valid$ Land | FoundDestination$ Library | FoundLibraryPosition$ -1 | \
         RevealedDestination$ Graveyard",
        // `RepeatEach`: multi-zone, `ChooseOrder$`, standalone `IsRemembered`, `DamageMap` non-damage, targeted sub.
        "Name:BadMultiZoneRepeat\nManaCost:3 B\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatCards$ Creature | Zone$ Battlefield,Graveyard | RepeatSubAbility$ DBEx\n\
         SVar:DBEx:DB$ ChangeZone | Defined$ Remembered | Origin$ Graveyard | Destination$ Exile",
        "Name:BadChooseOrderRepeat\nManaCost:3 U\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatCards$ Creature | ChooseOrder$ True | RepeatSubAbility$ DBTap\n\
         SVar:DBTap:DB$ Tap | Defined$ Remembered",
        "Name:BadBareRememberedRepeat\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatCards$ Card.IsRemembered | Zone$ Exile | RepeatSubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered",
        "Name:BadDamageMapNonDmg\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatCards$ Creature | DamageMap$ True | RepeatSubAbility$ DBTap\n\
         SVar:DBTap:DB$ Tap | Defined$ Remembered",
        "Name:BadTargetedRepeatSub\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatCards$ Creature.YouCtrl | RepeatSubAbility$ DBDmg\n\
         SVar:DBDmg:DB$ DealDamage | ValidTgts$ Any | NumDmg$ 1",
        // `PeekAndReveal` -> `TwoPiles` refusals: inverted roles, face-down, bottom-of-library, dynamic count, non-You library.
        "Name:BadInvertedPiles\nManaCost:2 U R\nTypes:Instant\n\
         A:SP$ PeekAndReveal | NoPeek$ True | RememberRevealed$ True | PeekAmount$ 5 | SubAbility$ DBTwo\n\
         SVar:DBTwo:DB$ TwoPiles | Defined$ You | DefinedCards$ Remembered | Separator$ You | Chooser$ Opponent | ChosenPile$ DBH | UnchosenPile$ DBG\n\
         SVar:DBH:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Hand\n\
         SVar:DBG:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Graveyard",
        "Name:BadFaceDownPiles\nManaCost:3 U\nTypes:Instant\n\
         A:SP$ PeekAndReveal | NoPeek$ True | RememberRevealed$ True | PeekAmount$ 4 | SubAbility$ DBTwo\n\
         SVar:DBTwo:DB$ TwoPiles | Defined$ You | DefinedCards$ Remembered | Separator$ Opponent | Chooser$ You | FaceDown$ One | ChosenPile$ DBH | UnchosenPile$ DBG\n\
         SVar:DBH:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Hand\n\
         SVar:DBG:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Graveyard",
        "Name:BadBottomPiles\nManaCost:3 U\nTypes:Instant\n\
         A:SP$ PeekAndReveal | NoPeek$ True | RememberRevealed$ True | PeekAmount$ 3 | SubAbility$ DBTwo\n\
         SVar:DBTwo:DB$ TwoPiles | Defined$ You | DefinedCards$ Remembered | Separator$ Opponent | Chooser$ You | ChosenPile$ DBH | UnchosenPile$ DBL\n\
         SVar:DBH:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Hand\n\
         SVar:DBL:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Library | LibraryPosition$ -1",
        "Name:BadDynamicPiles\nManaCost:X U\nTypes:Instant\n\
         A:SP$ PeekAndReveal | NoPeek$ True | RememberRevealed$ True | PeekAmount$ X | SubAbility$ DBTwo\n\
         SVar:DBTwo:DB$ TwoPiles | Defined$ You | DefinedCards$ Remembered | Separator$ Opponent | Chooser$ You | ChosenPile$ DBH | UnchosenPile$ DBG\n\
         SVar:DBH:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Hand\n\
         SVar:DBG:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Graveyard\n\
         SVar:X:Count$xPaid",
        "Name:BadOppLibraryPiles\nManaCost:3 U\nTypes:Instant\n\
         A:SP$ PeekAndReveal | Defined$ Opponent | NoPeek$ True | RememberRevealed$ True | PeekAmount$ 5 | SubAbility$ DBTwo\n\
         SVar:DBTwo:DB$ TwoPiles | Defined$ You | DefinedCards$ Remembered | Separator$ Opponent | Chooser$ You | ChosenPile$ DBH | UnchosenPile$ DBG\n\
         SVar:DBH:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Hand\n\
         SVar:DBG:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Graveyard",
        // Digital-only Alchemy guard (`MakeCard`).
        "Name:BadAlchemyMakeCard\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ MakeCard | Name$ Lightning Bolt | Zone$ Hand",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}
