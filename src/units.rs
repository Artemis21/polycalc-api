//! Tool for loading the unit types from the JSON file, and accessing them.
extern crate serde;
extern crate serde_json;

use std::fs;
use serde::{Serialize, Deserialize};


lazy_static! {
    pub static ref UNIT_LIST: UnitTypeList = init_unit_list();
}


/// Utility to read a flag from a set of flags.
fn read_flag(flags: u8, flag_num: u8) -> bool {
    ((1 << flag_num) & flags) != 0
}


/// A single unit type, eg. Catapult, loaded from JSON.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UnitType {
    id: String,
    display_name: String,
    aliases: Vec<String>,
    hidden: bool,
    health: f32,
    attack: f32,
    defence: f32,
    range: u8,
    abilities: Vec<String>
}

impl UnitType {
    /// Create an instance of a unit with default flags.
    pub fn create_unit(&self) -> Unit {
        let can_retaliate = (self.attack != 0.0) && (self.defence != 0.0);
        let can_freeze = self.abilities.contains(
            &String::from("freeze_area")
        );
        let can_convert = self.abilities.contains(&String::from("convert"));
        Unit {
            display_name: self.display_name.clone(),
            max_health: self.health,
            health: self.health,
            attack: self.attack,
            defence: self.defence,
            defence_with_bonus: self.defence,
            forced_retaliation: Option::None,
            can_retaliate: can_retaliate,
            can_convert: can_convert,
            can_freeze: can_freeze,
            ranged: self.range > 1,
            veteran: false,
            frozen: false,
            converted: false
        }
    }
}


/// An actual unit, an instance of one of the `UnitType`s.
/// Includes additional flags to indicate the current state of the unit.
#[derive(Clone, Debug, Serialize)]
pub struct Unit {
    pub display_name: String,
    pub max_health: f32,
    pub health: f32,
    pub attack: f32,
    pub defence: f32,
    pub defence_with_bonus: f32,
    // For an attacker: will it recieve retaliation.
    // For a defender: will it retaliate.
    pub forced_retaliation: Option<bool>,
    pub can_freeze: bool,
    pub can_convert: bool,
    pub can_retaliate: bool,
    pub ranged: bool,
    pub veteran: bool,
    pub frozen: bool,
    pub converted: bool
}

impl Unit {
    /// Read and apply bit flags from a byte.
    pub fn apply_bit_flags(&mut self, flags: u8) {
        if read_flag(flags, 0) {
            self.defence_with_bonus *= 0.8;    // Poisoned
        }
        if read_flag(flags, 1) {
            self.defence_with_bonus *= 1.5;    // Bonus
        }
        if read_flag(flags, 2) {
            self.defence_with_bonus *= 4.0;    // Walled
        }
        if read_flag(flags, 3) {
            self.defence_with_bonus += 0.5;     // Boosted
        }
        self.veteran = read_flag(flags, 4);
        if self.veteran {
            self.max_health += 5.0;
        }
        self.forced_retaliation = if read_flag(flags, 5) {
            Option::Some(true)
        } else if read_flag(flags, 6) {
            Option::Some(false)
        } else {
            Option::None
        };
        self.frozen = read_flag(flags, 7);
    }

    pub fn is_better_than(&self, other: &Unit) -> Option<bool> {
        if self.health > other.health {
            return Option::Some(true);
        } else if other.health > self.health {
            return Option::Some(false);
        }
        if (!self.frozen) && other.frozen {
            return Option::Some(true);
        } else if self.frozen && (!other.frozen) {
            return Option::Some(false);
        }
        return Option::None;
    }
}


/// A list of all the possible unit types.
/// Only one of these should ever need to be initialised.
#[derive(Debug)]
pub struct UnitTypeList {
    pub units: Vec<UnitType>
}

impl UnitTypeList {
    /// Read all the units from a JSON file.
    /// Panics if the file is missing or badly formatted.
    pub fn read_units(&mut self) {
        let raw = fs::read_to_string("units.json")
            .expect("Unit file missing.");
        self.units = serde_json::from_str(&raw)
            .expect("Unit file badly formatted.");
    }

    /// Look up a unit by ID.
    pub fn get_unit_by_id(&self, unit_id: &String) -> Option<Unit> {
        for elem in self.units.iter() {
            if &elem.id == unit_id {
                return Option::Some(elem.create_unit());
            }
        }
        Option::None
    }
}


/// Utility to create and initialise a UnitTypeList.
/// This should only be called once.
pub fn init_unit_list() -> UnitTypeList {
    let mut units = UnitTypeList {
        units: vec![]
    };
    units.read_units();
    units
}
