//! Wave 28a unit tests:
//! 1. Nonzero `TargetMin$`..`TargetMax$` target ranges, Slot-2/3 exact & range
//!    counts, multi-target graveyard-to-top-of-library, up-to-one `AlterAttribute`
//!    (`Suspected`) & `Clone`, and activated X-target abilities.
//! 2. Generalized `DealDamage | DividedAsYouChoose$` across spells (`TargetMin$ 1`,
//!    Slot-2 continuations), activated/loyalty abilities, charm modes, and
//!    object-only triggered abilities (strictly refusing player-including triggers).
//! 3. `ChangeZone` & `CaptureAffected` / `RememberChanged` gaps and soundness
//!    refusal boundaries.

use super::*;

fn w28a_cats() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Dinosaur".into(),
            "Dragon".into(),
            "Elemental".into(),
            "Goblin".into(),
            "Human".into(),
            "Wizard".into(),
            "Zombie".into(),
            "Fractal".into(),
        ],
        artifact: vec!["Clue".into(), "Equipment".into(), "Treasure".into()],
        land: vec![
            "Forest".into(),
            "Island".into(),
            "Mountain".into(),
            "Plains".into(),
            "Swamp".into(),
            "Desert".into(),
        ],
        ..SubtypeCatalogs::default()
    };
    c.normalize();
    c
}

fn read_w28a(text: &str) -> CardBody {
    let parsed = parse(text);
    transcode(&parsed, &w28a_cats(), None).unwrap_or_else(|| {
        let why = refusal_reason(&parsed, &w28a_cats(), None);
        panic!("expected Wave 28a script to transcode, got refusal `{why:?}`:\n{text}");
    })
}

fn refused_w28a(text: &str) -> bool {
    let parsed = parse(text);
    transcode(&parsed, &w28a_cats(), None).is_none()
}

#[test]
fn nonzero_target_ranges_and_multi_graveyard_top() {
    // 1..=2 target range on ChangeZone bounce.
    let bounce_range = read_w28a(
        "Name:TwinBounce\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Hand | ValidTgts$ Creature | TargetMin$ 1 | TargetMax$ 2",
    );
    assert!(
        bounce_range.abilities[0].contains(
            "TargetReq { min: 1, ..TargetReq::up_to(TargetSpec::Object(&Filter::CREATURE), 2) }"
        ),
        "{}",
        bounce_range.abilities[0]
    );

    // 1..=3 target range on Pump.
    let pump_range = read_w28a(
        "Name:TriplePump\nManaCost:2 G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | TargetMin$ 1 | TargetMax$ 3 | NumAtt$ +2 | NumDef$ +2",
    );
    assert!(
        pump_range.abilities[0].contains(
            "TargetReq { min: 1, ..TargetReq::up_to(TargetSpec::Object(&Filter::CREATURE), 3) }"
        ),
        "{}",
        pump_range.abilities[0]
    );

    // 1..=whole-pool SVar max -> u8::MAX.
    let pool_range = read_w28a(
        "Name:TapOneOrMore\nManaCost:3 U\nTypes:Instant\n\
         A:SP$ Tap | ValidTgts$ Creature | TargetMin$ 1 | TargetMax$ X\n\
         SVar:X:Count$Valid Creature",
    );
    assert!(
        pool_range.abilities[0].contains(
            "TargetReq { min: 1, ..TargetReq::up_to(TargetSpec::Object(&Filter::CREATURE), 255) }"
        ),
        "{}",
        pool_range.abilities[0]
    );

    // Slot-2 range (1..=3) alongside Slot-1.
    let slot2_range = read_w28a(
        "Name:DestroyAndTapRange\nManaCost:3 W B\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Creature | SubAbility$ TapRange\n\
         SVar:TapRange:DB$ Tap | ValidTgts$ Creature | TargetMin$ 1 | TargetMax$ 3",
    );
    assert!(
        slot2_range.abilities[0].contains(
            "second_targets = Some(TargetReq { min: 1, ..TargetReq::up_to(TargetSpec::Object(&Filter::CREATURE), 3) })"
        ),
        "{}",
        slot2_range.abilities[0]
    );

    // Multi-target Graveyard -> Top of Library emits PutTargetOnTopOfLibrary (not GraveyardToTop).
    let gy_top_multi = read_w28a(
        "Name:FeastProxy\nManaCost:2 B\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Library | LibraryPosition$ 0 | ValidTgts$ Creature.YouOwn | TargetMin$ 0 | TargetMax$ X\n\
         SVar:X:Count$ValidGraveyard Creature.YouOwn",
    );
    assert!(
        gy_top_multi.abilities[0].contains("Effect::PutTargetOnTopOfLibrary")
            && !gy_top_multi.abilities[0].contains("Effect::GraveyardToTop"),
        "{}",
        gy_top_multi.abilities[0]
    );

    // Single-target Graveyard -> Top of Library preserves legacy Effect::GraveyardToTop.
    let gy_top_single = read_w28a(
        "Name:SingleGyTop\nManaCost:B\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Library | LibraryPosition$ 0 | ValidTgts$ Creature.YouOwn",
    );
    assert!(
        gy_top_single.abilities[0].contains("Effect::GraveyardToTop"),
        "{}",
        gy_top_single.abilities[0]
    );

    // Refusals: multi-player targets, 0..=X (Count$xPaid), inverted bounds, literal 255, multi Counter.
    for bad in [
        "Name:MultiPlayerRange\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ LoseLife | ValidTgts$ Player | TargetMin$ 1 | TargetMax$ 2 | LifeAmount$ 2",
        "Name:MultiPlayerExact\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Opponent | TargetMin$ 1 | TargetMax$ 2 | NumDmg$ 2",
        "Name:ZeroToX\nManaCost:X G\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Artifact | TargetMin$ 0 | TargetMax$ X\n\
         SVar:X:Count$xPaid",
        "Name:InvertedRange\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ Tap | ValidTgts$ Creature | TargetMin$ 3 | TargetMax$ 2",
        "Name:Literal255\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ Tap | ValidTgts$ Creature | TargetMin$ 1 | TargetMax$ 255",
        "Name:MultiCounter\nManaCost:U U R\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | TargetMin$ 1 | TargetMax$ 2",
    ] {
        assert!(refused_w28a(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn up_to_one_suspect_clone_and_activated_x_targets() {
    // Up-to-one AlterAttribute (Suspected) with redundant Defined$ Targeted.
    let suspect = read_w28a(
        "Name:Suspector\nManaCost:2 B\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigSuspect\n\
         SVar:TrigSuspect:DB$ AlterAttribute | ValidTgts$ Creature | Defined$ Targeted | TargetMin$ 0 | TargetMax$ 1 | Attributes$ Suspected | Activate$ True",
    );
    assert!(
        suspect.abilities[0].contains("Effect::Suspect")
            && suspect.abilities[0].contains("TargetReq::up_to_one("),
        "{}",
        suspect.abilities[0]
    );

    // Up-to-one Clone.
    let clone = read_w28a(
        "Name:ShapeshiftUpToOne\nManaCost:3 U\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | Execute$ TrigClone\n\
         SVar:TrigClone:DB$ Clone | ValidTgts$ Creature.Other | TargetMin$ 0 | TargetMax$ 1 | Duration$ UntilEndOfTurn",
    );
    assert!(
        clone.abilities[0].contains("Effect::BecomeCopy")
            && clone.abilities[0].contains("TargetReq::up_to_one("),
        "{}",
        clone.abilities[0]
    );

    // Activated ability with X in cost and TargetMin$ X | TargetMax$ X.
    let magus = read_w28a(
        "Name:CandelabraProxy\nManaCost:G\nTypes:Creature\nPT:1/2\n\
         A:AB$ Untap | Cost$ X T | ValidTgts$ Land | TargetMin$ X | TargetMax$ X\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        magus.abilities[0].contains("TargetReq::x_targets(TargetSpec::Object(&Filter::LAND))"),
        "{}",
        magus.abilities[0]
    );

    // Refusals: X-targets on activated without X cost; X-targets on loyalty ability;
    // up-to-one AlterAttribute on non-Suspected (Prepared).
    for bad in [
        "Name:ActivatedXNoCost\nManaCost:2 U\nTypes:Creature\nPT:1/1\n\
         A:AB$ Untap | Cost$ 2 T | ValidTgts$ Land | TargetMin$ X | TargetMax$ X\n\
         SVar:X:Count$xPaid",
        "Name:LoyaltyXTargets\nManaCost:3 U\nTypes:Planeswalker\nLoyalty:4\n\
         A:AB$ Untap | Cost$ SubCounter<X/LOYALTY> | Planeswalker$ True | ValidTgts$ Land | TargetMin$ X | TargetMax$ X\n\
         SVar:X:Count$xPaid",
        "Name:UpToOnePrepared\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ AlterAttribute | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ 1 | Attributes$ Prepared | Activate$ True",
    ] {
        assert!(refused_w28a(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn divided_damage_spells_activated_loyalty_modes_and_object_triggers() {
    // Spell with TargetMin$ 1 | TargetMax$ 2.
    let forked = read_w28a(
        "Name:TwinSpark\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 2 | TargetMin$ 1 | TargetMax$ 2 | DividedAsYouChoose$ 2",
    );
    assert!(
        forked.abilities[0]
            .contains("Effect::DealDamageDivided { amount: DivisionAmount::Fixed(2) }")
            && forked.abilities[0]
                .contains("TargetReq { min: 1, ..TargetReq::up_to(TargetSpec::AnyTarget, 2) }"),
        "{}",
        forked.abilities[0]
    );

    // Spell with divided damage + Slot-2 exact multi-target continuation (Magma Opus shape).
    let opus = read_w28a(
        "Name:OpusProxy\nManaCost:6 U R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 4 | TargetMin$ 0 | TargetMax$ 4 | DividedAsYouChoose$ 4 | SubAbility$ DBTap\n\
         SVar:DBTap:DB$ Tap | TargetMin$ 2 | TargetMax$ 2 | ValidTgts$ Permanent | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | NumCards$ 2",
    );
    assert!(
        opus.abilities[0]
            .contains("Effect::DealDamageDivided { amount: DivisionAmount::Fixed(4) }")
            && opus.abilities[0].contains("Effect::WithSecondTarget(&[Effect::TapTarget])")
            && opus.abilities[0].contains(
                "second_targets = Some(TargetReq::exactly(TargetSpec::Object(&Filter::Any), 2))"
            ),
        "{}",
        opus.abilities[0]
    );

    // Activated ability with DividedAsYouChoose$.
    let hellkite = read_w28a(
        "Name:HellkiteProxy\nManaCost:3 R R\nTypes:Creature\nPT:4/4\n\
         A:AB$ DealDamage | Cost$ 3 R | ValidTgts$ Any | NumDmg$ 2 | TargetMin$ 1 | TargetMax$ 2 | DividedAsYouChoose$ 2",
    );
    assert!(
        hellkite.abilities[0]
            .contains("Effect::DealDamageDivided { amount: DivisionAmount::Fixed(2) }")
            && hellkite.abilities[0]
                .contains("TargetReq { min: 1, ..TargetReq::up_to(TargetSpec::AnyTarget, 2) }"),
        "{}",
        hellkite.abilities[0]
    );

    // Loyalty ability with DividedAsYouChoose$.
    let walker = read_w28a(
        "Name:FlameWalker\nManaCost:4 R R\nTypes:Planeswalker\nLoyalty:5\n\
         A:AB$ DealDamage | Cost$ SubCounter<4/LOYALTY> | Planeswalker$ True | NumDmg$ 6 | TargetMin$ 0 | TargetMax$ 6 | DividedAsYouChoose$ 6 | ValidTgts$ Creature,Planeswalker",
    );
    assert!(
        walker.abilities[0].contains(
            "loyalty!(-4, &[Effect::DealDamageDivided { amount: DivisionAmount::Fixed(6) }]"
        ),
        "{}",
        walker.abilities[0]
    );

    // Triggered ability with object-only DividedAsYouChoose$ (Dragonlord Atarka shape).
    let atarka = read_w28a(
        "Name:AtarkaProxy\nManaCost:5 R G\nTypes:Creature\nPT:8/8\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDmg\n\
         SVar:TrigDmg:DB$ DealDamage | ValidTgts$ Creature.OppCtrl,Planeswalker.OppCtrl | NumDmg$ 5 | TargetMin$ 0 | TargetMax$ 5 | DividedAsYouChoose$ 5",
    );
    assert!(
        atarka.abilities[0]
            .contains("Effect::DealDamageDivided { amount: DivisionAmount::Fixed(5) }")
            && atarka.abilities[0].contains("TargetReq::up_to(TargetSpec::Object("),
        "{}",
        atarka.abilities[0]
    );

    // Refusals: triggered divided damage including players (ask_trigger_division ignores target_players!),
    // optional triggered divided damage, TargetMin > 1, player-only divided damage.
    for bad in [
        "Name:BogardanProxy\nManaCost:6 R R\nTypes:Creature\nPT:5/5\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDmg\n\
         SVar:TrigDmg:DB$ DealDamage | ValidTgts$ Any | NumDmg$ 5 | TargetMin$ 0 | TargetMax$ 5 | DividedAsYouChoose$ 5",
        "Name:OptionalTriggerDiv\nManaCost:4 R\nTypes:Creature\nPT:4/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | OptionalDecider$ You | Execute$ TrigDmg\n\
         SVar:TrigDmg:DB$ DealDamage | ValidTgts$ Creature | NumDmg$ 3 | TargetMin$ 0 | TargetMax$ 3 | DividedAsYouChoose$ 3",
        "Name:MinTwoDiv\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 4 | TargetMin$ 2 | TargetMax$ 4 | DividedAsYouChoose$ 4",
        "Name:MaxExceedsTotal\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 2 | TargetMin$ 1 | TargetMax$ 3 | DividedAsYouChoose$ 2",
    ] {
        assert!(refused_w28a(bad), "must refuse:\n{bad}");
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn change_zone_self_dies_shuffle_hand_library_targeted_exile_and_gaps() {
    // Redundant Defined$ Targeted on targeted ChangeZone.
    let bounce = read_w28a(
        "Name:RedundantDefinedBounce\nManaCost:U\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Permanent.nonLand | Defined$ Targeted | Origin$ Battlefield | Destination$ Hand",
    );
    assert!(
        bounce.abilities[0].contains("Effect::bounce("),
        "{}",
        bounce.abilities[0]
    );

    // Self-dies shuffle into owner's library (explicit Origin$ Graveyard and omitted Origin$ + Scry).
    let alabaster = read_w28a(
        "Name:AlabasterProxy\nManaCost:4 W W\nTypes:Creature\nPT:4/4\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigShuf\n\
         SVar:TrigShuf:DB$ ChangeZone | Origin$ Graveyard | Destination$ Library | Shuffle$ True | Defined$ TriggeredNewCardLKICopy",
    );
    assert!(
        alabaster.abilities[0].contains(
            "Effect::ShuffleGraveyardMatchingIntoLibrary { who: PlayerRel::OwnerOfSource, filter: &Filter::This }"
        ),
        "{}",
        alabaster.abilities[0]
    );

    let cavalier = read_w28a(
        "Name:CavalierProxy\nManaCost:2 U U U\nTypes:Creature\nPT:5/5\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigShuf\n\
         SVar:TrigShuf:DB$ ChangeZone | Destination$ Library | Shuffle$ True | Defined$ TriggeredNewCardLKICopy | SubAbility$ DBScry\n\
         SVar:DBScry:DB$ Scry | ScryNum$ 2\n\
         Oracle:When CavalierProxy dies, shuffle it into its owner's library, then scry 2.",
    );
    assert!(
        cavalier.abilities[0].contains("Effect::ShuffleGraveyardMatchingIntoLibrary")
            && cavalier.abilities[0].contains("Effect::scry(2)"),
        "{}",
        cavalier.abilities[0]
    );

    // Omitted LibraryPosition$ in choose_from_your_graveyard defaults to top (0).
    let glowspore = read_w28a(
        "Name:GlowsporeProxy\nManaCost:B G\nTypes:Creature\nPT:3/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigTop\n\
         SVar:TrigTop:DB$ ChangeZone | Origin$ Graveyard | Destination$ Library | ChangeType$ Land.YouOwn | Hidden$ True | Optional$ True",
    );
    assert!(
        glowspore.abilities[0].contains("SearchDest::TopOfLibrary"),
        "{}",
        glowspore.abilities[0]
    );

    // Hand -> Library with Shuffle$ True (See Beyond & Credit Voucher shapes).
    let see_beyond = read_w28a(
        "Name:SeeBeyondProxy\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ 2 | SubAbility$ DBShuffle\n\
         SVar:DBShuffle:DB$ ChangeZone | Origin$ Hand | Destination$ Library | ChangeNum$ 1 | Shuffle$ True | Mandatory$ True",
    );
    assert!(
        see_beyond.abilities[0].contains("ChooseMove::TopOfLibrary")
            && see_beyond.abilities[0].contains("Effect::ShuffleLibrary { who: PlayerRel::You }"),
        "{}",
        see_beyond.abilities[0]
    );

    let voucher = read_w28a(
        "Name:VoucherProxy\nManaCost:2\nTypes:Artifact\n\
         A:AB$ ChangeZone | Cost$ T Sac<1/CARDNAME> | Origin$ Hand | Destination$ Library | ChangeType$ Card.YouOwn | ChangeNum$ X | Hidden$ True | Shuffle$ True | RememberChanged$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ Y | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$ValidHand Card.YouOwn\n\
         SVar:Y:Remembered$Amount\n\
         Oracle:{T}, Sacrifice VoucherProxy: Shuffle any number of cards from your hand into your library, then draw that many cards.",
    );
    assert!(
        voucher.abilities[0].contains("Effect::CaptureAffected")
            && voucher.abilities[0].contains("ChooseMove::TopOfLibrary")
            && voucher.abilities[0].contains("Effect::ShuffleLibrary { who: PlayerRel::You }")
            && voucher.abilities[0].contains("Amount::RememberedCount"),
        "{}",
        voucher.abilities[0]
    );

    // Target opponent exiles a permanent they control (Debt to the Kami / Doomfall shape).
    let debt = read_w28a(
        "Name:DebtProxy\nManaCost:2 B\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Exile | ValidTgts$ Opponent | DefinedPlayer$ Targeted | Chooser$ Targeted | Mandatory$ True | Hidden$ True | IsCurse$ True | ChangeType$ Creature | ChangeNum$ 1",
    );
    assert!(
        debt.abilities[0].contains("chooser: PlayerRel::Chosen")
            && debt.abilities[0].contains("movement: ChooseMove::Exile")
            && debt
                .statics
                .contains("Filter::ControlledBy(PlayerRel::Chosen)"),
        "{}\n{}",
        debt.statics,
        debt.abilities[0]
    );

    // Self-exile via ChangeType$ Card.Self (Silent Gravestone shape) & optional self-bounce.
    let gravestone = read_w28a(
        "Name:GravestoneProxy\nManaCost:1\nTypes:Artifact\n\
         A:AB$ ChangeZone | Cost$ 4 T | Origin$ Battlefield | Destination$ Exile | ChangeType$ Card.Self",
    );
    assert!(
        gravestone.abilities[0].contains("Effect::ExileSource"),
        "{}",
        gravestone.abilities[0]
    );

    let self_opt_bounce = read_w28a(
        "Name:OptionalBounceSelf\nManaCost:1 U\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | Execute$ TrigBounce\n\
         SVar:TrigBounce:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand | Defined$ Self | Optional$ True",
    );
    assert!(
        self_opt_bounce.abilities[0]
            .contains("Effect::MayDo { effects: &[Effect::bounce(TargetSpec::ThisObject)] }"),
        "{}",
        self_opt_bounce.abilities[0]
    );

    // Conditional library search destination (Loyal Inventor shape).
    let inventor = read_w28a(
        "Name:InventorProxy\nManaCost:2 U\nTypes:Creature\nPT:2/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigSearch\n\
         SVar:TrigSearch:DB$ ChangeZone | Origin$ Library | Destination$ Library | ChangeType$ Artifact | ChangeNum$ 1 | RememberChanged$ True | SubAbility$ DBHand\n\
         SVar:DBHand:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Hand | ConditionCheckSVar$ X | ConditionSVarCompare$ GE1 | SubAbility$ DBTop\n\
         SVar:DBTop:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Library | LibraryPosition$ 0 | ConditionCheckSVar$ X | ConditionSVarCompare$ LE0 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$Valid Artifact.YouCtrl",
    );
    assert!(
        inventor.abilities[0].contains("Effect::IfCondition")
            && inventor.abilities[0].contains("Find::HAND")
            && inventor.abilities[0].contains("Find::TOP_OF_LIBRARY"),
        "{}",
        inventor.abilities[0]
    );

    // Refusals: Stack -> Library Shuffle, Battlefield -> Library Shuffle,
    // Hand,Graveyard multi-origin, non-dies trigger self-shuffle.
    for bad in [
        "Name:BeaconProxy\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ 1 | SubAbility$ DBShuf\n\
         SVar:DBShuf:DB$ ChangeZone | Defined$ Self | Origin$ Stack | Destination$ Library | Shuffle$ True",
        "Name:SphinxProxy\nManaCost:4 U U\nTypes:Creature\nPT:5/5\n\
         A:AB$ ChangeZone | Cost$ U | Origin$ Battlefield | Destination$ Library | Shuffle$ True",
        "Name:MultiOrigin\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Hand,Graveyard | Destination$ Battlefield | ChangeType$ Land.YouOwn | ChangeNum$ 1 | Hidden$ True",
        "Name:NonDiesShuffle\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigShuf\n\
         SVar:TrigShuf:DB$ ChangeZone | Destination$ Library | Shuffle$ True | Defined$ TriggeredNewCardLKICopy\n\
         Oracle:When NonDiesShuffle enters, shuffle it into its owner's library.",
    ] {
        assert!(refused_w28a(bad), "must refuse:\n{bad}");
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn capture_affected_remembered_extensions_and_soundness_boundaries() {
    // Full Animate | Defined$ Remembered (Types, Colors, P/T, Keywords, LeaveBattlefield$ Exile, AtEOT$ YourExile).
    let encore = read_w28a(
        "Name:EncoreProxy\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Creature.OppOwn | GainControl$ True | RememberChanged$ True | SubAbility$ DBAnim\n\
         SVar:DBAnim:DB$ Animate | Defined$ Remembered | Keywords$ Haste | LeaveBattlefield$ Exile | AtEOT$ YourExile | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        encore.abilities[0].contains("Modifier::AddKeyword(KeywordSet::HASTE)")
            && encore.abilities[0].contains("Modifier::ExileInsteadOfLeavingBattlefield")
            && encore.abilities[0].contains("whose: Some(PlayerRel::You)"),
        "{}",
        encore.abilities[0]
    );

    let animate_land = read_w28a(
        "Name:EmergentProxy\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Battlefield | Tapped$ True | ChangeType$ Land.Basic | ChangeNum$ 1 | RememberChanged$ True | SubAbility$ DBAnim\n\
         SVar:DBAnim:DB$ Animate | Defined$ Remembered | Power$ 0 | Toughness$ 0 | Types$ Creature,Fractal | Colors$ Green,Blue | OverwriteColors$ True | Duration$ Permanent | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        animate_land.abilities[0].contains("Modifier::AddType(TypeSet::CREATURE)")
            && animate_land.abilities[0].contains("Modifier::SetPT(0, 0)")
            && animate_land.abilities[0].contains("Modifier::SetColor(")
            && animate_land.abilities[0].contains("Duration::Indefinitely"),
        "{}",
        animate_land.abilities[0]
    );

    // Leading Cleanup | ClearRemembered$ True + Attach | Defined$ Remembered (Deathrender shape).
    let deathrender = read_w28a(
        "Name:DeathrenderProxy\nManaCost:4\nTypes:Artifact Equipment\n\
         K:Equip:2\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.AttachedBy | Execute$ Clean | OptionalDecider$ You\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True | SubAbility$ Put\n\
         SVar:Put:DB$ ChangeZone | Origin$ Hand | Destination$ Battlefield | ChangeType$ Creature | ChangeNum$ 1 | RememberChanged$ True | SubAbility$ AttachIt\n\
         SVar:AttachIt:DB$ Attach | Defined$ Remembered\n\
         Oracle:Whenever equipped creature dies, you may put a creature card from your hand onto the battlefield and attach DeathrenderProxy to it.",
    );
    assert!(
        deathrender.abilities[1].contains("Effect::CaptureAffected")
            && deathrender.abilities[1]
                .contains("Effect::AttachSelf { target: TargetSpec::EventObject }"),
        "{:?}",
        deathrender.abilities
    );

    // ConditionCheckSVar$ over Remembered$Amount and Remembered$Valid <F>.
    let mistbreath = read_w28a(
        "Name:MistbreathProxy\nManaCost:G\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | Execute$ TrigBounce\n\
         SVar:TrigBounce:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand | Defined$ Self | Optional$ True | RememberChanged$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | ConditionCheckSVar$ X | ConditionSVarCompare$ GE1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        mistbreath.abilities[0]
            .contains("Effect::MayDo { effects: &[Effect::bounce(TargetSpec::ThisObject)] }")
            && mistbreath.abilities[0].contains("Effect::IfCondition"),
        "{}",
        mistbreath.abilities[0]
    );

    let rem_svar_count = read_w28a(
        "Name:RememberedCountProxy\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Battlefield | ChangeType$ Land.Basic | ChangeNum$ 1 | RememberChanged$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | ConditionCheckSVar$ X | ConditionSVarCompare$ GE1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        rem_svar_count.abilities[0].contains("left: &Amount::RememberedCount")
            && rem_svar_count.abilities[0].contains("op: AmountCmp::AtLeast"),
        "{}",
        rem_svar_count.abilities[0]
    );

    let oriq = read_w28a(
        "Name:OriqProxy\nManaCost:2 B B\nTypes:Creature\nPT:3/3\n\
         A:AB$ ChangeZone | Cost$ T | Origin$ Library | Destination$ Graveyard | ChangeType$ Card | ChangeNum$ 1 | RememberChanged$ True | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1 | ConditionCheckSVar$ X | ConditionSVarCompare$ GE1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Valid Card.Instant,Card.Sorcery",
    );
    assert!(
        oriq.abilities[0].contains("Effect::ForEachRemembered")
            && oriq.abilities[0].contains("Effect::AddCounter { kind: CounterKind::P1P1"),
        "{}",
        oriq.abilities[0]
    );

    // CopyPermanent | Defined$ Remembered after Hand -> Exile (Nexus of Becoming shape)
    // and mandatory hidden Graveyard -> Exile (Mordor on the March shape).
    let nexus = read_w28a(
        "Name:NexusProxy\nManaCost:6\nTypes:Artifact\n\
         T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | Origin$ Hand | Destination$ Exile | ChangeType$ Creature | ChangeNum$ 1 | Optional$ True | RememberChanged$ True | SubAbility$ DBCopy\n\
         SVar:DBCopy:DB$ CopyPermanent | Defined$ Remembered | SetPower$ 3 | SetToughness$ 3 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        nexus.abilities[0].contains("Effect::CaptureAffected")
            && nexus.abilities[0].contains("Effect::ForEachRemembered")
            && nexus.abilities[0]
                .contains("Effect::CreateTokenCopy { target: TargetSpec::EventObject"),
        "{}",
        nexus.abilities[0]
    );

    let mordor = read_w28a(
        "Name:MordorProxy\nManaCost:3 B R\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Exile | Hidden$ True | ChangeType$ Creature.YouCtrl | ChangeNum$ 1 | RememberChanged$ True | SubAbility$ DBCopy\n\
         SVar:DBCopy:DB$ CopyPermanent | Defined$ Remembered | PumpKeywords$ Haste | PumpDuration$ EOT | AtEOT$ Exile | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:Exile a creature card from your graveyard. Create a token that's a copy of it. It gains haste until end of turn. Exile it at the beginning of the next end step.",
    );
    assert!(
        mordor.abilities[0].contains("optional: false")
            && mordor.abilities[0].contains("Effect::CreateTokenCopy")
            && mordor.abilities[0].contains("cleanup: CopyCleanup::ExileAtNextEndStep"),
        "{}",
        mordor.abilities[0]
    );

    // RememberTargets$ True on mandatory targeted ChangeZone (Cemetery Recruitment shape).
    let recruitment = read_w28a(
        "Name:RecruitmentProxy\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Creature.YouOwn | RememberTargets$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | ConditionDefined$ Remembered | ConditionPresent$ Card.Zombie | ConditionCompare$ GE1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:Return target creature card from your graveyard to your hand. If it's a Zombie card, draw a card.",
    );
    assert!(
        recruitment.abilities[0].contains("Effect::CaptureAffected")
            && recruitment.abilities[0].contains("Effect::ForEachRemembered"),
        "{}",
        recruitment.abilities[0]
    );

    // Refusals: Battlefield -> Exile -> CopyPermanent (LKI soundness),
    // RememberTargets$ True when Oracle says "you may" without Optional$,
    // persistent RememberChanged on permanent without Cleanup.
    for bad in [
        "Name:BadBfCopy\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         A:AB$ ChangeZone | Cost$ T | Origin$ Battlefield | Destination$ Exile | ValidTgts$ Creature.Other+YouCtrl | RememberChanged$ True | SubAbility$ DBCopy\n\
         SVar:DBCopy:DB$ CopyPermanent | Defined$ Remembered | SetPower$ 1 | SetToughness$ 1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
        "Name:BadMayRememberTargets\nManaCost:1 B B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Library | ValidTgts$ Creature.YouOwn | RememberTargets$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | ConditionCheckSVar$ Y | ConditionSVarCompare$ GT0 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:Y:Remembered$Amount\n\
         Oracle:You may put target creature card from your graveyard on top of your library. If you do, draw a card.",
        "Name:PersistentRemember\nManaCost:3 W\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigSearch\n\
         SVar:TrigSearch:DB$ ChangeZone | Origin$ Library | Destination$ Battlefield | ChangeType$ Land.Basic | ChangeNum$ 1 | RememberChanged$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | ConditionCheckSVar$ X | ConditionSVarCompare$ GE1\n\
         SVar:X:Remembered$Amount",
        "Name:PersistentAttach\nManaCost:4\nTypes:Artifact Equipment\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.EquippedBy | Execute$ TrigPut\n\
         SVar:TrigPut:DB$ ChangeZone | Origin$ Hand | Destination$ Battlefield | ChangeType$ Creature | ChangeNum$ 1 | Optional$ True | RememberChanged$ True | SubAbility$ DBAttach\n\
         SVar:DBAttach:DB$ Attach | Defined$ Remembered",
    ] {
        assert!(refused_w28a(bad), "must refuse:\n{bad}");
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave28a_p2_reference_filters_any_exclusions_and_emblem_subsumption() {
    // NotDefinedTargeted -> Filter::Not(&Filter::Reference) (Mirrorweave & Flames of the Raze-Boar).
    let mirror = read_w28a(
        "Name:MirrorweaveProxy\nManaCost:2 WU WU\nTypes:Instant\n\
         A:SP$ Clone | Defined$ Targeted | CloneTarget$ Valid Creature.NotDefinedTargeted | Duration$ UntilEndOfTurn | ValidTgts$ Creature.nonLegendary",
    );
    assert!(
        mirror.abilities[0].contains("Effect::BecomeCopy")
            && mirror.statics.contains("Filter::Not(&Filter::Reference)"),
        "{:?}\n{}",
        mirror.abilities,
        mirror.statics
    );

    let raze_boar = read_w28a(
        "Name:FlamesProxy\nManaCost:5 R\nTypes:Instant\n\
         A:SP$ DealDamage | NumDmg$ 4 | ValidTgts$ Creature.OppCtrl | SubAbility$ DBDealDamage\n\
         SVar:DBDealDamage:DB$ DamageAll | NumDmg$ 2 | ValidCards$ Creature.NotDefinedTargeted+ControlledBy TargetedController | ConditionPresent$ Creature.powerGE4+YouCtrl",
    );
    assert!(
        raze_boar.abilities[0].contains("Effect::DealDamageEach")
            && raze_boar
                .statics
                .contains("Filter::Not(&Filter::Reference)")
            && raze_boar
                .statics
                .contains("Filter::ControlledBy(PlayerRel::ControllerOfTarget)"),
        "{:?}\n{}",
        raze_boar.abilities,
        raze_boar.statics
    );

    // !targetedBy -> Filter::Not(&Filter::Reference) (Public Execution).
    let pub_exec = read_w28a(
        "Name:PublicExecutionProxy\nManaCost:5 B\nTypes:Instant\n\
         A:SP$ Destroy | ValidTgts$ Creature.OppCtrl | SubAbility$ WeakenOther\n\
         SVar:WeakenOther:DB$ PumpAll | ValidCards$ Creature.ControlledBy TargetedController+!targetedBy | NumAtt$ -2 | IsCurse$ True",
    );
    assert!(
        pub_exec.abilities[0].contains("Effect::PumpFilter")
            && pub_exec.statics.contains("Filter::Not(&Filter::Reference)"),
        "{:?}\n{}",
        pub_exec.abilities,
        pub_exec.statics
    );

    // !TriggeredAttacker on Attacks trigger -> Filter::Not(&Filter::Reference) (Sigil of Valor).
    let sigil = read_w28a(
        "Name:SigilProxy\nManaCost:2\nTypes:Artifact Equipment\nK:Equip:1\n\
         T:Mode$ Attacks | ValidCard$ Card.EquippedBy | Alone$ True | Execute$ TrigPump\n\
         SVar:TrigPump:DB$ Pump | Defined$ TriggeredAttackerLKICopy | NumAtt$ +X | NumDef$ +X\n\
         SVar:X:Count$Valid Creature.YouCtrl+!TriggeredAttacker",
    );
    assert!(
        sigil.abilities[0].contains("Filter::Not(&Filter::Reference)")
            || sigil.statics.contains("Filter::Not(&Filter::Reference)"),
        "{:?}\n{}",
        sigil.abilities,
        sigil.statics
    );

    // ValidTgts$ Any.<strictly-object-exclusion> -> TargetSpec::PlayerOrObject (Wrathful Raptors, Lozhan).
    let raptors = read_w28a(
        "Name:WrathfulRaptorsProxy\nManaCost:4 R\nTypes:Creature Dinosaur\nPT:5/5\n\
         T:Mode$ DamageDoneOnce | Execute$ TrigDamage | ValidTarget$ Dinosaur.YouCtrl | TriggerZones$ Battlefield\n\
         SVar:TrigDamage:DB$ DealDamage | NumDmg$ X | DamageSource$ TriggeredTarget | ValidTgts$ Any.!Dinosaur\n\
         SVar:X:TriggerCount$DamageAmount",
    );
    assert!(
        raptors.abilities[0].contains("TargetSpec::PlayerOrObject(")
            && raptors
                .statics
                .contains("Filter::Not(&Filter::HasSubtype(subtypes::creature::DINOSAUR))"),
        "{:?}\n{}",
        raptors.abilities,
        raptors.statics
    );

    let lozhan = read_w28a(
        "Name:LozhanProxy\nManaCost:3 U R\nTypes:Legendary Creature Dragon\nPT:4/2\n\
         T:Mode$ SpellCast | ValidCard$ Dragon | ValidActivatingPlayer$ You | Execute$ TrigDamage | TriggerZones$ Battlefield\n\
         SVar:TrigDamage:DB$ DealDamage | ValidTgts$ Any.!IsCommander | NumDmg$ X\n\
         SVar:X:TriggeredSpellAbility$CardManaCostLKI",
    );
    assert!(
        lozhan.abilities[0].contains("TargetSpec::PlayerOrObject(")
            && lozhan.statics.contains("Filter::Not(&Filter::IsCommander)"),
        "{:?}\n{}",
        lozhan.abilities,
        lozhan.statics
    );

    // ControlledBy Player.Other -> Filter::Not(&Filter::ControlledByYou) (Dog Umbra).
    let dog_umbra = read_w28a(
        "Name:DogUmbraProxy\nManaCost:1 W\nTypes:Enchantment Aura\nK:Flash\nK:Enchant:Creature\n\
         S:Mode$ CantAttack,CantBlock | ValidCard$ Creature.EnchantedBy+ControlledBy Player.Other",
    );
    assert!(
        dog_umbra
            .statics
            .contains("Filter::Not(&Filter::ControlledByYou)")
            || dog_umbra
                .abilities
                .iter()
                .any(|a| a.contains("Filter::Not(&Filter::ControlledByYou)")),
        "{:?}\n{}",
        dog_umbra.abilities,
        dog_umbra.statics
    );

    // Emblem subsumption for non-<Subtype> and Other+YouCtrl (Repel the Abominable, Talon of Pain).
    let repel = read_w28a(
        "Name:RepelProxy\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Effect | ReplacementEffects$ RPrevent\n\
         SVar:RPrevent:Event$ DamageDone | Prevent$ True | ValidSource$ Card.nonHuman,Emblem | ActiveZones$ Command",
    );
    assert!(
        repel
            .statics
            .contains("Filter::Not(&Filter::HasSubtype(subtypes::creature::HUMAN))"),
        "{:?}\n{}",
        repel.abilities,
        repel.statics
    );

    let talon = read_w28a(
        "Name:TalonProxy\nManaCost:4\nTypes:Artifact\n\
         T:Mode$ DamageDone | ValidSource$ Card.Other+YouCtrl,Emblem.YouCtrl | ValidTarget$ Opponent | TriggerZones$ Battlefield | Execute$ PutC\n\
         SVar:PutC:DB$ PutCounter | Defined$ Self | CounterType$ CHARGE | CounterNum$ 1",
    );
    assert!(
        talon.statics.contains("Filter::Another")
            && talon.statics.contains("Filter::ControlledByYou"),
        "{:?}\n{}",
        talon.abilities,
        talon.statics
    );

    // Refusals: player-applicable Any.<pred> (Needle Drop), cast-time NotDefinedTargeted / !targetedBy /
    // !TriggeredAttacker targets, blockingEquipped outside AttackerBlocked (Plasma Caster),
    // Emblem alone or mismatched controller, !wasCastFromYourHand on ReduceCost (CastFromHand checks
    // any hand, not specifically your hand).
    for bad in [
        "Name:NeedleDropProxy\nManaCost:R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Any.wasDealtDamageThisTurn | NumDmg$ 1",
        "Name:BadAnyCtrl\nManaCost:R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Any.OppCtrl | NumDmg$ 2",
        "Name:BadCastTargetNotDefined\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature.NotDefinedTargeted | NumDmg$ 2",
        "Name:BadCastTargetNotTargetedBy\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature.!targetedBy | NumDmg$ 2",
        "Name:BadTargetNotAttacker\nManaCost:2 W\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         T:Mode$ AttackerUnblocked | ValidCard$ Creature.EnchantedBy | Execute$ D\n\
         SVar:D:DB$ DealDamage | ValidTgts$ Creature.!TriggeredAttacker | NumDmg$ 2",
        "Name:BadBlockingEquippedActivated\nManaCost:1 R\nTypes:Artifact Equipment\nK:Equip:2\n\
         A:AB$ DealDamage | Cost$ 1 | ValidTgts$ Creature.blockingEquipped | NumDmg$ 1",
        "Name:BadEmblemOtherOnEmblem\nTypes:Emblem\n\
         T:Mode$ DamageDone | ValidSource$ Card.Other+YouCtrl,Emblem.YouCtrl | ValidTarget$ Opponent | TriggerZones$ Command | Execute$ D\n\
         SVar:D:DB$ GainLife | LifeAmount$ 1",
        "Name:SageReduceCostRefused\nManaCost:5 U U\nTypes:Creature\nPT:5/5\n\
         S:Mode$ ReduceCost | ValidCard$ Card.!wasCastFromYourHand | Type$ Spell | Activator$ You | Amount$ 2\n\
         Oracle:Spells you cast from anywhere other than your hand cost {2} less to cast.",
    ] {
        assert!(refused_w28a(bad), "must refuse:\n{bad}");
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave28a_p2_dynamic_filter_amounts_and_remembered_play_animate() {
    // TriggeredPlayersDefenders$Amount -> Amount::OpponentsYouAttackedThisCombat (Custodi Soulcaller).
    let custodi = read_w28a(
        "Name:CustodiProxy\nManaCost:1 W W\nTypes:Creature\nPT:1/2\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigSoulcall\n\
         SVar:TrigSoulcall:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Creature.YouCtrl+cmcLEX\n\
         SVar:X:TriggeredPlayersDefenders$Amount",
    );
    assert!(
        custodi
            .statics
            .contains("Filter::CmcAtMostAmount(&Amount::OpponentsYouAttackedThisCombat)"),
        "{:?}\n{}",
        custodi.abilities,
        custodi.statics
    );

    // SVar$Y/Plus.1 + Count$Valid ...$GreatestCardManaCost in svar_to_filter_amount.
    let greatness = read_w28a(
        "Name:GreatnessProxy\nManaCost:G G\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigPlay\n\
         SVar:TrigPlay:DB$ Play | Valid$ Permanent.YouOwn | ValidZone$ Hand | ValidSA$ Spell.cmcEQX | WithoutManaCost$ True | Amount$ 1 | Optional$ True\n\
         SVar:X:SVar$Y/Plus.1\n\
         SVar:Y:Count$Valid Permanent.YouCtrl+Other$GreatestCardManaCost",
    );
    assert!(
        greatness.statics.contains(
            "Filter::CmcEqualsAmount(&Amount::Plus { base: &Amount::GreatestManaValueAmong("
        ),
        "{:?}\n{}",
        greatness.abilities,
        greatness.statics
    );

    // TriggeredCardOwner on Drawn -> PlayerRel::EventPlayer (Spiteful Visions).
    let spiteful = read_w28a(
        "Name:SpitefulProxy\nManaCost:2 BR BR\nTypes:Enchantment\n\
         T:Mode$ Drawn | ValidCard$ Card | TriggerZones$ Battlefield | Execute$ TrigDmg\n\
         SVar:TrigDmg:DB$ DealDamage | Defined$ TriggeredCardOwner | NumDmg$ 1",
    );
    assert!(
        spiteful.abilities[0].contains("TargetSpec::Player(PlayerRel::EventPlayer)"),
        "{:?}",
        spiteful.abilities
    );

    // Single-object CaptureAffected -> Play | Defined$ Remembered (Bring to Light).
    let bring = read_w28a(
        "Name:BringToLightProxy\nManaCost:3 G U\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Exile | ChangeType$ Creature.cmcLEX,Instant.cmcLEX,Sorcery.cmcLEX | ChangeNum$ 1 | RememberChanged$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | ValidSA$ Spell | WithoutManaCost$ True | Optional$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$Converge",
    );
    assert!(
        bring.abilities[0].contains("Effect::CaptureAffected")
            && bring.abilities[0].contains("Effect::ForEachRemembered")
            && bring.abilities[0].contains("PlaySource::Subject(TargetSpec::EventObject)")
            && bring.abilities[0].contains("PlayCostMode::WithoutPayingManaCost"),
        "{:?}",
        bring.abilities
    );

    // Animate | Defined$ Remembered with Forge AI-only sVars$ MustAttack hint (Whip of Erebos).
    let whip = read_w28a(
        "Name:WhipProxy\nManaCost:2 B B\nTypes:Legendary Enchantment Artifact\n\
         A:AB$ ChangeZone | Cost$ 2 B B T | ValidTgts$ Creature.YouOwn | Origin$ Graveyard | Destination$ Battlefield | SorcerySpeed$ True | RememberChanged$ True | SubAbility$ DBUnearthed\n\
         SVar:DBUnearthed:DB$ Animate | Defined$ Remembered | Keywords$ Haste | LeaveBattlefield$ Exile | sVars$ WhipMustAttack | Duration$ Permanent | AtEOT$ Exile | SubAbility$ DBCleanup\n\
         SVar:WhipMustAttack:SVar:MustAttack:True\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        whip.abilities[0].contains("Effect::CaptureAffected")
            && whip.abilities[0].contains("Modifier::AddKeyword(KeywordSet::HASTE)")
            && whip.abilities[0].contains("Modifier::ExileInsteadOfLeavingBattlefield"),
        "{:?}",
        whip.abilities
    );

    // Refusals: LastDieRoll in dynamic filter (eval::amounts returns 0),
    // Play | Defined$ Remembered with AllowRepeats (Mnemonic Deluge) or non-AI sVars$.
    for bad in [
        "Name:BadDieRollFilter\nManaCost:1 G\nTypes:Artifact\n\
         A:AB$ RollDice | Cost$ 4 G T | Defined$ You | Sides$ 8 | ResultSVar$ X | SubAbility$ DBReveal\n\
         SVar:DBReveal:DB$ DigUntil | Defined$ You | Valid$ Creature.cmcEQX | FoundDestination$ Battlefield | RevealedDestination$ Library | RevealedLibraryPosition$ -1 | RevealRandomOrder$ True",
        "Name:BadMultiRepeatPlay\nManaCost:6 U U U\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Instant,Sorcery | Origin$ Graveyard | Destination$ Exile | RememberChanged$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | CopyCard$ True | Amount$ 3 | AllowRepeats$ True | Controller$ You | Optional$ True | WithoutManaCost$ True | ValidSA$ Spell | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
        "Name:BadNonAiSVars\nManaCost:2 B B\nTypes:Artifact\n\
         A:AB$ ChangeZone | Cost$ 2 B B T | ValidTgts$ Creature.YouOwn | Origin$ Graveyard | Destination$ Battlefield | RememberChanged$ True | SubAbility$ DBAnim\n\
         SVar:DBAnim:DB$ Animate | Defined$ Remembered | Keywords$ Haste | sVars$ RealRule | Duration$ Permanent | SubAbility$ DBCleanup\n\
         SVar:RealRule:SVar:X:Count$Valid Creature\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    ] {
        assert!(refused_w28a(bad), "must refuse:\n{bad}");
    }
}
