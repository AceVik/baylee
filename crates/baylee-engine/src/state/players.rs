//! Players: hand size, sides, what a life total may do, damage to a player, leaving the game, day and night, the monarch, draw limits.

use super::{GameState, Side};
use crate::event::{Cause, GameEvent};
use crate::turn::DayNight;
use baylee_core::ids::{ObjectId, PlayerId};

impl GameState {
    /// The latest applicable hand-size setting, or the normal seven-card limit.
    /// Removing the maximum is itself a timestamped setting (CR 613.11).
    #[must_use]
    pub fn maximum_hand_size(&self, player: PlayerId) -> Option<usize> {
        use baylee_cards_dsl::Modifier;
        let setting = if self.effects.may_reveal_or_change_hand_size() {
            self.effects
                .iter()
                .filter_map(|fx| {
                    let value = match fx.modifier {
                        Modifier::NoMaxHandSize if fx.controller == player => None,
                        Modifier::ChosenPlayerMaxHandSize(size)
                            if fx
                                .source
                                .and_then(|id| self.object(id))
                                .and_then(crate::object::GameObject::chosen_opponent)
                                == Some(player) =>
                        {
                            Some(usize::from(size))
                        }
                        _ => return None,
                    };
                    Some((fx.timestamp, fx.id, value))
                })
                .max_by_key(|(timestamp, id, _)| (*timestamp, *id))
        } else {
            None
        };
        setting.map_or_else(
            || {
                Some(
                    usize::try_from(
                        7i32 + i32::from(self.players[player.get() as usize].hand_modifier),
                    )
                    .unwrap_or(0),
                )
            },
            |(_, _, value)| value,
        )
    }

    /// Whether the currently applicable setting removes this player's limit.
    #[must_use]
    pub fn no_max_hand_size(&self, player: PlayerId) -> bool {
        self.maximum_hand_size(player).is_none()
    }

    /// The side a seat plays for (CR 102.3).
    #[must_use]
    pub fn side_of(&self, player: PlayerId) -> Side {
        self.players
            .get(player.get() as usize)
            .and_then(|p| p.team)
            .map_or(Side::Solo(player), Side::Team)
    }

    /// Whether `other` is an opponent of `player` — a different side, which
    /// in a game with no teams is simply a different seat.
    ///
    /// Every rule that says "opponent" goes through here. The ones that say
    /// "each other player" (a draw offer, a symmetrical effect) deliberately
    /// do not: a teammate is not an opponent, but they are another player.
    #[must_use]
    pub fn is_opponent(&self, other: PlayerId, player: PlayerId) -> bool {
        other != player && self.side_of(other) != self.side_of(player)
    }

    /// Whether `player` may pay `amount` life (CR 119.4).
    ///
    /// The rule is "greater than or **equal** to the amount of the payment",
    /// so a player on exactly two life may pay two and lose the game to
    /// CR 704.5a a moment later. That is their call and not the engine's:
    /// this was written three times as `life <= amount → no`, which quietly
    /// took the last point of life off the table — a shockland entered
    /// tapped without asking, and a fetchland was never offered. The margin
    /// that keeps the house AI from killing itself is the AI's own
    /// (`activate::life_ok`, `life > amount + 5`), which is where a policy
    /// belongs; a rule that refuses a legal action is not a policy.
    ///
    /// Zero is always payable whatever the life total, including a negative
    /// one, which is CR 119.4b said exactly.
    ///
    /// Two-Headed Giant pays out of the team's life total (CR 119.4a), which
    /// this does not model: teams exist here ([`Player::team`]) but life
    /// does not pool — every seat carries its own — so this asks the seat.
    ///
    /// A player who can't lose life can't pay any (CR 119.8), which
    /// [`Self::life_payable`] answers for this and for the pay-X-life cap.
    #[must_use]
    pub fn can_pay_life(&self, player: PlayerId, amount: i32) -> bool {
        amount <= 0 || self.life_payable(player) >= amount
    }

    /// How much life `player` has to pay with: their total, or none while
    /// they can't lose life (CR 119.8: "a cost that involves having that
    /// player pay life can't be paid").
    ///
    /// [`Self::can_pay_life`] and Toxic Deluge's cap on X both read this, so
    /// the two can't disagree about what a player may pay.
    #[must_use]
    pub fn life_payable(&self, player: PlayerId) -> i32 {
        if self.cant_lose_life(player) || self.life_total_cant_change(player) {
            return 0;
        }
        self.players
            .get(player.get() as usize)
            .map_or(0, |p| p.life)
    }

    /// Whether a continuous rule forbids any change to this player's life total.
    #[must_use]
    pub fn life_total_cant_change(&self, player: PlayerId) -> bool {
        self.effects.may_change_player_or_sba_rules()
            && self.effects.iter().any(|effect| {
                let baylee_cards_dsl::Modifier::LifeTotalCantChange { who } = effect.modifier
                else {
                    return false;
                };
                crate::eval::players(who, self, effect.controller)
                    .is_some_and(|players| players.contains(&player))
            })
    }

    /// Whether an effect says `player` can't lose life (Everybody Lives!).
    ///
    /// Each effect's `who` is read from that effect's own controller, the
    /// way [`Self::draw_limit`] reads its own. A relation only a resolution
    /// can answer (`Chosen`, `ControllerOfTarget`) names nobody here, and
    /// `lints::a_continuous_player_relation_is_one_the_state_can_answer` keeps
    /// the pool from printing one.
    #[must_use]
    pub fn cant_lose_life(&self, player: PlayerId) -> bool {
        self.effects.may_change_player_or_sba_rules()
            && self.effects.iter().any(|fx| {
                let baylee_cards_dsl::Modifier::CantLoseLife { who } = fx.modifier else {
                    return false;
                };
                crate::eval::players(who, self, fx.controller).is_some_and(|p| p.contains(&player))
            })
    }

    /// Whether an effect says `player` doesn't lose the game for having 0 or
    /// less life (`Modifier::NoLossForZeroLife`, Lich). Each effect's `who`
    /// is read from its own controller, so a Lich that changes hands takes
    /// the exception with it.
    #[must_use]
    pub fn no_loss_for_zero_life(&self, player: PlayerId) -> bool {
        self.effects.may_change_player_or_sba_rules()
            && self.effects.iter().any(|fx| {
                let baylee_cards_dsl::Modifier::NoLossForZeroLife { who } = fx.modifier else {
                    return false;
                };
                crate::eval::players(who, self, fx.controller).is_some_and(|p| p.contains(&player))
            })
    }

    /// Whether `player`'s life gains are replaced by draws
    /// (`Modifier::LifeGainDrawsInstead`, Lich), read as
    /// [`Self::cant_lose_life`] reads its own.
    #[must_use]
    pub fn life_gain_draws_instead(&self, player: PlayerId) -> bool {
        self.effects.may_change_player_or_sba_rules()
            && self.effects.iter().any(|fx| {
                let baylee_cards_dsl::Modifier::LifeGainDrawsInstead { who } = fx.modifier else {
                    return false;
                };
                crate::eval::players(who, self, fx.controller).is_some_and(|p| p.contains(&player))
            })
    }

    /// Whether a continuous effect skips this player's whole draw step.
    #[must_use]
    pub fn skips_draw_step(&self, player: PlayerId) -> bool {
        self.effects.may_change_player_or_sba_rules()
            && self.effects.iter().any(|fx| {
                let baylee_cards_dsl::Modifier::SkipDrawStep { who } = fx.modifier else {
                    return false;
                };
                crate::eval::players(who, self, fx.controller).is_some_and(|p| p.contains(&player))
            })
    }

    /// Whether an effect has `player` skip their untap steps
    /// (`Modifier::SkipUntapStep`, Stasis). Each effect's `who` is read from
    /// its own controller, as [`Self::cant_lose_life`] reads its own.
    #[must_use]
    pub fn skips_untap_step(&self, player: PlayerId) -> bool {
        self.effects.may_change_player_or_sba_rules()
            && self.effects.iter().any(|fx| {
                let baylee_cards_dsl::Modifier::SkipUntapStep { who } = fx.modifier else {
                    return false;
                };
                crate::eval::players(who, self, fx.controller).is_some_and(|p| p.contains(&player))
            })
    }

    /// Damage dealt to a player, after prevention: the life it costs
    /// (CR 120.3a), the turn's tally and the [`GameEvent::DamageDealt`]
    /// record. **The** door for damage to a player, as
    /// [`Self::change_life`] is for life: combat damage
    /// (`combat::deal_damage_to_player`) and an effect's
    /// (`resolve::life::deal_to_player`) both come through here, so "the
    /// damage dealt to you this turn" (Simulacrum) reads one tally that
    /// neither of them can forget to write.
    ///
    /// The tally counts damage dealt, not life lost, so it is not kept in
    /// `change_life`: a payment loses life and is no damage, and a player
    /// whose life can't change is still dealt the damage (`change_life`
    /// refuses the loss, this records the damage). Nothing is dealt below
    /// one point; the caller has already prevented what it prevents.
    pub fn damage_player(
        &mut self,
        source: ObjectId,
        player: PlayerId,
        amount: u32,
        is_combat: bool,
        cause: Cause,
    ) {
        if amount == 0 {
            return;
        }
        self.change_life(player, -i32::try_from(amount).unwrap_or(i32::MAX), cause);
        self.record_player_damage_result(source, player, amount, is_combat);
    }

    /// Records damage independently of its life-loss or poison result.
    pub(crate) fn record_player_damage_result(
        &mut self,
        source: ObjectId,
        player: PlayerId,
        amount: u32,
        is_combat: bool,
    ) {
        if amount == 0 {
            return;
        }
        if let Some(tally) = self.per_turn.damage_dealt_to.get_mut(player.get() as usize) {
            *tally = tally.saturating_add(amount);
        }
        self.log_damage(
            source,
            None,
            crate::event::DamageTarget::Player(player),
            None,
            amount,
            is_combat,
        );
        self.journal.record(GameEvent::DamageDealt {
            source: Some(source),
            target: crate::event::DamageTarget::Player(player),
            amount,
            is_combat,
        });
    }

    /// Whether `player` has left the game (CR 800.4a). Nothing is created
    /// for them any more, and nothing they control triggers (CR 800.4d).
    #[must_use]
    pub fn has_left(&self, player: PlayerId) -> bool {
        self.players[usize::from(player.get())].has_lost()
    }

    /// The game becomes day, or day becomes night's opposite (CR 730.1).
    ///
    /// Every route to the designation goes through this door and
    /// [`Self::become_night`], so that "it becomes day" is journalled once
    /// and in one place — CR 730.1a's "night becomes day" is the same
    /// event, the game losing one designation and gaining the other, and a
    /// card that triggers on it has one entry to read. Setting the field
    /// directly would record nothing.
    pub fn become_day(&mut self) {
        self.set_designation(DayNight::Day);
    }

    /// The game becomes night (CR 730.1). See [`Self::become_day`].
    pub fn become_night(&mut self) {
        self.set_designation(DayNight::Night);
    }

    fn set_designation(&mut self, now: DayNight) {
        if self.day_night == Some(now) {
            return;
        }
        self.day_night = Some(now);
        self.journal.record(GameEvent::DayNightChanged { now });
    }

    /// Sets the monarch (CR 724.3), and ends every exile that lasted "until
    /// an opponent becomes the monarch" (Palace Jailer) for which the new
    /// monarch is such an opponent.
    ///
    /// "An opponent" of the player who controlled the exiling ability, which
    /// the exile wrote down: not of whoever controls the Jailer now, and not
    /// of anybody at all when the Jailer is gone. Every other linked exile
    /// is left alone. Skyclave Apparition's has no end, and Safe Haven's
    /// ends when Safe Haven says so.
    pub fn set_monarch(&mut self, player: PlayerId) {
        let previous = self.monarch;
        self.monarch = Some(player);
        if previous == Some(player) {
            return;
        }
        self.journal.record(GameEvent::BecameMonarch { player });
        self.return_linked(|state, _, until| {
            matches!(
                until,
                Some(crate::object::LinkUntil::OpponentBecomesMonarch { of })
                    if state.is_opponent(player, of)
            )
        });
    }

    /// `player` has left the game (CR 800.4a) and, if they were the monarch,
    /// the designation passes on at the same time (CR 724.4): to the active
    /// player, or, when the active player is the one leaving, to the next
    /// player in turn order still in the game. With nobody left the game goes
    /// on with no monarch.
    ///
    /// `sba::eliminate_player` calls this once `player` is marked as having
    /// left, so the leaver is never the heir, and neither is anybody in
    /// `leaving`, the players leaving at the same time who have yet to be
    /// marked: players who lose in one state-based check leave together, and
    /// the crown passed through one of them on its way to the heir, ending a
    /// Palace Jailer's exile on the way. Through [`Self::set_monarch`],
    /// so an exile that waited for an opponent to become the monarch (Palace
    /// Jailer) ends if the heir is such an opponent. The rule's "if there is
    /// no active player" never arises here, because the engine always has one
    /// (`TurnInfo::active`), and nothing in the engine keeps a player still
    /// in the game from becoming the monarch.
    pub(crate) fn monarch_leaves(&mut self, player: PlayerId, leaving: &[PlayerId]) {
        if self.monarch != Some(player) {
            return;
        }
        let seats = self.players.len();
        let active = usize::from(self.turn.active.get());
        let heir = (0..seats)
            .map(|offset| PlayerId::new(((active + offset) % seats) as u8))
            .find(|&p| !self.has_left(p) && !leaving.contains(&p));
        match heir {
            Some(heir) => self.set_monarch(heir),
            None => self.monarch = None,
        }
    }

    /// How many cards `player` may draw this turn, or `None` for no limit.
    ///
    /// The **lowest** limit wins rather than the newest or the sum, because
    /// "can't" is not a number being modified: two Spirits of the Labyrinth
    /// are one limit of one, and a limit of one beside a limit of two is a
    /// limit of one (CR 101.2).
    ///
    /// The relation is read from each effect's own controller, so one
    /// Leovold limits that seat's opponents and says nothing about its
    /// controller, while one Spirit limits the table including whoever
    /// played it.
    #[must_use]
    pub fn draw_limit(&self, player: PlayerId) -> Option<u32> {
        if !self.effects.may_change_player_or_sba_rules() {
            return None;
        }
        self.effects
            .iter()
            .filter_map(|fx| {
                let baylee_cards_dsl::Modifier::DrawLimitPerTurn { who, limit } = fx.modifier
                else {
                    return None;
                };
                crate::eval::players(who, self, fx.controller)?
                    .contains(&player)
                    .then_some(u32::from(limit))
            })
            .min()
    }
}
