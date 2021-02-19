//! Defines the API routes.
#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;

use rocket_contrib::json::{Json, JsonValue};

mod calc;
mod units;


#[get("/units")]
fn get_units() -> JsonValue {
    json!(units::UNIT_LIST.units)
}


#[post("/battle", format="json", data="<units>")]
fn calc_battle(units: Json<calc::BattleInput>) -> JsonValue {
    let mut state = units.to_state();
    calc::battle_many(&mut state);
    state.to_json()
}


#[post("/optim", format="json", data="<units>")]
fn optimise_battle(units: Json<calc::BattleInput>) -> JsonValue {
    let state = units.to_state();
    let (best_order, best_state) = calc::optimise_battle(state);
    json!({
        "order": best_order,
        "state": best_state.to_json()
    })
}


fn main() {
    rocket::ignite()
        .mount("/", routes![get_units, calc_battle, optimise_battle])
        .launch();
}
