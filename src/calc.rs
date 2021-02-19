//! Calculations of battles between units.
extern crate serde;

use crate::units;
use serde::{Serialize, Deserialize};
use rocket_contrib::json::JsonValue;


#[derive(Deserialize)]
pub struct UnitInput {
    pub unit: String,
    #[serde(default)]
    pub health: Option<f32>,
    #[serde(default)]
    pub flags: u8
}

impl UnitInput {
    pub fn to_unit(&self) -> units::Unit {
        let mut unit = units::UNIT_LIST.get_unit_by_id(
            &self.unit
        ).unwrap();    // TODO: Handle error for bad unit ID.
        unit.apply_bit_flags(self.flags);
        unit.health = self.health.unwrap_or(unit.max_health);
        unit
    }
}


#[derive(Deserialize)]
pub struct BattleInput {
    pub attackers: Vec<UnitInput>,
    pub defender: UnitInput
}

impl BattleInput {
    pub fn to_state(&self) -> BattleState {
        let mut attackers: Vec<units::Unit> = vec![];
        for attacker in self.attackers.iter() {
            attackers.push(attacker.to_unit());
        }
        let defender = self.defender.to_unit();
        BattleState { attackers, defender }
    }
}


#[derive(Serialize)]
pub struct BattleState {
    pub attackers: Vec<units::Unit>,
    pub defender: units::Unit
}

impl BattleState {
    pub fn defender_is_better(&self, other: &BattleState) -> Option<bool> {
        let defender_is_better = self.defender.is_better_than(
            &other.defender
        );
        if self.defender.converted {
            if !other.defender.converted {
                Option::Some(true)
            } else if defender_is_better.is_some() {
                defender_is_better
            } else {
                Option::None
            }
        } else {
            if other.defender.converted {
                Option::Some(false)
            } else if defender_is_better.is_some() {
                return Option::Some(!defender_is_better.unwrap());
            } else {
                Option::None
            }
        }
    }

    pub fn count_dead(&self) -> u8 {
        let mut count = 0;
        for attacker in self.attackers.iter() {
            if attacker.health < 0.0 {
                count += 1;
            }
        }
        count
    }

    pub fn attackers_are_better(&self, other: &BattleState) -> bool {
        let (this_dead, other_dead) = (self.count_dead(), other.count_dead());
        if this_dead < other_dead {
            return true;
        } else if other_dead < this_dead {
            return false;
        }
        // TODO: Compare HP of remaining units.
        return false;
    }

    pub fn is_better_than(&self, other: &BattleState) -> bool {
        let defender_is_better = self.defender_is_better(other);
        if defender_is_better.is_some() {
            return defender_is_better.unwrap();
        }
        return self.attackers_are_better(other);
    }

    pub fn to_json(&self) -> JsonValue {
        let mut attackers_health = vec![];
        for attacker in &self.attackers {
            attackers_health.push(attacker.health);
        }
        let defender_health = unsafe {
            self.defender.health.to_int_unchecked::<i8>()
        };
        json!({
            "attackers": attackers_health,
            "defender": {
                "health": defender_health,
                "frozen": self.defender.frozen,
                "converted": self.defender.converted
            }
        })
    }
}


/// Check if an attacker will recieve retaliation from a defender.
fn check_retaliation(attacker: &units::Unit, defender: &units::Unit) -> bool {
    if defender.frozen || defender.converted {
        false
    } else if defender.health <= 0.0 {
        false
    } else if !defender.can_retaliate {
        false
    } else if attacker.forced_retaliation.is_some() {
        attacker.forced_retaliation.unwrap()
    } else if defender.forced_retaliation.is_some() {
        defender.forced_retaliation.unwrap()
    } else {
        (!attacker.ranged) || defender.ranged
    }
}


/// Calculate the damage done to a defender, and retaliation to an attacker.
pub fn attack(attacker: &mut units::Unit, defender: &mut units::Unit) {
    let attack_force = attacker.attack * (
        attacker.health / attacker.max_health
    );
    let defence_force = defender.defence_with_bonus * (
        defender.health / defender.max_health
    );
    let total_force = 4.5 / (attack_force + defence_force);
    let damage = (attack_force * attacker.attack * total_force).round();
    defender.health -= damage;
    if check_retaliation(attacker, defender) {
        let retaliation_damage = (
            defence_force * defender.defence * total_force
        ).round();
        attacker.health -= retaliation_damage;
    }
}


/// Calculate a battle between two units.
/// Includes converting and freezing as well as actually attacking.
pub fn battle(attacker: &mut units::Unit, defender: &mut units::Unit) {
    if defender.converted {
        return;
    }
    if attacker.attack > 0.0 {
        attack(attacker, defender);
    }
    if attacker.health > 0.0 {
        if attacker.can_convert {
            defender.converted = true;
        } else if attacker.can_freeze {
            defender.frozen = true;
        }
    }
}


/// Calculate the result of attacking a defender with a series of attackers.
pub fn battle_many(state: &mut BattleState) {
    for mut attacker in state.attackers.iter_mut() {
        battle(&mut attacker, &mut state.defender);
    }
}


struct AttackerPermuter {
    order: Vec<usize>,
    p: Vec<usize>,
    i: usize,
    n: usize
}

impl Iterator for AttackerPermuter {
    type Item = Vec<usize>;

    /// Use QuickPerm to find all permutations of the attackers list.
    /// Instead of creating many lists, this simply returns the indeces of the
    /// attackers to use (in order).
    fn next(&mut self) -> Option<Vec<usize>> {
        if self.i >= self.n {
            return Option::None;
        }
        self.p[self.i] -= 1;
        let j = if (self.i % 2) == 1 { self.p[self.i] } else { 0 };
        self.order.swap(j.into(), self.i);
        self.i = 1;
        while self.p[self.i] == 0 {
            self.p[self.i] = self.i;
            self.i += 1;
        }
        Option::Some(self.order.clone())
    }
}


fn attacker_permuatations(num_attackers: usize) -> AttackerPermuter {
    AttackerPermuter {
        order: (0..num_attackers).collect(),
        p: (0..(num_attackers + 1)).collect(),
        i: 1,
        n: num_attackers
    }
}


/// Calculate the best order of attack.
pub fn optimise_battle(state: BattleState) -> (Vec<usize>, BattleState) {
    let mut best_order = Option::None;
    let mut best_state: Option<BattleState> = Option::None;
    for order in attacker_permuatations(state.attackers.len()) {
        let mut attackers = vec![];
        for idx in order.iter() {
            attackers.push(state.attackers[*idx].clone());
        }
        let defender = state.defender.clone();
        let mut this_state = BattleState { attackers, defender };
        let best_state_ref = &best_state.as_ref();
        battle_many(&mut this_state);
        let use_state = if best_state_ref.is_some() {
            this_state.is_better_than(&best_state_ref.unwrap())
        } else {
            true
        };
        if use_state {
            best_state = Option::Some(this_state);
            best_order = Option::Some(order);
        }
    }
    (best_order.unwrap(), best_state.unwrap())
}
