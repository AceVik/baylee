//! Wave 24e unit tests: `Dig` (`Dig.RememberChanged`, `DigUntil`, conditional
//! `ChangeNum$`, `ConditionCheckSVar$`, `Count$CardCounters.*`, `RandomChange$`,
//! `DestinationZone2$ Hand`, `TopOfLibrary`), `etbCounter` keywords, non-LKI
//! `Moved` replacements and `Phase.CheckSVar` triggers, mandatory self-sacrifice
//! trigger costs, `*/*+N` CDA tokens, and strict Paper-MTG refusals.

use super::*;

fn w24e_cats() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Beast".into(),
            "Cleric".into(),
            "Demon".into(),
            "Dragon".into(),
            "Elf".into(),
            "Goblin".into(),
            "Gorgon".into(),
            "Hellion".into(),
            "Human".into(),
            "Knight".into(),
            "Ogre".into(),
            "Ooze".into(),
            "Phyrexian".into(),
            "Pilot".into(),
            "Scout".into(),
            "Shaman".into(),
            "Soldier".into(),
            "Spirit".into(),
            "Treefolk".into(),
            "Vampire".into(),
            "Warrior".into(),
            "Whale".into(),
            "Wizard".into(),
            "Wurm".into(),
        ],
        artifact: vec!["Clue".into(), "Food".into(), "Treasure".into()],
        land: vec![
            "Forest".into(),
            "Island".into(),
            "Mountain".into(),
            "Plains".into(),
            "Swamp".into(),
        ],
        ..SubtypeCatalogs::default()
    };
    c.normalize();
    c
}

fn read_w24e(text: &str) -> CardBody {
    let parsed = parse(text);
    transcode(&parsed, &w24e_cats(), None).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &w24e_cats(), None)
        )
    })
}

fn refused_w24e(text: &str) -> bool {
    transcode(&parse(text), &w24e_cats(), None).is_none()
}

// ---------------------------------------------------------------------------
// 1. Dig extensions & DigUntil
// ---------------------------------------------------------------------------

#[test]
fn dig_conditional_change_num_and_condition_check_svar() {
    // Accumulate Wisdom-like conditional `ChangeNum$` via `Count$Compare`.
    let body = read_w24e(
        "Name:Lesson Dig\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Dig | DigNum$ 3 | ChangeNum$ X | RestRandomOrder$ True\n\
         SVar:X:Count$Compare Y GE3.3.1\n\
         SVar:Y:Count$ValidGraveyard Instant.YouOwn",
    );
    let joined = body.abilities.join("\n");
    assert!(joined.contains("Effect::IfCondition"), "{joined}");
    assert!(joined.contains("pick: 3"), "{joined}");
    assert!(joined.contains("pick: 1"), "{joined}");

    // Advice from the Fae: `ConditionCheckSVar$` with `ControlMoreThanEachOtherPlayer`
    // across complementary `SP$ Dig` -> `DB$ Dig`.
    let fae = read_w24e(
        "Name:Advice from the Fae\nManaCost:2U 2U 2U\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 5 | ChangeNum$ 1 | SubAbility$ Dig2 | ConditionCheckSVar$ X | ConditionSVarCompare$ LEY\n\
         SVar:Dig2:DB$ Dig | DigNum$ 5 | ChangeNum$ 2 | ConditionCheckSVar$ X | ConditionSVarCompare$ GTY\n\
         SVar:X:Count$Valid Creature.YouCtrl\n\
         SVar:Y:PlayerCountOther$HighestValid Creature.YouCtrl",
    );
    let fae_ab = fae.abilities.join("\n");
    assert!(
        fae_ab.contains(
            "Condition::Not(&Condition::ControlMoreThanEachOtherPlayer(&Filter::CREATURE))"
        ),
        "{fae_ab}"
    );
    assert!(
        fae_ab.contains("Condition::ControlMoreThanEachOtherPlayer(&Filter::CREATURE)"),
        "{fae_ab}"
    );
    assert!(fae_ab.contains("pick: 1"), "{fae_ab}");
    assert!(fae_ab.contains("pick: 2"), "{fae_ab}");
}

#[test]
fn dig_random_change_top_dest_hand_rest_and_source_counters() {
    // Orcish Librarian-like `RandomChange$ True` -> `LookAtTopExileRandom`.
    let lib = read_w24e(
        "Name:Library Shredder\nManaCost:1 R\nTypes:Creature Human\nPT:1/1\n\
         A:AB$ Dig | Cost$ R T | DigNum$ 8 | ChangeNum$ 4 | DestinationZone$ Exile | LibraryPosition2$ 0 | RandomChange$ True",
    );
    assert!(
        lib.abilities[0].contains("Effect::LookAtTopExileRandom"),
        "{:?}",
        lib.abilities
    );

    // Coiling Oracle-like `DestinationZone$ Battlefield | DestinationZone2$ Hand`.
    let oracle = read_w24e(
        "Name:Top Sort\nManaCost:G U\nTypes:Creature Elf Wizard\nPT:1/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDig\n\
         SVar:TrigDig:DB$ Dig | DigNum$ 1 | Reveal$ True | ChangeNum$ All | ChangeValid$ Land | DestinationZone$ Battlefield | DestinationZone2$ Hand",
    );
    assert!(
        oracle.abilities[0].contains("RevealTopAndSort"),
        "{:?}",
        oracle.abilities
    );

    // Dimir Charm-like `DestinationZone$ Library | LibraryPosition$ 0 | DestinationZone2$ Graveyard`.
    let charm = read_w24e(
        "Name:Top One Rest Yard\nManaCost:U B\nTypes:Instant\n\
         A:SP$ Dig | ValidTgts$ Player | DigNum$ 3 | ChangeNum$ 1 | DestinationZone$ Library | LibraryPosition$ 0 | DestinationZone2$ Graveyard | NoReveal$ True",
    );
    assert!(
        charm.abilities[0].contains("PeekAndReveal"),
        "{:?}",
        charm.abilities
    );
    assert!(
        charm.abilities[0].contains("PeekRestDest::Graveyard"),
        "{:?}",
        charm.abilities
    );

    // Wilfred Mott-like `Count$CardCounters.Time` (TitleCase counter name on permanent).
    let mott = read_w24e(
        "Name:Star Gazer\nManaCost:3 W\nTypes:Creature Human Soldier\nPT:2/4\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigPut\n\
         SVar:TrigPut:DB$ PutCounter | Defined$ Self | CounterType$ TIME | CounterNum$ 1 | SubAbility$ DBDig\n\
         SVar:DBDig:DB$ Dig | DigNum$ X | ChangeNum$ 1 | Optional$ True | ChangeValid$ Permanent.nonLand+cmcLE3 | DestinationZone$ Battlefield | RestRandomOrder$ True\n\
         SVar:X:Count$CardCounters.Time",
    );
    assert!(
        mott.abilities[0].contains("Amount::CountersOnSource(CounterKind::Time)"),
        "{:?}",
        mott.abilities
    );
}

#[test]
fn dig_until_named_exile_count_spoils_of_the_vault() {
    let spoils = read_w24e(
        "Name:Spoils of the Vault\nManaCost:B\nTypes:Instant\n\
         A:SP$ NameCard | Defined$ You | SubAbility$ DBDigUntil\n\
         SVar:DBDigUntil:DB$ DigUntil | Valid$ Card.NamedCard | FoundDestination$ Hand | RevealedDestination$ Exile | ValidDescription$ named card | RememberRevealed$ True | SubAbility$ DBLoseLife\n\
         SVar:DBLoseLife:DB$ LoseLife | LifeAmount$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    let ab = &spoils.abilities[0];
    assert!(ab.contains("Effect::ChooseCardName"), "{ab}");
    assert!(ab.contains("Effect::RevealUntilNamed"), "{ab}");
    assert!(
        ab.contains("Amount::ThisWay(ThisWay::ExiledFromLibrary(&Filter::Any))"),
        "{ab}"
    );
}

// ---------------------------------------------------------------------------
// 2. Trigger Dig.RememberChanged patterns (A–E)
// ---------------------------------------------------------------------------

#[test]
fn trigger_dig_remember_changed_patterns_a_and_b() {
    // Pattern A: Reveal top, event-object life loss (`Baneful Omen`).
    let omen = read_w24e(
        "Name:Omen Enchantment\nManaCost:4 B B B\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | OptionalDecider$ You | Execute$ TrigDig\n\
         SVar:TrigDig:DB$ Dig | DigNum$ 1 | Reveal$ True | ChangeNum$ All | ChangeValid$ Card | DestinationZone$ Library | LibraryPosition$ 0 | LibraryPosition2$ 0 | RememberChanged$ True | SubAbility$ DBLoseLife\n\
         SVar:DBLoseLife:DB$ LoseLife | Defined$ Player.Opponent | LifeAmount$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost",
    );
    assert!(
        omen.abilities[0].contains("Effect::RevealTop"),
        "{:?}",
        omen.abilities
    );
    assert!(
        omen.abilities[0].contains("Amount::EventManaValue"),
        "{:?}",
        omen.abilities
    );

    // Pattern B1: `Blessing$ True` + Reveal top -> Hand + Drain (`Twilight Prophet`).
    let prophet = read_w24e(
        "Name:Twilight Seer\nManaCost:2 B B\nTypes:Creature Vampire Cleric\nPT:2/4\n\
         K:Ascend\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Blessing$ True | Execute$ TrigReveal\n\
         SVar:TrigReveal:DB$ Dig | DigNum$ 1 | Reveal$ True | ChangeNum$ All | ChangeValid$ Card | DestinationZone$ Hand | RememberChanged$ True | SubAbility$ DBLoseLife\n\
         SVar:DBLoseLife:DB$ LoseLife | Defined$ Player.Opponent | LifeAmount$ X | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | LifeAmount$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost",
    );
    assert!(
        prophet.abilities[0].contains("Condition::CitysBlessing"),
        "{:?}",
        prophet.abilities
    );
    assert!(
        prophet.abilities[0].contains("ThisWay::RevealedManaValue"),
        "{:?}",
        prophet.abilities
    );

    // Pattern B2: Reveal top -> Hand (if Creature) + gain toughness / lose power (`Sapling of Colfenor`).
    let sapling = read_w24e(
        "Name:Colfenor Sapling\nManaCost:3 BG BG\nTypes:Creature Treefolk Shaman\nPT:2/5\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigDig\n\
         SVar:TrigDig:DB$ Dig | DigNum$ 1 | Reveal$ True | ChangeNum$ All | ChangeValid$ Creature | DestinationZone$ Hand | RememberChanged$ True | DestinationZone2$ Library | LibraryPosition2$ 0 | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainLife | LifeAmount$ Y | SubAbility$ DBLose\n\
         SVar:DBLose:DB$ LoseLife | LifeAmount$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:Y:Remembered$CardToughness\n\
         SVar:X:Remembered$CardPower",
    );
    assert!(
        sapling.abilities[0].contains("Effect::RevealUntilThen"),
        "{:?}",
        sapling.abilities
    );
    assert!(
        sapling.abilities[0].contains("Amount::EventLastToughness"),
        "{:?}",
        sapling.abilities
    );
    assert!(
        sapling.abilities[0].contains("Amount::EventPower"),
        "{:?}",
        sapling.abilities
    );
}

#[test]
fn trigger_dig_remember_changed_pattern_c() {
    // Pattern C2: Exile top + targeted opponent loses life + conditional free cast (`Hidetsugu and Kairi`).
    let hk = read_w24e(
        "Name:Ogre Demon\nManaCost:2 U U B\nTypes:Creature Ogre Demon Dragon\nPT:5/4\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ Dig | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBLoseLife\n\
         SVar:DBLoseLife:DB$ LoseLife | LifeAmount$ X | ValidTgts$ Opponent | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | ConditionCheckSVar$ Z | ConditionSVarCompare$ EQ1 | ValidSA$ Spell | WithoutManaCost$ True | Optional$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost\n\
         SVar:Z:Remembered$Valid Instant,Sorcery",
    );
    assert!(
        hk.abilities[0].contains("exile_while_revealing: true"),
        "{:?}",
        hk.abilities
    );
    assert!(
        hk.abilities[0].contains(
            "Effect::LoseLife { amount: Amount::EventManaValue, target: PlayerRel::Chosen }"
        ),
        "{:?}",
        hk.abilities
    );
    assert!(
        hk.abilities[0].contains("Effect::IfEventObjectMatches"),
        "{:?}",
        hk.abilities
    );
    assert!(
        hk.abilities[0]
            .contains("Effect::Play { source: PlaySource::Subject(TargetSpec::EventObject)"),
        "{:?}",
        hk.abilities
    );
    assert!(
        hk.abilities[0].contains("targets = Some(TargetReq::one(TargetSpec::AnyOpponent))"),
        "{:?}",
        hk.abilities
    );
}

#[test]
fn trigger_dig_remember_changed_patterns_d_and_e() {
    // Pattern D: `PeekAndReveal` with `ConditionPresent$ Card | ConditionCompare$ EQ0` fallback (`Blossom Prancer`).
    let prancer = read_w24e(
        "Name:Forest Spirit\nManaCost:3 G G\nTypes:Creature Spirit\nPT:4/4\n\
         T:Mode$ ChangesZone | Destination$ Battlefield | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ TrigDig\n\
         SVar:TrigDig:DB$ Dig | DigNum$ 5 | ChangeNum$ 1 | ChangeValid$ Creature,Enchantment | Optional$ True | RestRandomOrder$ True | RememberChanged$ True | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | Defined$ You | LifeAmount$ 4 | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ EQ0 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        prancer.abilities[0].contains("Effect::PeekAndReveal"),
        "{:?}",
        prancer.abilities
    );
    assert!(
        prancer.abilities[0].contains("ThisWay::Revealed"),
        "{:?}",
        prancer.abilities
    );

    // Pattern E: `Lurking Predators` (`RevealTop` -> `MoveAndModify` or optional bottom).
    let predators = read_w24e(
        "Name:Lurking Beast\nManaCost:4 G G\nTypes:Enchantment\n\
         T:Mode$ SpellCast | ValidCard$ Card | ValidActivatingPlayer$ Opponent | TriggerZones$ Battlefield | Execute$ TrigDig\n\
         SVar:TrigDig:DB$ Dig | DigNum$ 1 | Reveal$ True | ChangeNum$ All | ChangeValid$ Creature | DestinationZone$ Battlefield | LibraryPosition2$ 0 | RememberChanged$ True | SubAbility$ DBBottom\n\
         SVar:DBBottom:DB$ Dig | DigNum$ 1 | DestinationZone$ Library | Optional$ True | LibraryPosition$ -1 | LibraryPosition2$ 0 | ConditionCheckSVar$ X | ConditionSVarCompare$ EQ0 | SubAbility$ DBCleanup\n\
         SVar:X:Remembered$Amount\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        predators.abilities[0].contains("Effect::RevealTop"),
        "{:?}",
        predators.abilities
    );
    assert!(
        predators.abilities[0].contains("Effect::MoveAndModify"),
        "{:?}",
        predators.abilities
    );
    assert!(
        predators.abilities[0].contains("PeekRestDest::BottomOfLibrary { random: false }"),
        "{:?}",
        predators.abilities
    );
}

// ---------------------------------------------------------------------------
// 3. etbCounter keywords & non-LKI Moved replacements
// ---------------------------------------------------------------------------

#[test]
fn etb_counter_keywords_wave24e() {
    // Realm Seekers: `Count$ValidHand Card`.
    let seekers = read_w24e(
        "Name:Hand Counter Scout\nManaCost:4 G G\nTypes:Creature Elf Scout\nPT:0/0\n\
         K:etbCounter:P1P1:X\n\
         SVar:X:Count$ValidHand Card",
    );
    assert!(
        seekers
            .enter_modifiers
            .iter()
            .any(|m| m.contains("ZoneSel::Hand")),
        "{:?}",
        seekers.enter_modifiers
    );

    // Frontier Mastodon: qualified `Count$Valid Creature.YouCtrl+powerGE4` without `SVarCompare$`.
    let mastodon = read_w24e(
        "Name:Ferocious Elephant\nManaCost:2 G\nTypes:Creature Beast\nPT:3/2\n\
         K:etbCounter:P1P1:1:CheckSVar$ X\n\
         SVar:X:Count$Valid Creature.YouCtrl+powerGE4",
    );
    assert!(
        mastodon
            .enter_modifiers
            .iter()
            .any(|m| m.contains("EnterModifier::WithCountersIf")),
        "{:?}",
        mastodon.enter_modifiers
    );

    // `Count$wasCastFromYourHandByYou.1.0` conditional ETB counter.
    let hand_cast = read_w24e(
        "Name:Hand Cast Knight\nManaCost:2 W\nTypes:Creature Human Knight\nPT:2/2\n\
         K:etbCounter:P1P1:1:CheckSVar$ X\n\
         SVar:X:Count$wasCastFromYourHandByYou.1.0",
    );
    assert!(
        hand_cast
            .enter_modifiers
            .iter()
            .any(|m| m.contains("Filter::CastFromHand")),
        "{:?}",
        hand_cast.enter_modifiers
    );
}

#[test]
fn self_etb_moved_replacements_wave24e() {
    // Shimatsu the Bloodcloaked & Devouring Hellion: `EnterModifier::Devour` via `Moved` replacement.
    let shimatsu = read_w24e(
        "Name:Shimatsu the Bloodcloaked\nManaCost:3 R\nTypes:Legendary Creature Demon Spirit\nPT:0/0\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ TrigSac | ReplacementResult$ Updated\n\
         SVar:TrigSac:DB$ Sacrifice | Amount$ SacX | SacValid$ Permanent | Defined$ You | RememberSacrificed$ True | Optional$ True | SubAbility$ DBPutCounter\n\
         SVar:DBPutCounter:DB$ PutCounter | ETB$ True | CounterType$ P1P1 | Defined$ Self | CounterNum$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:SacX:Count$Valid Permanent.YouCtrl\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        shimatsu
            .enter_modifiers
            .iter()
            .any(|m| m == "EnterModifier::Devour { filter: &Filter::Any, factor: 1 }"),
        "{:?}",
        shimatsu.enter_modifiers
    );

    let hellion = read_w24e(
        "Name:Devouring Hellion\nManaCost:2 R\nTypes:Creature Hellion\nPT:2/2\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ DevourCreatsAndWalkers | ReplacementResult$ Updated\n\
         SVar:DevourCreatsAndWalkers:DB$ Sacrifice | Defined$ You | Amount$ DevourSacX | SacValid$ Creature.Other,Planeswalker | SacMessage$ any number of creatures and/or planeswalkers | RememberSacrificed$ True | Optional$ True | SubAbility$ DevourCounters\n\
         SVar:DevourCounters:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ Devour2X | ETB$ True | SubAbility$ DevourCleanup\n\
         SVar:DevourCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:DevourSacX:Count$Valid Creature.YouCtrl+Other,Planeswalker.YouCtrl\n\
         SVar:Devour2X:Count$RememberedSize/Times.2",
    );
    assert!(
        hellion
            .enter_modifiers
            .iter()
            .any(|m| m.contains("EnterModifier::Devour") && m.contains("factor: 2")),
        "{:?}",
        hellion.enter_modifiers
    );

    // Pharika's Spawn: `Card.Self+escaped` -> `WithCountersIf(Escaped)` + `Trigger::ETB(Escaped)`.
    let spawn = read_w24e(
        "Name:Pharika's Spawn\nManaCost:3 B\nTypes:Creature Gorgon\nPT:3/4\n\
         K:Escape:5 B ExileFromGrave<3/Card.Other/other>\n\
         R:Event$ Moved | ValidCard$ Card.Self+escaped | Destination$ Battlefield | ReplaceWith$ Escaped | ReplacementResult$ Updated\n\
         SVar:Escaped:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 2 | ETB$ True | SubAbility$ DBImmediate\n\
         SVar:DBImmediate:DB$ ImmediateTrigger | Execute$ TrigSac\n\
         SVar:TrigSac:DB$ Sacrifice | Defined$ Player.Opponent | SacValid$ Creature",
    );
    assert!(
        spawn
            .enter_modifiers
            .iter()
            .any(|m| m.contains("Condition::Escaped")),
        "{:?}",
        spawn.enter_modifiers
    );
    assert!(
        spawn
            .abilities
            .iter()
            .any(|a| a.contains("Trigger::ETB") && a.contains("Condition::Escaped")),
        "{:?}",
        spawn.abilities
    );

    // Multiversal Passage: `ChooseType(Basic Land)` -> `Tap | UnlessCost$ PayLife<2>`.
    let passage = read_w24e(
        "Name:Multiversal Passage\nManaCost:no cost\nTypes:Land\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ ChooseType | ReplacementResult$ Updated\n\
         SVar:ChooseType:DB$ ChooseType | ETB$ True | Type$ Basic Land | SubAbility$ DBTap\n\
         SVar:DBTap:DB$ Tap | ETB$ True | Defined$ Self | UnlessCost$ PayLife<2> | UnlessPayer$ You",
    );
    assert!(
        passage
            .enter_modifiers
            .iter()
            .any(|m| m == "EnterModifier::ChooseBasicLandType"),
        "{:?}",
        passage.enter_modifiers
    );
    assert!(
        passage
            .enter_modifiers
            .iter()
            .any(|m| m == "EnterModifier::TappedOrPayLife(2)"),
        "{:?}",
        passage.enter_modifiers
    );
}

// ---------------------------------------------------------------------------
// 4. Triggers: Mandatory Sac<1/CARDNAME>, NotPlayerTurn, HighestLifeLostThisTurn
// ---------------------------------------------------------------------------

#[test]
fn trigger_mandatory_sac_self_not_player_turn_and_highest_life_lost() {
    // Aspiring Champion-like `AB$ ... | Cost$ Mandatory Sac<1/CARDNAME>` -> `Effect::SacrificeSelfThen`.
    let champ = read_w24e(
        "Name:Sacrifice Scout\nManaCost:3 R\nTypes:Creature Human Warrior\nPT:3/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigSac\n\
         SVar:TrigSac:AB$ Draw | Cost$ Mandatory Sac<1/CARDNAME> | NumCards$ 2",
    );
    assert!(
        champ.abilities[0].contains("Effect::SacrificeSelfThen"),
        "{:?}",
        champ.abilities
    );
    assert!(
        champ.abilities[0].contains("Effect::draw(2)"),
        "{:?}",
        champ.abilities
    );

    // Lighthouse Chronologist-like `NotPlayerTurn$ True`.
    let chrono = read_w24e(
        "Name:Time Keeper\nManaCost:1 U\nTypes:Creature Human Wizard\nPT:1/3\n\
         T:Mode$ Phase | Phase$ End of Turn | NotPlayerTurn$ True | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(
        chrono.abilities[0].contains("Condition::Not(&Condition::YourTurn)"),
        "{:?}",
        chrono.abilities
    );

    // `PlayerCountOpponents$HighestLifeLostThisTurn` with `GE1` and `EQ0`.
    let lost_life = read_w24e(
        "Name:Blood Tracker\nManaCost:2 B\nTypes:Creature Vampire\nPT:2/2\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | CheckSVar$ X | SVarCompare$ GE1 | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1\n\
         SVar:X:PlayerCountOpponents$HighestLifeLostThisTurn",
    );
    assert!(
        lost_life.abilities[0].contains("Condition::OpponentLostLifeThisTurn"),
        "{:?}",
        lost_life.abilities
    );
}

// ---------------------------------------------------------------------------
// 5. Tokengen `*/*+N` CDA validation
// ---------------------------------------------------------------------------

#[test]
fn tokengen_star_plus_n_requires_matching_cda() {
    let cats = w24e_cats();

    // Valid `*/*+1` token with matching `CharacteristicPT { ..., toughness_plus: 1 }` CDA.
    let valid_ooze = crate::tokengen::read(
        "Name:Ooze Token\nManaCost:no cost\nColors:green\nTypes:Creature Ooze\nPT:*/*+1\n\
         S:Mode$ Continuous | CharacteristicDefining$ True | SetPower$ X | SetToughness$ Y\n\
         SVar:X:Count$ValidGraveyard Card$CardTypes\n\
         SVar:Y:SVar$X/Plus.1",
        &cats,
    )
    .expect("valid */*+1 CDA token should be read");
    assert!(
        valid_ooze.constant.contains("_X_X_PLUS_1_"),
        "{}",
        valid_ooze.constant
    );
    assert!(valid_ooze.doc.contains("*/*+1 "), "{}", valid_ooze.doc);
    assert!(valid_ooze.has_characteristic_pt(), "{}", valid_ooze.literal);

    // Refused: `*/*+1` token with NO CDA at all.
    assert!(
        crate::tokengen::read(
            "Name:Ooze Token\nManaCost:no cost\nColors:green\nTypes:Creature Ooze\nPT:*/*+1",
            &cats,
        )
        .is_none(),
        "*/*+1 without CDA must be refused"
    );

    // Refused: `*/*+1` token whose CDA sets toughness_plus = 2 (mismatch!).
    assert!(
        crate::tokengen::read(
            "Name:Ooze Token\nManaCost:no cost\nColors:green\nTypes:Creature Ooze\nPT:*/*+1\n\
             S:Mode$ Continuous | CharacteristicDefining$ True | SetPower$ X | SetToughness$ Y\n\
             SVar:X:Count$ValidGraveyard Card$CardTypes\n\
             SVar:Y:SVar$X/Plus.2",
            &cats,
        )
        .is_none(),
        "*/*+1 with mismatched CDA toughness_plus must be refused"
    );
}

// ---------------------------------------------------------------------------
// 6. Strict Paper-MTG & Semantic-Soundness Refusals
// ---------------------------------------------------------------------------

#[test]
fn strict_refusals_for_alchemy_unset_planechase_and_unsound_approximations() {
    // Alchemy mechanics must be strictly refused.
    assert!(refused_w24e(
        "Name:Alchemy Seek\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ Seek | Type$ Creature"
    ));
    assert!(refused_w24e(
        "Name:Alchemy Conjure\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ MakeCard | Name$ Lightning Bolt | Zone$ Hand"
    ));
    assert!(refused_w24e(
        "Name:Alchemy Perpetual\nManaCost:1 B\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -1 | NumDef$ -1 | Perpetual$ True"
    ));

    // Un-set Sticker / Planechase mechanics must be strictly refused.
    assert!(refused_w24e(
        "Name:Sticker Card\nManaCost:2 W\nTypes:Creature Human\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigSticker\n\
         SVar:TrigSticker:DB$ PutSticker | Defined$ Self"
    ));
    assert!(refused_w24e(
        "Name:Planechase Card\nManaCost:3 U\nTypes:Enchantment\n\
         T:Mode$ PlaneswalkedTo | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1"
    ));

    // Multiplayer-unsound `PlayerCountOpponents$HighestLifeLostThisTurn GE2` MUST stay refused
    // (summing across opponents is not equal to max single opponent's life lost!).
    assert!(refused_w24e(
        "Name:Multiplayer Trap\nManaCost:2 B\nTypes:Creature Vampire\nPT:2/2\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | CheckSVar$ X | SVarCompare$ GE2 | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1\n\
         SVar:X:PlayerCountOpponents$HighestLifeLostThisTurn"
    ));

    // Mandatory Sac<1/CARDNAME> that reads `Sacrificed$` LKI MUST stay refused.
    assert!(refused_w24e(
        "Name:LKI Sac Trap\nManaCost:2 R\nTypes:Creature Goblin\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigSac\n\
         SVar:TrigSac:AB$ DealDamage | Cost$ Mandatory Sac<1/CARDNAME> | ValidTgts$ Any | NumDmg$ X\n\
         SVar:X:Sacrificed$CardPower"
    ));

    // Non-trigger `A:SP$ Dig | RememberChanged$ True` with no SubAbility MUST stay refused.
    assert!(refused_w24e(
        "Name:Bare Remember Dig\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 3 | ChangeNum$ 1 | RememberChanged$ True"
    ));

    // Unqualified `Count$Valid Creature.YouCtrl` on `etbCounter` (`CondEtb`) MUST stay refused.
    assert!(refused_w24e(
        "Name:Plain Valid Etb\nManaCost:2 G\nTypes:Creature Beast\nPT:2/2\n\
         K:etbCounter:P1P1:1:CheckSVar$ X\n\
         SVar:X:Count$Valid Creature.YouCtrl"
    ));
}
