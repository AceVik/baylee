//! Reading block pairs, damage triggers, "unless", charms, conditions and
//! counters.

use super::*;

/// "Whenever an opponent casts a spell, you may draw a card unless that
/// player pays {1}" (Rhystic Study): on a cast trigger `TriggeredActivator`
/// is the caster (`EventPlayer`), as the payer and as `Defined$`, and
/// `TriggeredCardController` the spell's controller (`ControllerOfEvent`).
/// On any other trigger the same words name nobody this reader can see.
#[test]
fn a_cast_triggers_that_player_is_the_one_who_cast() {
    let study = read(
        "Name:X\nTypes:Enchantment\n\
         T:Mode$ SpellCast | ValidCard$ Card | ValidActivatingPlayer$ Opponent | \
         TriggerZones$ Battlefield | Execute$ D\n\
         SVar:D:DB$ Draw | Defined$ You | UnlessCost$ 1 | UnlessPayer$ TriggeredActivator | \
         NumCards$ 1",
    );
    let a = study.abilities.join("");
    assert!(a.contains("player: PlayerRel::EventPlayer"), "{a}");
    let ping = read(
        "Name:X\nTypes:Enchantment\n\
         T:Mode$ SpellCast | ValidCard$ Instant | TriggerZones$ Battlefield | Execute$ D\n\
         SVar:D:DB$ DealDamage | Defined$ TriggeredActivator | NumDmg$ 1",
    );
    assert!(
        ping.abilities
            .join("")
            .contains("TargetSpec::Player(PlayerRel::EventPlayer)"),
        "{:?}",
        ping.abilities
    );
    let its_controller = read(
        "Name:X\nTypes:Enchantment\n\
         T:Mode$ SpellCast | ValidCard$ Card | TriggerZones$ Battlefield | Execute$ D\n\
         SVar:D:DB$ DealDamage | Defined$ TriggeredCardController | NumDmg$ 1",
    );
    assert!(
        its_controller
            .abilities
            .join("")
            .contains("TargetSpec::Player(PlayerRel::ControllerOfEvent)"),
        "{:?}",
        its_controller.abilities
    );
    // The same payer on a trigger that is not a cast is refused.
    assert!(refused(
        "Name:X\nTypes:Enchantment\n\
         T:Mode$ Attacks | ValidCard$ Creature | Execute$ D\n\
         SVar:D:DB$ Draw | Defined$ You | UnlessCost$ 1 | UnlessPayer$ TriggeredActivator"
    ));
}

/// Cockatrice: "whenever this creature blocks or becomes blocked by a
/// non-Wall creature, destroy that creature at end of combat", written
/// by the reference as two lines, each executing a delayed trigger that
/// remembers the other creature of its side of the block.
const BLOCK_PAIR: &str = "Name:X\nTypes:Creature\nPT:2/4\n\
    T:Mode$ AttackerBlockedByCreature | ValidCard$ Creature.nonGoblin | \
    ValidBlocker$ Card.Self | Execute$ DelBlocked | TriggerDescription$ …\n\
    T:Mode$ AttackerBlockedByCreature | ValidCard$ Card.Self | \
    ValidBlocker$ Creature.nonGoblin | Execute$ DelBlocker | Secondary$ True | \
    TriggerDescription$ …\n\
    SVar:DelBlocked:DB$ DelayedTrigger | Mode$ Phase | Phase$ EndCombat | \
    ValidPlayer$ Player | Execute$ TrigDestroy | \
    RememberObjects$ TriggeredAttackerLKICopy | TriggerDescription$ …\n\
    SVar:DelBlocker:DB$ DelayedTrigger | Mode$ Phase | Phase$ EndCombat | \
    ValidPlayer$ Player | Execute$ TrigDestroy | \
    RememberObjects$ TriggeredBlockerLKICopy | TriggerDescription$ …\n\
    SVar:TrigDestroy:DB$ Destroy | Defined$ DelayTriggerRememberedLKI";

/// The pair is one ability — the card prints one — about the other
/// creature, destroyed by a delayed trigger at end of combat.
#[test]
fn a_blocks_or_becomes_blocked_pair_is_one_ability_at_end_of_combat() {
    let body = read(BLOCK_PAIR);
    assert_eq!(body.abilities.len(), 1, "{:?}", body.abilities);
    let a = &body.abilities[0];
    assert!(a.contains("Trigger::BlocksOrBecomesBlockedBy(&"), "{a}");
    assert!(
        a.contains(
            "Effect::AtEndOfCombat { about: TargetSpec::EventObject, \
             effects: &[Effect::destroy(TargetSpec::EventObject)] }"
        ),
        "{a}"
    );
    assert!(
        body.statics.contains("creature::GOBLIN"),
        "{}",
        body.statics
    );
}

/// Either half alone is another sentence, and so is a pair whose halves
/// disagree; the remembered object outside a delayed body, a delayed
/// trigger remembering its own source's side, and another phase are
/// each refused.
#[test]
fn a_block_trigger_is_read_only_as_the_whole_pair() {
    let lines: Vec<&str> = BLOCK_PAIR.lines().collect();
    let without = |skip: usize| {
        lines
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != skip)
            .map(|(_, l)| *l)
            .collect::<Vec<_>>()
            .join("\n")
    };
    // The "blocks" half alone, and the "becomes blocked by" half alone.
    for skip in [3, 4] {
        let text = without(skip);
        assert!(refused(&text), "{text}");
        let reason = refusal_reason(&parse(&text), &cats(), None);
        assert!(
            reason
                .as_deref()
                .is_some_and(|r| r.contains("without its mirror")),
            "{reason:?}"
        );
    }
    for (from, to) in [
        // The halves are about different creatures.
        ("ValidBlocker$ Creature.nonGoblin", "ValidBlocker$ Creature"),
        // Both halves marked, or neither.
        (
            "Execute$ DelBlocked |",
            "Execute$ DelBlocked | Secondary$ True |",
        ),
        ("Secondary$ True | ", ""),
        // A delayed trigger about the source's own side of the block.
        (
            "RememberObjects$ TriggeredAttackerLKICopy",
            "RememberObjects$ TriggeredBlockerLKICopy",
        ),
        // Another phase.
        (
            "Phase$ EndCombat | ValidPlayer$ Player | Execute$ TrigDestroy | \
          RememberObjects$ TriggeredBlockerLKICopy",
            "Phase$ End of Turn | ValidPlayer$ Player | Execute$ TrigDestroy | \
          RememberObjects$ TriggeredBlockerLKICopy",
        ),
    ] {
        assert_eq!(BLOCK_PAIR.matches(from).count(), 1, "{from}");
        let text = BLOCK_PAIR.replace(from, to);
        assert!(refused(&text), "{from} → {to}");
    }
    // "Destroy that creature" remembered by nothing.
    assert!(refused(
        "Name:X\nTypes:Creature\nPT:2/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | \
         ValidCard$ Card.Self | Execute$ TrigDestroy\n\
         SVar:TrigDestroy:DB$ Destroy | Defined$ DelayTriggerRememberedLKI"
    ));
}

/// Hypnotic Specter and Fungusaur: damage to an opponent in or out of
/// combat, "that player" the one dealt damage, and "is dealt damage"
/// once however many sources. A player dealt damage "once", a creature
/// target, and damage to a player out of combat are refused.
#[test]
fn damage_triggers_read_whose_damage_and_to_whom() {
    let specter = read(
        "Name:X\nTypes:Creature\nPT:2/2\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Opponent | Execute$ D | \
         TriggerZones$ Battlefield\n\
         SVar:D:DB$ Discard | Defined$ TriggeredTarget | NumCards$ 1 | Mode$ Random",
    );
    let a = specter.abilities.join("");
    assert!(
        a.contains("Trigger::DealsDamageToOpponent(&Filter::This)"),
        "{a}"
    );
    assert!(a.contains("who: PlayerRel::DamagedPlayer"), "{a}");
    let combat = read(
        "Name:X\nTypes:Creature\nPT:2/2\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True \
         | Execute$ D\n\
         SVar:D:DB$ Draw | NumCards$ 1",
    );
    assert!(
        combat
            .abilities
            .join("")
            .contains("Trigger::DealsCombatDamageToPlayer(&Filter::This)"),
        "{:?}",
        combat.abilities
    );
    let fungusaur = read(
        "Name:X\nTypes:Creature\nPT:2/2\n\
         T:Mode$ DamageDoneOnce | ValidTarget$ Card.Self | Execute$ C\n\
         SVar:C:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1",
    );
    assert!(
        fungusaur
            .abilities
            .join("")
            .contains("Trigger::DealtDamage(&Filter::This)"),
        "{:?}",
        fungusaur.abilities
    );
    for refused_line in [
        "T:Mode$ DamageDoneOnce | ValidTarget$ You | Execute$ C",
        "T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Creature | Execute$ C",
        "T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | Execute$ C",
    ] {
        assert!(
            refused(&format!(
                "Name:X\nTypes:Creature\nPT:2/2\n{refused_line}\n\
                 SVar:C:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1"
            )),
            "{refused_line}"
        );
    }
}

/// Gauntlet of Might, Manabarbs, Badgermole Cub: who tapped it is
/// `Activator$` (nobody named is anybody), "its controller" and "that
/// player" are the tapped permanent's controller, and mana for them is
/// `AddManaFor`.
#[test]
fn a_permanent_tapped_for_mana_names_who_tapped_it_and_whose_mana_it_is() {
    let gauntlet = read(
        "Name:X\nTypes:Artifact\n\
         T:Mode$ TapsForMana | ValidCard$ Mountain | Execute$ TrigMana | TriggerZones$ \
         Battlefield | Static$ True | TriggerDescription$ x\n\
         SVar:TrigMana:DB$ Mana | Produced$ R | Amount$ 1 | Defined$ TriggeredCardController",
    );
    let a = gauntlet.abilities.join("");
    assert!(a.contains("by: PlayerRel::EachPlayer"), "{a}");
    assert!(
        a.contains(
            "Effect::AddManaFor { who: PlayerRel::ControllerOfEvent, color: ManaColor::Red, \
             amount: 1 }"
        ),
        "{a}"
    );
    let barbs = read(
        "Name:X\nTypes:Enchantment\n\
         T:Mode$ TapsForMana | ValidCard$ Land | TriggerZones$ Battlefield | Execute$ D\n\
         SVar:D:DB$ DealDamage | Defined$ TriggeredActivator | NumDmg$ 1",
    );
    assert!(
        barbs
            .abilities
            .join("")
            .contains("TargetSpec::Player(PlayerRel::ControllerOfEvent)"),
        "{:?}",
        barbs.abilities
    );
    let cub = read(
        "Name:X\nTypes:Creature\nPT:2/2\n\
         T:Mode$ TapsForMana | ValidCard$ Creature | Activator$ You | Execute$ M | \
         TriggerZones$ Battlefield | Static$ True\n\
         SVar:M:DB$ Mana | Produced$ G",
    );
    assert!(
        cub.abilities.join("").contains("by: PlayerRel::You"),
        "{:?}",
        cub.abilities
    );
    // "That player" of any other trigger is not the tapper.
    assert!(refused(
        "Name:X\nTypes:Enchantment\n\
         T:Mode$ Attacks | ValidCard$ Creature | Execute$ D\n\
         SVar:D:DB$ DealDamage | Defined$ TriggeredActivator | NumDmg$ 1"
    ));
}

/// Mana Short: the line's player target is whose lands tap, and the
/// sub-line's `Defined$ Targeted` is the same player. A `Targeted` on a
/// chain that targets no player names nobody and is refused; a `TapAll`
/// that names nobody is every matching permanent.
#[test]
fn tapping_all_of_a_players_lands_and_their_mana_reads_the_chains_player() {
    let short = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ TapAll | ValidTgts$ Player | ValidCards$ Land | SubAbility$ DrainMana\n\
         SVar:DrainMana:DB$ DrainMana | Defined$ Targeted",
    );
    let a = short.abilities.join("");
    assert!(
        a.contains("Effect::TapAllOf { who: PlayerRel::Chosen, filter: &Filter::LAND }"),
        "{a}"
    );
    assert!(
        a.contains("Effect::LoseUnspentMana { who: PlayerRel::Chosen }"),
        "{a}"
    );
    assert!(a.contains("TargetSpec::AnyPlayer"), "{a}");
    // `Targeted` where the chain targets no player.
    assert!(refused(
        "Name:X\nTypes:Instant\nA:SP$ DrainMana | Defined$ Targeted"
    ));
    // `TargetedController` of a spell target is its controller.
    let spell = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | SubAbility$ Drain\n\
         SVar:Drain:DB$ DrainMana | Defined$ TargetedController",
    );
    assert!(
        spell
            .abilities
            .join("")
            .contains("Effect::LoseUnspentMana { who: PlayerRel::ControllerOfTarget }"),
        "{:?}",
        spell.abilities
    );
    // Twiddle: "tap or untap" is a yes or a no around the toggle.
    let twiddle = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ TapOrUntap | ValidTgts$ Artifact,Creature,Land",
    );
    assert!(
        twiddle
            .abilities
            .join("")
            .contains("Effect::MayDo { effects: &[Effect::ToggleTapTarget] }"),
        "{:?}",
        twiddle.abilities
    );
    let all = read("Name:X\nTypes:Sorcery\nA:SP$ TapAll | ValidCards$ Creature");
    assert!(
        all.abilities
            .join("")
            .contains("Effect::TapAll { filter: &Filter::CREATURE }"),
        "{:?}",
        all.abilities
    );
}

/// Pestilence, Karma, Spell Blast, Dwarven Warriors: counts of
/// everybody's permanents, the active player's permanents, a mana value
/// of exactly X, and a creature nobody can block this turn.
#[test]
fn an_unless_cost_wraps_the_line_whatever_it_says() {
    // Phantasmal Forces: a colour, printed exactly.
    let forces = read(
        "Name:X\nTypes:Creature\nPT:5/1\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield \
         | Execute$ U | TriggerDescription$ x.\n\
         SVar:U:DB$ Sacrifice | UnlessPayer$ You | UnlessCost$ U",
    );
    assert!(
        forces.abilities[0].contains(
            "Effect::PlayerMayPayManaOr { player: PlayerRel::You, \
             cost: mana!(\"{U}\"), effect: &Effect::SacrificeSelf }"
        ),
        "{:?}",
        forces.abilities
    );
    // Force of Nature: the price is the line's, whatever the line does.
    let nature = read(
        "Name:X\nTypes:Creature\nPT:8/8\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield \
         | Execute$ D | TriggerDescription$ x.\n\
         SVar:D:DB$ DealDamage | Defined$ You | NumDmg$ 8 | UnlessCost$ G G G G \
         | UnlessPayer$ You",
    );
    assert!(
        nature.abilities[0].contains("cost: mana!(\"{G}{G}{G}{G}\"), effect: &Effect::DealDamage"),
        "{:?}",
        nature.abilities
    );
    // Generic mana stays the `Amount` tax, and Mana Leak's absent payer
    // is its target's controller.
    let leak = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | UnlessCost$ 3",
    );
    assert!(
        leak.abilities[0].contains(
            "Effect::PlayerMayPayOr { player: PlayerRel::ControllerOfTarget, \
             mana: Amount::Fixed(3), effect: &Effect::CounterTargetSpell }"
        ),
        "{:?}",
        leak.abilities
    );
    // Switched: paying buys the effect.
    let bought = read(
        "Name:X\nTypes:Artifact\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield \
         | Execute$ G | TriggerDescription$ x.\n\
         SVar:G:DB$ GainLife | LifeAmount$ 1 | UnlessCost$ W | UnlessPayer$ You \
         | UnlessSwitched$ True",
    );
    assert!(
        bought.abilities[0].contains("Effect::PlayerMayPayManaThen"),
        "{:?}",
        bought.abilities
    );
    // Refused by name: a payer this cannot ask, subs on one answer, and
    // an absent payer on a line with no target to take one from.
    for (line, reason) in [
        (
            "DB$ Sacrifice | UnlessPayer$ Player | UnlessCost$ 2",
            "an unless-cost paid by `Player`",
        ),
        (
            "DB$ Sacrifice | UnlessCost$ 2",
            "an unless-cost paid by `TargetedController`",
        ),
        (
            "DB$ Sacrifice | UnlessPayer$ You | UnlessCost$ 2 | UnlessResolveSubs$ WhenNotPaid",
            "`UnlessResolveSubs$ WhenNotPaid`",
        ),
        (
            "DB$ Sacrifice | UnlessPayer$ You | UnlessCost$ PayLife<2>",
            "an unless-cost of `PayLife<2>`",
        ),
    ] {
        let script = parse(&format!(
            "Name:X\nTypes:Creature\nPT:1/1\n\
             T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield \
             | Execute$ S | TriggerDescription$ x.\nSVar:S:{line}"
        ));
        assert_eq!(
            refusal_reason(&script, &cats(), None).as_deref(),
            Some(reason),
            "{line}"
        );
    }
}

#[test]
fn counts_bounds_and_an_unblockable_target() {
    let body = read(
        "Name:X\nManaCost:B B\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ End of Turn | TriggerZones$ Battlefield | \
         IsPresent$ Creature | PresentCompare$ EQ0 | Execute$ S\nSVar:S:DB$ Sacrifice\n",
    );
    let text = format!("{}\n{}", body.abilities.join("\n"), body.statics);
    assert!(
        text.contains("Condition::BattlefieldCountAtMost(&Filter::CREATURE, 0)"),
        "{text}"
    );
    // "At the beginning of the end step": everybody's, not yours.
    assert!(text.contains("whose: PlayerRel::EachPlayer"), "{text}");
    // And Pestilence's ability, whose printed recipients are a key the
    // rule reading the recipients claims.
    let body = read(
        "Name:X\nManaCost:B B\nTypes:Enchantment\n\
         A:AB$ DamageAll | Cost$ B | NumDmg$ 1 | ValidCards$ Creature | \
         ValidPlayers$ Player | ValidDescription$ each creature and each player.\n",
    );
    assert_eq!(body.abilities.len(), 1, "{:?}", body.abilities);
    // Somebody's permanents, but not yours: still refused.
    assert!(refused(
        "Name:X\nTypes:Enchantment\nT:Mode$ Phase | Phase$ Upkeep | \
         IsPresent$ Creature.OppCtrl | Execute$ S\nSVar:S:DB$ Sacrifice\n"
    ));
    // An exact count above zero is two bounds.
    assert!(refused(
        "Name:X\nTypes:Enchantment\nT:Mode$ Phase | Phase$ Upkeep | \
         IsPresent$ Creature | PresentCompare$ EQ2 | Execute$ S\nSVar:S:DB$ Sacrifice\n"
    ));

    let body = read(
        "Name:X\nManaCost:B B\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player | TriggerZones$ Battlefield | \
         Execute$ D\nSVar:D:DB$ DealDamage | Defined$ TriggeredPlayer | NumDmg$ X\n\
         SVar:X:Count$Valid Creature.ActivePlayerCtrl\n",
    );
    let text = format!("{}\n{}", body.abilities.join("\n"), body.statics);
    assert!(text.contains("Filter::ControlledByActivePlayer"), "{text}");
    assert!(text.contains("amount: Amount::CountOf"), "{text}");

    let body = read(
        "Name:X\nManaCost:U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card.cmcEQX\nSVar:X:Count$xPaid\n",
    );
    let text = format!("{}\n{}", body.abilities.join("\n"), body.statics);
    assert!(text.contains("Filter::CmcExactlyX"), "{text}");
    // X that counts something else is not the announcement.
    assert!(refused(
        "Name:X\nManaCost:U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card.cmcEQX\n\
         SVar:X:Count$Valid Creature.YouCtrl\n"
    ));

    let body = read(
        "Name:X\nManaCost:2 R\nTypes:Creature\n\
         A:AB$ Effect | Cost$ T | ValidTgts$ Creature.powerLE2 | RememberObjects$ Targeted | \
         ExileOnMoved$ Battlefield | StaticAbilities$ U\n\
         SVar:U:Mode$ CantBlockBy | ValidAttacker$ Card.IsRemembered | Description$ No.\n",
    );
    let text = body.abilities.join("\n");
    assert!(text.contains("keywords: KeywordSet::UNBLOCKABLE"), "{text}");
    assert!(text.contains("Effect::PumpTarget"), "{text}");
    // A blocker named is "can't be blocked by …", another sentence.
    assert!(refused(
        "Name:X\nTypes:Creature\n\
         A:AB$ Effect | Cost$ T | ValidTgts$ Creature | RememberObjects$ Targeted | \
         ExileOnMoved$ Battlefield | StaticAbilities$ U\n\
         SVar:U:Mode$ CantBlockBy | ValidAttacker$ Card.IsRemembered | ValidBlocker$ Wall\n"
    ));
}

/// Keldon Warlord, Plague Rats, Time Walk, Regeneration: the sentences
/// the DSL already had words for.
#[test]
fn characteristic_counts_extra_turns_and_the_enchanted_host() {
    let body = read(
        "Name:X\nManaCost:2 R R\nTypes:Creature Goblin\nPT:*/*\n\
         S:Mode$ Continuous | CharacteristicDefining$ True | SetPower$ X | SetToughness$ X | \
         Description$ Its power.\n\
         SVar:X:Count$Valid Creature.nonWizard+YouCtrl\n",
    );
    let text = format!("{}\n{}", body.abilities.join("\n"), body.statics);
    assert!(
        text.contains("count: PtCount::YouControl(&COUNTED1)"),
        "{text}"
    );
    assert!(text.contains("Filter::ControlledByYou"), "{text}");

    let body = read(
        "Name:X\nManaCost:2 B\nTypes:Creature Rat\nPT:*/*\n\
         S:Mode$ Continuous | CharacteristicDefining$ True | SetPower$ X | SetToughness$ X\n\
         SVar:X:Count$Valid Creature.namedPlague Rats\n",
    );
    let text = format!("{}\n{}", body.abilities.join("\n"), body.statics);
    assert!(text.contains("PtCount::OnBattlefield(&COUNTED1)"), "{text}");
    assert!(text.contains("Filter::Named(\"Plague Rats\")"), "{text}");

    // Somebody else's permanents, and a clause beside the count.
    assert!(refused(
        "Name:X\nTypes:Creature\nS:Mode$ Continuous | CharacteristicDefining$ True | \
         SetPower$ X | SetToughness$ X\nSVar:X:Count$Valid Forest.DefenderCtrl\n"
    ));
    assert!(refused(
        "Name:X\nTypes:Creature\nS:Mode$ Continuous | CharacteristicDefining$ True | \
         IsPresent$ Card.Self+attacking | SetPower$ X | SetToughness$ X\n\
         SVar:X:Count$Valid Forest.YouCtrl\n"
    ));

    let body = read("Name:X\nManaCost:U\nTypes:Sorcery\nA:SP$ AddTurn | NumTurns$ 1\n");
    assert_eq!(body.abilities.len(), 1);
    assert!(body.abilities[0].contains("Effect::TakeExtraTurn"));
    assert!(refused(
        "Name:X\nTypes:Sorcery\nA:SP$ AddTurn | NumTurns$ 2\n"
    ));

    let body = read(
        "Name:X\nManaCost:1 G\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         A:AB$ Regenerate | Cost$ G | Defined$ Enchanted | SpellDescription$ Regenerate.\n",
    );
    assert!(
        body.abilities
            .join("")
            .contains("Effect::RegenerateAll { filter: &Filter::AttachedToBySource }"),
        "{:?}",
        body.abilities
    );
}

/// The Circles of Protection and Reverse Damage: a source chosen as the
/// line resolves, and a shield on you that waits for it.
#[test]
fn a_chosen_source_shield_is_one_sentence_over_three_lines() {
    const COP: &str = "Name:X\nManaCost:1 W\nTypes:Enchantment\n\
         A:AB$ ChooseSource | Cost$ 1 | Choices$ Card.RedSource | AILogic$ NeedsPrevention | \
         SubAbility$ DBEffect | SpellDescription$ The next time.\n\
         SVar:DBEffect:DB$ Effect | ReplacementEffects$ RPrevent | SubAbility$ DBCleanup | \
         ConditionDefined$ ChosenCard | ConditionPresent$ Card | ConditionCompare$ GE1\n\
         SVar:RPrevent:Event$ DamageDone | ValidSource$ Card.ChosenCardStrict+RedSource | \
         ValidTarget$ You | ReplaceWith$ ExileEffect | PreventionEffect$ True | \
         Description$ Prevent it.\n\
         SVar:ExileEffect:DB$ ChangeZone | Defined$ Self | Origin$ Command | Destination$ Exile\n\
         SVar:DBCleanup:DB$ Cleanup | ClearChosenCard$ True\n";
    let body = read(COP);
    let text = format!("{}\n{}", body.abilities.join("\n"), body.statics);
    assert!(
        text.contains("Effect::PreventNextFromChosenSource { sources: &SOURCE1, combat_only: false, all_but: 0, gain_life: false }"),
        "{text}"
    );
    assert!(text.contains("Color::Red"), "{text}");

    let body = read(
        "Name:X\nManaCost:1 W W\nTypes:Instant\n\
         A:SP$ ChooseSource | Choices$ Card,Emblem | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | ReplacementEffects$ RPrevent | ConditionDefined$ ChosenCard | \
         ConditionPresent$ Card,Emblem\n\
         SVar:RPrevent:Event$ DamageDone | ValidSource$ Card.ChosenCardStrict,Emblem.ChosenCard | \
         ValidTarget$ You | ReplaceWith$ GainLifeInstead | PreventionEffect$ True\n\
         SVar:GainLifeInstead:DB$ GainLife | Defined$ You | LifeAmount$ X | SubAbility$ ExileEffect\n\
         SVar:ExileEffect:DB$ ChangeZone | Defined$ Self | Origin$ Command | Destination$ Exile\n\
         SVar:X:ReplaceCount$DamageAmount\n",
    );
    assert!(
        body.abilities
            .join("\n")
            .contains("sources: &Filter::Any, combat_only: false, all_but: 0, gain_life: true"),
        "{:?}",
        body.abilities
    );

    // A replacement that does not recheck what was chosen, a delayed
    // trigger in place of the cleanup, and a chosen colour.
    assert!(refused(&COP.replace(
        "Card.ChosenCardStrict+RedSource",
        "Card.ChosenCardStrict"
    )));
    assert!(refused(&COP.replace(
        "SVar:DBCleanup:DB$ Cleanup | ClearChosenCard$ True",
        "SVar:DBCleanup:DB$ DelayedTrigger | Mode$ Phase"
    )));
    assert!(refused(
        &COP.replace("Card.RedSource", "Card.ChosenColorSource")
    ));
}

/// Samite Healer, Conservator, Fog: the shields a line names.
#[test]
fn prevention_is_a_shield_on_what_the_line_names() {
    let body = read(
        "Name:X\nManaCost:1 W\nTypes:Creature Goblin\nPT:1/1\n\
         A:AB$ PreventDamage | Cost$ T | ValidTgts$ Any | Amount$ 1 | \
         SpellDescription$ Prevent the next 1 damage.\n",
    );
    let text = body.abilities.join("\n");
    assert!(
        text.contains(
            "Effect::PreventNextDamage { target: TargetSpec::AnyTarget, amount: Amount::Fixed(1) }"
        ),
        "{text}"
    );
    assert!(
        text.contains("target = Some(TargetSpec::AnyTarget)"),
        "{text}"
    );

    let body = read(
        "Name:X\nManaCost:2\nTypes:Artifact\n\
         A:AB$ PreventDamage | Cost$ 3 T | Defined$ You | Amount$ 2 | \
         SpellDescription$ Prevent the next 2 damage that would be dealt to you.\n",
    );
    let text = body.abilities.join("\n");
    assert!(
        text.contains("target: TargetSpec::Player(PlayerRel::You), amount: Amount::Fixed(2)"),
        "{text}"
    );

    let body = read("Name:X\nManaCost:G\nTypes:Instant\nA:SP$ Fog | SpellDescription$ Fog.\n");
    assert!(
        body.abilities
            .join("\n")
            .contains("Effect::PreventAllCombatDamageThisTurn"),
        "{:?}",
        body.abilities
    );

    // A shield on nothing, and a Fog narrowed by a key it does not claim.
    assert!(refused(
        "Name:X\nTypes:Instant\nA:SP$ PreventDamage | Amount$ 2\n"
    ));
    assert!(refused(
        "Name:X\nTypes:Instant\nA:SP$ Fog | ValidSource$ Creature.nonBlack\n"
    ));
    assert!(refused(
        "Name:X\nTypes:Instant\nA:SP$ PreventDamage | ValidTgts$ Any | Amount$ 3 | \
         DividedAsYouChoose$ 3\n"
    ));
}

/// A charm is a modal spell, each mode targeting for itself.
#[test]
fn a_charm_is_a_modal_spell_whose_modes_target_for_themselves() {
    let body = read(
        "Name:X\nManaCost:U\nTypes:Instant\n\
         A:SP$ Charm | Choices$ DBCounter,DBDestroy\n\
         SVar:DBCounter:DB$ Counter | TargetType$ Spell | ValidTgts$ Card.Red | \
         SpellDescription$ Counter target red spell.\n\
         SVar:DBDestroy:DB$ Destroy | ValidTgts$ Permanent.Red | \
         SpellDescription$ Destroy target red permanent.\n",
    );
    let text = body.abilities.join("\n");
    assert!(text.contains("AbilityDef::ModalSpell"), "{text}");
    assert!(text.contains("choose: ModeCount::ONE"), "{text}");
    assert_eq!(text.matches("mode!(").count(), 2, "{text}");
    assert!(text.contains("TargetSpec::Spell("), "{text}");
    assert!(text.contains("TargetSpec::Object("), "{text}");

    // A mode the reader cannot say takes the whole card with it.
    assert!(refused(
        "Name:X\nTypes:Instant\nA:SP$ Charm | Choices$ DBGain,DBBalance\n\
         SVar:DBGain:DB$ GainLife | ValidTgts$ Player | LifeAmount$ 3\n\
         SVar:DBBalance:DB$ Balance | Valid$ Land\n"
    ));
    // A count it has not met.
    assert!(refused(
        "Name:X\nTypes:Instant\nA:SP$ Charm | Choices$ A,B | MinCharmNum$ 0\n\
         SVar:A:DB$ Draw | NumCards$ 1\nSVar:B:DB$ Draw | NumCards$ 2\n"
    ));
}

/// `CantBlockBy` from either side of the pairing.
#[test]
fn a_pairing_nobody_may_block_is_the_attackers_restriction() {
    // Invisibility: every creature but a Wall is refused.
    let body = read(
        "Name:X\nManaCost:U U\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ CantBlockBy | ValidAttacker$ Creature.EnchantedBy | \
         ValidBlocker$ Creature.nonGoblin | Description$ can't be blocked except by Goblins.\n",
    );
    let text = body.abilities.join("\n");
    assert!(
        text.contains("Modifier::CantBeBlockedBy(&BLOCKER"),
        "{text}"
    );
    assert!(text.contains("Filter::AttachedToBySource"), "{text}");
    let statics = &body.statics;
    assert!(statics.contains("Not("), "{statics}");

    // Ironclaw Orcs: the source is the blocker, every big creature the
    // attacker.
    let body = read(
        "Name:X\nManaCost:1 R\nTypes:Creature Goblin\nPT:2/2\n\
         S:Mode$ CantBlockBy | ValidAttacker$ Creature.powerGE2 | \
         ValidBlocker$ Creature.Self | Description$ can't block big ones.\n",
    );
    let statics = &body.statics;
    assert!(statics.contains("Filter::This"), "{statics}");

    assert!(refused(
        "Name:X\nTypes:Creature Goblin\nPT:1/1\n\
         S:Mode$ CantBlockBy | ValidAttacker$ Creature.Self\n"
    ));
}

/// No `Defined$` and no target is the source too: Shivan Dragon's
/// "{R}: This creature gets +1/+0 until end of turn." A pump that
/// moves nothing stays refused, because pumping the source by nought
/// would be a card that claims to work and does nothing.
#[test]
fn a_pump_naming_nobody_is_the_sources_own() {
    let body = read(
        "Name:X\nManaCost:4 R R\nTypes:Creature Dragon\nPT:5/5\nK:Flying\n\
         A:AB$ Pump | Cost$ R | NumAtt$ +1 | SpellDescription$ gets +1/+0.\n",
    );
    let text = body.abilities.join("\n");
    assert!(text.contains("filter: &Filter::This"), "{text}");
    assert!(text.contains("power: Amount::Fixed(1)"), "{text}");

    let script = parse("Name:X\nTypes:Instant\nA:SP$ Pump | StackDescription$ None");
    assert!(transcode(&script, &cats(), None).is_none());

    // An Aura's "enchanted creature gets +1/+0" is the host, not the
    // Aura: Firebreathing.
    let body = read(
        "Name:X\nManaCost:R\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         A:AB$ Pump | Cost$ R | Defined$ Enchanted | NumAtt$ +1\n",
    );
    let text = body.abilities.join("\n");
    assert!(
        text.contains("filter: &Filter::AttachedToBySource"),
        "{text}"
    );
}

/// The storage lands' own clause: `PresentDefined$ Self | IsPresent$
/// Card.tapped` as an intervening `if` about this card.
///
/// `landgen` reads the same card from the printed text and writes a
/// bare `Filter::Tapped` for the same ability, and the two are meant to
/// differ. "If this land is tapped" is a sentence about a permanent,
/// and a land that has left the battlefield is not tapped; `IsPresent$`
/// is a question about a *zone*, with the predicate hung off it. Each
/// reader translates the sentence it was handed, and the extra clause
/// is true whenever the shorter one is.
#[test]
fn a_clause_about_this_card_becomes_a_condition_on_the_trigger() {
    let body = read(
        "Name:Bottomless Vault\nManaCost:no cost\nTypes:Land\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | PresentDefined$ Self | \
         IsPresent$ Card.tapped | Execute$ TrigStore\n\
         SVar:TrigStore:DB$ PutCounter | Defined$ Self | CounterType$ STORAGE | \
         CounterNum$ 1\n",
    );
    assert_eq!(
        body.abilities,
        [concat!(
            "triggered!(Trigger::StepBegin { step: StepKind::Upkeep, whose: PlayerRel::You }, ",
            "&[Effect::AddCounter { kind: counters::STORAGE, amount: Amount::Fixed(1) }], ",
            "condition = Some(Condition::SourceMatches(&CHECK1)))"
        )]
    );
    assert!(
        body.statics.contains(concat!(
            "static CHECK1: Filter = ",
            "Filter::And(&[Filter::InZone(ZoneRef::Battlefield), Filter::Tapped]);"
        )),
        "the filter it named: {}",
        body.statics
    );
}

/// The same clause written without `PresentDefined$`, which is what the
/// corpus does four times out of five: of the `T:` lines whose clause
/// is about this card, 155 pin it in the valid-string alone against 38
/// that say `PresentDefined$ Self`.
///
/// A reader that missed the pin would not refuse these — it would
/// *write* them, as a count of the permanents you control, and this one
/// says `YouCtrl` so nothing downstream could tell. That is the whole
/// argument for the case: `Card.Self` on its own would be caught by
/// `ControlCount`'s own guard, and the shape that names a player would
/// not.
///
/// It is also the splice: the zone goes in beside the clauses the
/// valid-string named, not around the pair of them.
#[test]
fn a_valid_string_that_pins_the_source_is_also_a_clause_about_this_card() {
    let body = read(
        "Name:X\nManaCost:W\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | \
         IsPresent$ Card.Self+YouCtrl+YouOwn | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1\n",
    );
    assert_eq!(
        body.abilities,
        [concat!(
            "triggered!(Trigger::StepBegin { step: StepKind::Upkeep, whose: PlayerRel::You }, ",
            "&[Effect::draw(1)], condition = Some(Condition::SourceMatches(&CHECK1)))"
        )]
    );
    assert!(
        body.statics.contains(concat!(
            "static CHECK1: Filter = Filter::And(&[Filter::InZone(ZoneRef::Battlefield), ",
            "Filter::This, Filter::ControlledByYou, Filter::OwnedByYou]);"
        )),
        "the filter it named: {}",
        body.statics
    );
}

/// The same family on an `A:` line, where it restricts *activating*
/// rather than resolving: "Add {U}. Activate only if you control an
/// Island or a Mountain."
///
/// The verge lands are the shape worth testing, because the pool holds
/// a hand-written one — Bleachbone Verge — that says the same thing
/// with the same `Condition::ControlCount`, so this is the one place a
/// reader and a person can be held against each other.
///
/// It is also the arity trap: the one-argument `mana_ability!` takes
/// the **cost** as its only positional argument, so a condition beside
/// it has to spell `Cost::TAP` out or the effects would bind where the
/// cost goes.
#[test]
fn an_activation_clause_becomes_the_condition_on_the_ability() {
    let script = "Name:Riverpyre Verge\nManaCost:no cost\nTypes:Land\n\
         A:AB$ Mana | Cost$ T | Produced$ U | \
         IsPresent$ Island.YouCtrl,Mountain.YouCtrl | SpellDescription$ Add {U}.\n";
    let body = read(script);
    assert_eq!(
        body.abilities,
        [concat!(
            "mana_ability!(Cost::TAP, &[Effect::mana(ManaColor::Blue, 1)], ",
            "condition = Some(Condition::ControlCount(&CHECK1, 1)))"
        )]
    );
    assert!(
        body.statics.contains(concat!(
            "static CHECK1: Filter = Filter::Or(&[",
            "Filter::And(&[Filter::HasSubtype(subtypes::land::ISLAND), Filter::ControlledByYou]), ",
            "Filter::And(&[Filter::HasSubtype(subtypes::land::MOUNTAIN), Filter::ControlledByYou])]);"
        )),
        "the filter it named: {}",
        body.statics
    );

    // A spell is cast and not activated, so there is nothing for the
    // clause to restrict: `spell!` has no precondition and the line
    // refuses by name rather than casting unconditionally.
    let spell = parse(
        "Name:X\nManaCost:U\nTypes:Instant\n\
         A:SP$ Draw | NumCards$ 1 | IsPresent$ Island.YouCtrl\n",
    );
    assert!(transcode(&spell, &cats(), None).is_none());
    assert_eq!(
        refusal_reason(&spell, &cats(), None).as_deref(),
        Some("`IsPresent$` on a spell line")
    );

    // And the family's other keys are claimed only where the clause
    // itself was read: a `PresentZone$` with no `IsPresent$` beside it
    // is still an unclaimed parameter, and still refuses the card.
    let lone = parse(
        "Name:X\nManaCost:no cost\nTypes:Land\n\
         A:AB$ Mana | Cost$ T | Produced$ U | PresentZone$ Graveyard\n",
    );
    assert!(transcode(&lone, &cats(), None).is_none());
    assert_eq!(
        refusal_reason(&lone, &cats(), None).as_deref(),
        Some("unclaimed parameter `Mana.PresentZone`")
    );
}

/// A clause that counts instead: "if you control two or more
/// creatures".
///
/// `Condition::ControlCount` is only the right reading while the
/// valid-string says *whose* — the reference writes the controller into
/// the filter, and a filter that does not name one is asking whether
/// such a permanent exists at all: `Condition::BattlefieldCount`. A
/// filter naming somebody else's is a third question, refused by name.
#[test]
fn a_clause_that_counts_needs_the_filter_to_say_whose() {
    let script = "Name:X\nManaCost:G\nTypes:Creature Elf\nPT:1/1\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | \
         IsPresent$ Creature.YouCtrl | PresentCompare$ GE2 | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1\n";
    let body = read(script);
    assert_eq!(
        body.abilities,
        [concat!(
            "triggered!(Trigger::StepBegin { step: StepKind::Upkeep, whose: PlayerRel::You }, ",
            "&[Effect::draw(1)], ",
            "condition = Some(Condition::ControlCount(&Filter::YOUR_CREATURE, 2)))"
        )]
    );

    let anyone = read(&script.replace("Creature.YouCtrl", "Creature"));
    assert!(
        anyone.abilities[0].contains("Condition::BattlefieldCount(&Filter::CREATURE, 2)"),
        "{:?}",
        anyone.abilities
    );
    let theirs = parse(&script.replace("Creature.YouCtrl", "Creature.OppCtrl"));
    assert!(transcode(&theirs, &cats(), None).is_none());
    assert_eq!(
        refusal_reason(&theirs, &cats(), None).as_deref(),
        Some("`IsPresent$ Creature.OppCtrl`, a count of somebody else's")
    );

    // The same trap one atom further in, and the one that would have
    // been written rather than refused: `Other` is a filter about the
    // card stating the clause, which a count has no room for.
    let another = parse(&script.replace("Creature.YouCtrl", "Creature.Other+YouCtrl"));
    assert!(transcode(&another, &cats(), None).is_none());
    assert_eq!(
        refusal_reason(&another, &cats(), None).as_deref(),
        Some("`IsPresent$ Creature.Other+YouCtrl`, a count relative to this card")
    );
}

/// Every other key of the family refuses by name, and the reasons are
/// what a worklist is made of.
///
/// `NoResolvingCheck$ True` is the one that matters most and is easiest
/// to read past: it is the reference opting *out* of CR 603.4's second
/// check — the clause asked once instead of twice — and a reader that
/// dropped the key would write a card that behaves differently from the
/// script it came from, in the one direction nothing would notice.
#[test]
fn the_rest_of_the_condition_family_refuses_by_name() {
    let base = "Name:X\nManaCost:G\nTypes:Creature Elf\nPT:1/1\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | \
         IsPresent$ Creature.YouCtrl | {EXTRA}Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1\n";
    for (extra, reason) in [
        (
            "NoResolvingCheck$ True | ",
            "`NoResolvingCheck$`, a clause checked once",
        ),
        ("IsPresent2$ Card.Self | ", "a second `IsPresent2$` clause"),
        ("PresentPlayer$ You | ", "`PresentPlayer$ You`"),
        ("PresentZone$ Graveyard | ", "`PresentZone$ Graveyard`"),
        ("PresentCompare$ EQ2 | ", "`PresentCompare$ EQ2`"),
        (
            "PresentDefined$ Remembered | ",
            "`PresentDefined$ Remembered`",
        ),
    ] {
        let parsed = parse(&base.replace("{EXTRA}", extra));
        assert!(transcode(&parsed, &cats(), None).is_none(), "{extra}");
        assert_eq!(
            refusal_reason(&parsed, &cats(), None).as_deref(),
            Some(reason),
            "{extra}"
        );
    }
}

/// A trigger that fires from somewhere other than the battlefield is
/// refused, rather than quietly relocated to it.
///
/// `AbilityDef::Triggered` carries no zone and the engine collects
/// triggers off the battlefield, so a `TriggerZones$ Command` read as
/// an ordinary trigger is an ability that can never fire on a card
/// claiming `Coverage::Implemented` — worse than the stub it replaced.
/// The same line without the key is read as it always was, which is the
/// half that says the refusal is about the zone and not about the
/// trigger.
#[test]
fn a_trigger_that_fires_from_another_zone_is_refused_rather_than_relocated() {
    let elsewhere = "Name:X\nTypes:Creature\nPT:1/1\n\
         T:Mode$ ChangesZone | TriggerZones$ Command | Origin$ Battlefield | \
         Destination$ Graveyard | ValidCard$ Creature.YouCtrl | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1\n";
    let parsed = parse(elsewhere);
    assert!(transcode(&parsed, &cats(), None).is_none());
    assert_eq!(
        refusal_reason(&parsed, &cats(), None).as_deref(),
        Some("`TriggerZones$ Command`")
    );

    let here = parse(&elsewhere.replace("TriggerZones$ Command", "TriggerZones$ Battlefield"));
    assert!(
        transcode(&here, &cats(), None).is_some(),
        "the same trigger on the battlefield: {:?}",
        refusal_reason(&here, &cats(), None)
    );
}

/// `Defined$ Self` on a line that also targets is the **source**, and
/// `Effect::AddCounter` cannot say so.
///
/// That effect puts its counters on the first target when there is one,
/// which is the right reading of a `PutCounter` line with no `Defined$`
/// at all and the wrong one the moment the script names a subject. Both
/// spellings were accepted and emitted identically, so Consumptive Goo —
/// "{2}{B}{B}: Target creature gets -1/-1 until end of turn. Put a +1/+1
/// counter on this creature." — put its counter on the creature it was
/// shrinking, where the two cancelled each other exactly. The card
/// compiled, claimed `Coverage::Implemented`, charged four mana and
/// changed nothing any test could see.
///
/// Both directions, because the fix is a *distinction*: the same line
/// without a target must keep the simpler spelling, or one rule would
/// have been traded for another.
#[test]
fn defined_self_beside_a_target_puts_the_counter_on_the_source() {
    let targeted = read(
        "Name:Goo\nManaCost:B B\nTypes:Creature Ooze\nPT:1/1\n\
         A:AB$ Pump | Cost$ 2 B B | ValidTgts$ Creature | NumAtt$ -1 | \
         NumDef$ -1 | SubAbility$ DBCounter\n\
         SVar:DBCounter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | \
         CounterNum$ 1\n",
    );
    let text = targeted.abilities.join("\n");
    assert!(
        text.contains("Effect::AddCounterFilter { filter: &Filter::This"),
        "the counter is the source's, not the target's: {text}"
    );
    assert!(
        !text.contains("Effect::AddCounter {"),
        "and the spelling that would land it on the target is gone: {text}"
    );

    let untargeted = read(
        "Name:Vault\nManaCost:no cost\nTypes:Land\n\
         A:AB$ PutCounter | Cost$ T | Defined$ Self | CounterType$ STORAGE | \
         CounterNum$ 1\n",
    );
    let text = untargeted.abilities.join("\n");
    assert!(
        text.contains("Effect::AddCounter {"),
        "with no target the source is already what `AddCounter` means, and \
         the simpler spelling is the one to keep: {text}"
    );
}

/// A counter word the DSL registry has an id for reaches the card as
/// the constant, on both the rules that read a counter code.
///
/// The effect and the cost are tested together on purpose: they are
/// two callers of one function precisely so that `STORAGE` cannot come
/// out as two different counters, and a test that only exercised one of
/// them would not notice if that stopped being true. No card prints
/// storage counters as a *cost* in this direction — the cost half of
/// this script is written for the shared table and not for a printing.
#[test]
fn an_assigned_counter_word_reaches_the_card_as_its_constant() {
    let body = read(
        "Name:Crucible\nManaCost:no cost\nTypes:Land\n\
         A:AB$ PutCounter | Cost$ T | CounterType$ STORAGE | CounterNum$ 1\n\
         A:AB$ Untap | Cost$ AddCounter<1/STORAGE>\n",
    );
    let text = body.abilities.join("\n");
    assert!(
        text.contains("kind: counters::STORAGE"),
        "the effect: {text}"
    );
    assert!(
        text.contains("PutCounterSelf { kind: counters::STORAGE"),
        "the cost: {text}"
    );
}

/// The one `K:` line that is a static ability rather than a bit.
///
/// The reference files "you may choose not to untap" as a keyword
/// because it takes no parameters; the rules make it a continuous
/// effect that modifies CR 502.3's turn-based action. So it lands in
/// `abilities` beside the card's own, and leaves `keywords` alone —
/// a bit there would be a keyword no engine rule reads.
#[test]
fn the_may_not_untap_keyword_is_a_static_ability_and_not_a_bit() {
    let body = read(
        "Name:Bottomless Vault\nManaCost:no cost\nTypes:Land\n\
         K:You may choose not to untap CARDNAME during your untap step.\n\
         A:AB$ Mana | Cost$ T | Produced$ B | Amount$ 1\n",
    );
    assert_eq!(
        body.abilities,
        [
            "static_ability!(Filter::This, Modifier::MayChooseNotToUntap)",
            "mana_ability!(&[Effect::mana(ManaColor::Black, 1)])",
        ]
    );
    assert!(body.keywords.is_empty(), "{:?}", body.keywords);

    // One word off the printed sentence and it is an unread keyword
    // again. The match is the whole line on purpose: every one of the
    // 45 scripts that print this writes it exactly one way, so a
    // looser reading would only ever be reading something else.
    let parsed = parse(
        "Name:X\nTypes:Land\n\
         K:You may choose not to untap CARDNAME during your upkeep.\n",
    );
    assert!(transcode(&parsed, &cats(), None).is_none());
    assert_eq!(
        refusal_reason(&parsed, &cats(), None).as_deref(),
        Some("keyword `You`")
    );
}

/// `AB$ Untap` says what it untaps by what it leaves *out*, and
/// `Cost$ AddCounter<n/KIND>` is a counter paid rather than a counter an
/// effect puts on. Devoted Druid prints both in one line, which is why
/// it is the card read here.
///
/// CR 115.1c is why the untap cannot be one rule with a self-filter: an
/// ability whose script names no `ValidTgts$` has no target, and
/// `UntapTarget` would walk an empty `res.targets` and untap nothing.
#[test]
fn an_untap_with_no_valid_string_untaps_the_source_that_paid_for_it() {
    let body = read(
        "Name:Devoted Druid\nManaCost:1 G\nTypes:Creature Elf Druid\nPT:0/2\n\
         A:AB$ Mana | Cost$ T | Produced$ G | SpellDescription$ Add {G}.\n\
         A:AB$ Untap | Cost$ AddCounter<1/M1M1> | SpellDescription$ Untap CARDNAME.\n",
    );
    assert_eq!(
        body.abilities,
        [
            "mana_ability!(&[Effect::mana(ManaColor::Green, 1)])",
            "activated!(cost!(PutCounterSelf { kind: CounterKind::M1M1, n: 1 }), \
             &[Effect::UntapSelf])",
        ]
    );
}
