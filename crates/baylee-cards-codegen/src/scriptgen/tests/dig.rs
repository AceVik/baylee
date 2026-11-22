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

    // Refusal: `DigUntil` with unhandled `RememberFound$ True` -> `Play`.
    assert!(refused(
        "Name:BadDigPlay\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ DigUntil | Valid$ Card.nonLand | FoundDestination$ Exile | RevealedDestination$ Exile | RememberFound$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | WithoutManaCost$ True"
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
