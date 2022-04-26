use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::Item;

//////////////////////////////////////////////////////////////////////
/// STATE
//////////////////////////////////////////////////////////////////////

/// ## Description
/// A custom struct for storing the state contract setting.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub incentive_contract: Addr,
    pub astroport_factory_address: Addr,
    pub aust_token_address: Addr,
    pub anchor_market_contract: Addr,
    pub oracle_hub_contract: Addr,
    pub owner_address: Addr,
}

pub const STATE: Item<State> = Item::new("state");
