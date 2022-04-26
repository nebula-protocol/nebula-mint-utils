use cosmwasm_std::{Attribute, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use astroport::asset::Asset;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub incentive_contract: String,
    pub astroport_factory_address: String,
    pub aust_token_address: String,
    pub anchor_market_contract: String,
    pub oracle_hub_contract: String,
    pub owner_address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    MintCT {
        /// Cluster contract address
        cluster_address: String,
    },
    _MintCT {
        /// Cluster contract address
        cluster_address: String,
        natives: Vec<String>,
        tokens: Vec<String>,
        cluster_token: String,
        user: String,
    },
    _SendToUser {
        /// Cluster contract address
        cluster_token: String,
        user: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    SimulateMint {
        cluster_address: String,
        ust_amount: Uint128,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ClusterStateResponse {
    /// The current total supply of the cluster token
    pub outstanding_balance_tokens: Uint128,
    /// Prices of the assets in the cluster
    pub prices: Vec<String>,
    /// Current inventory / asset balances
    pub inv: Vec<Uint128>,
    /// Penalty contract address
    pub penalty: String,
    /// Cluster token address
    pub cluster_token: String,
    /// The current asset target weights
    pub target: Vec<Asset>,
    /// The address of this cluster contract
    pub cluster_contract_address: String,
    /// The cluster active status - not active if decommissioned
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SimulateMintResponse {
    /// Actual minted cluster token amount
    pub create_tokens: Uint128,
    /// Incurred penalty / reward from rebalance
    pub penalty: Uint128,
    /// Returned attributes to the caller
    pub attributes: Vec<Attribute>,
    pub create_asset_amounts: Vec<Uint128>,
}

/// ## Description
/// This structure describes the available query messages for the cluster contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsgNebula {
    /// ClusterState returns the current cluster state.
    ClusterState {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IncentivesMsg {
    /// IncentivesCreate executes the create operation on a specific cluster.
    IncentivesCreate {
        /// cluster contract
        cluster_contract: String,
        /// assets offerred for minting
        asset_amounts: Vec<Asset>,
        /// minimum cluster tokens returned
        min_tokens: Option<Uint128>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsgPenalty {
    /// PenaltyQueryCreate calculates the actual create amount after taking penalty into consideration.
    PenaltyQueryCreate {
        /// a specific height to compute mint at
        block_height: u64,
        /// current total supply for a cluster token
        cluster_token_supply: Uint128,
        /// current inventory of inventory assets in a cluster
        inventory: Vec<Uint128>,
        /// the provided asset amounts for minting cluster tokens
        create_asset_amounts: Vec<Uint128>,
        /// prices of the inventory assets in a cluster
        asset_prices: Vec<String>,
        /// current target weights of the assets in a cluster
        target_weights: Vec<Uint128>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnchorMsg {
    DepositStable {},
}

/// ## Description
/// This structure describes the available query messages for the oracle hub contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsgOracleHub {
    /// Queries the highes priority available price within the timeframe
    /// If timeframe is not provided, it will ignore the price age
    Price {
        asset_token: String,
        timeframe: Option<u64>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PriceResponse {
    pub rate: Decimal,
    pub last_updated: u64,
}

/// ## Description
/// A custom struct for each query that returns the actual mint amount and the subjected penalty.
#[derive(Serialize, Deserialize)]
pub struct PenaltyCreateResponse {
    /// Actual minted cluster token amount
    pub create_tokens: Uint128,
    /// Incurred penalty / reward from rebalance
    pub penalty: Uint128,
    /// Returned attributes to the caller
    pub attributes: Vec<Attribute>,
}
