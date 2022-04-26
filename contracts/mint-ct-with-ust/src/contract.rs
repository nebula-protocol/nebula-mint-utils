#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, coin, to_binary, Addr, Attribute, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    QuerierWrapper, QueryRequest, Response, StdError, StdResult, Uint128, WasmMsg, WasmQuery,
};

use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

use terra_cosmwasm::{create_swap_msg, TerraMsgWrapper, TerraQuerier};

use crate::msg::{
    AnchorMsg, ClusterStateResponse, ExecuteMsg, IncentivesMsg, InstantiateMsg,
    PenaltyCreateResponse, PriceResponse, QueryMsg, QueryMsgNebula, QueryMsgOracleHub,
    QueryMsgPenalty, SimulateMintResponse,
};
use crate::state::{State, STATE};
use astroport::asset::{Asset, AssetInfo};
use astroport::pair::ExecuteMsg as AstroportExecuteMsg;
use astroport::querier::{query_balance, query_pair_info, query_token_balance, simulate};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:mint-ct-with-ust";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DECIMAL_FRACTIONAL: Uint128 = Uint128::new(1_000_000_000u128); // 1*10**9

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let state = State {
        incentive_contract: deps.api.addr_validate(msg.incentive_contract.as_ref())?,
        astroport_factory_address: deps
            .api
            .addr_validate(msg.astroport_factory_address.as_ref())?,
        aust_token_address: deps.api.addr_validate(msg.aust_token_address.as_ref())?,
        anchor_market_contract: deps
            .api
            .addr_validate(msg.anchor_market_contract.as_ref())?,
        oracle_hub_contract: deps.api.addr_validate(msg.oracle_hub_contract.as_ref())?,
        owner_address: info.sender,
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner_address", state.owner_address)
        .add_attribute("astroport_factory_address", state.astroport_factory_address))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<TerraMsgWrapper>, StdError> {
    match msg {
        ExecuteMsg::MintCT { cluster_address } => mint_ct(deps, env, cluster_address, info.sender),
        ExecuteMsg::_MintCT {
            cluster_address,
            natives,
            tokens,
            cluster_token,
            user,
        } => _mint_ct(
            deps,
            env,
            cluster_address,
            natives,
            tokens,
            cluster_token,
            user,
        ),
        ExecuteMsg::_SendToUser {
            cluster_token,
            user,
        } => _send_to_user(deps, env, cluster_token, user),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::SimulateMint {
            cluster_address,
            ust_amount,
        } => to_binary(&simulate_mint(deps, env, cluster_address, ust_amount)?),
    }
}

pub fn _send_to_user(
    deps: DepsMut,
    env: Env,
    cluster_token: String,
    user: String,
) -> StdResult<Response<TerraMsgWrapper>> {
    let amount = query_token_balance(
        &deps.querier,
        deps.api.addr_validate(cluster_token.as_ref())?,
        env.contract.address,
    )?;

    Ok(
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cluster_token,
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: user,
                amount,
            })?,
            funds: vec![],
        })),
    )
}

pub fn _mint_ct(
    deps: DepsMut,
    env: Env,
    cluster_address: String,
    natives: Vec<String>,
    tokens: Vec<String>,
    cluster_token: String,
    user: String,
) -> StdResult<Response<TerraMsgWrapper>> {
    let state = STATE.load(deps.storage)?;
    let _validated_user = deps.api.addr_validate(user.as_str());

    let mut msgs: Vec<CosmosMsg<TerraMsgWrapper>> = vec![];
    let mut funds = vec![];
    let mut assets = vec![];
    let mut attrs: Vec<Attribute> = vec![];

    for native in natives {
        let amount = query_balance(&deps.querier, env.contract.address.clone(), native.clone())?;
        funds.push(coin(amount.u128(), native.clone()));
        assets.push(Asset {
            info: AssetInfo::NativeToken {
                denom: native.clone(),
            },
            amount,
        });

        attrs.push(attr("denom", native));
        attrs.push(attr("balance", amount));
    }

    for token in tokens {
        let contract_addr = deps.api.addr_validate(token.as_ref())?;
        let amount = query_token_balance(
            &deps.querier,
            contract_addr.clone(),
            env.contract.address.clone(),
        )?;
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                spender: state.incentive_contract.to_string(),
                amount,
                expires: None,
            })?,
            funds: vec![],
        }));
        assets.push(Asset {
            info: AssetInfo::Token { contract_addr },
            amount,
        });

        attrs.push(attr("token", token));
        attrs.push(attr("balance", amount));
    }

    funds.sort_by(|c1, c2| c1.denom.cmp(&c2.denom));

    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.incentive_contract.to_string(),
        msg: to_binary(&IncentivesMsg::IncentivesCreate {
            cluster_contract: cluster_address.clone(),
            asset_amounts: assets,
            min_tokens: None,
        })?,
        funds,
    }));

    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::_SendToUser {
            cluster_token,
            user,
        })?,
        funds: vec![],
    }));

    Ok(Response::new().add_messages(msgs).add_attributes(attrs))
}

pub fn swap_to_ust(
    querier: &QuerierWrapper,
    offer_asset: Asset,
    astroport_factory_address: Addr,
) -> Result<CosmosMsg<TerraMsgWrapper>, StdError> {
    let pair_contract = query_pair_info(
        querier,
        astroport_factory_address.clone(),
        &[
            offer_asset.info.clone(),
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        ],
    )?
    .contract_addr
    .to_string();

    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: pair_contract,
        msg: to_binary(&AstroportExecuteMsg::Swap {
            offer_asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: offer_asset.amount.clone(),
            },
            belief_price: None,
            max_spread: None,
            to: None,
        })?,
        funds: vec![coin(offer_asset.amount.u128(), "uusd")],
    }))
}

pub fn mint_ct(
    deps: DepsMut,
    env: Env,
    cluster_address: String,
    user: Addr,
) -> StdResult<Response<TerraMsgWrapper>> {
    let state = STATE.load(deps.storage)?;
    let validated_cluster_address = deps.api.addr_validate(cluster_address.as_ref())?;
    let cluster_state = get_cluster_state(deps.as_ref(), &validated_cluster_address)?;
    let total_target_weight: Uint128 = cluster_state.target.clone().iter().map(|x| x.amount).sum();
    let ust_amt = query_balance(
        &deps.querier,
        env.contract.address.clone(),
        "uusd".to_string(),
    )?;

    let mut natives: Vec<String> = vec![];
    let mut tokens: Vec<String> = vec![];
    let mut attrs: Vec<Attribute> = vec![];
    let mut msgs: Vec<CosmosMsg<TerraMsgWrapper>> = vec![];

    for asset in cluster_state.target.clone() {
        let asset_ratio = ust_amt * asset.amount / total_target_weight;

        match asset.info.clone() {
            AssetInfo::NativeToken { denom } => {
                natives.push(denom.clone());
                if denom == "uusd" {
                    continue;
                }
                attrs.push(attr("swap_ust_to_native_", denom.clone()));
                attrs.push(attr("amount", asset_ratio.clone()));

                msgs.push(create_swap_msg(coin(asset_ratio.into(), "uusd"), denom))
            }
            AssetInfo::Token { contract_addr } => {
                tokens.push(contract_addr.to_string());
                attrs.push(attr("swap_ust_to_token_", contract_addr.clone()));
                attrs.push(attr("amount", asset_ratio.clone()));

                if contract_addr != state.aust_token_address {
                    msgs.push(swap_to_ust(
                        &deps.querier,
                        Asset {
                            info: asset.info.clone(),
                            amount: asset_ratio,
                        },
                        state.astroport_factory_address.clone(),
                    )?);
                } else {
                    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: state.anchor_market_contract.to_string(),
                        msg: to_binary(&AnchorMsg::DepositStable {})?,
                        funds: vec![coin(asset_ratio.u128(), "uusd")],
                    }));
                }
            }
        }
    }

    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::_MintCT {
            cluster_address: cluster_address,
            tokens,
            natives,
            cluster_token: cluster_state.cluster_token,
            user: user.to_string(),
        })?,
        funds: vec![],
    }));

    Ok(Response::new().add_messages(msgs).add_attributes(attrs))
}

/// ## Description
/// Returns the state of a cluster.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **cluster** is a reference to an object of type [`Addr`] which is
///     the address of a cluster.
pub fn get_cluster_state(deps: Deps, cluster: &Addr) -> StdResult<ClusterStateResponse> {
    // Query the cluster state
    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: cluster.to_string(),
        msg: to_binary(&QueryMsgNebula::ClusterState {})?,
    }))
}

pub fn get_price(deps: Deps, contract: &Addr, asset_token: String) -> StdResult<PriceResponse> {
    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract.to_string(),
        msg: to_binary(&QueryMsgOracleHub::Price {
            asset_token,
            timeframe: None,
        })?,
    }))
}

pub fn get_penalty_query_create(
    deps: Deps,
    contract: &Addr,
    block_height: u64,
    cluster_token_supply: Uint128,
    inventory: Vec<Uint128>,
    create_asset_amounts: Vec<Uint128>,
    asset_prices: Vec<String>,
    target_weights: Vec<Uint128>,
) -> StdResult<PenaltyCreateResponse> {
    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract.to_string(),
        msg: to_binary(&QueryMsgPenalty::PenaltyQueryCreate {
            block_height,
            cluster_token_supply,
            inventory,
            create_asset_amounts,
            asset_prices,
            target_weights,
        })?,
    }))
}

pub fn simulate_mint(
    deps: Deps,
    env: Env,
    cluster_address: String,
    ust_amt: Uint128,
) -> StdResult<SimulateMintResponse> {
    let state = STATE.load(deps.storage)?;
    let validated_cluster_address = deps.api.addr_validate(cluster_address.as_ref())?;
    let cluster_state = get_cluster_state(deps, &validated_cluster_address)?;
    let total_target_weight: Uint128 = cluster_state.target.clone().iter().map(|x| x.amount).sum();
    let mut create_asset_amounts: Vec<Uint128> = vec![];
    let terra_querier = TerraQuerier::new(&deps.querier);

    for asset in cluster_state.target.clone() {
        let asset_ratio = ust_amt * asset.amount / total_target_weight;

        match asset.info.clone() {
            AssetInfo::NativeToken { denom } => {
                if denom == "uusd" {
                    create_asset_amounts.push(asset_ratio)
                } else {
                    let return_amount = terra_querier
                        .query_swap(coin(asset_ratio.u128(), "uusd"), denom)?
                        .receive
                        .amount;

                    create_asset_amounts.push(return_amount)
                }
            }
            AssetInfo::Token { contract_addr } => {
                if contract_addr != state.aust_token_address {
                    let pair_contract = query_pair_info(
                        &deps.querier,
                        state.astroport_factory_address.clone(),
                        &[
                            asset.info.clone(),
                            AssetInfo::NativeToken {
                                denom: "uusd".to_string(),
                            },
                        ],
                    )?
                    .contract_addr;

                    let return_amount = simulate(
                        &deps.querier,
                        pair_contract,
                        &Asset {
                            info: AssetInfo::NativeToken {
                                denom: "uusd".to_string(),
                            },
                            amount: asset_ratio,
                        },
                    )?
                    .return_amount;

                    create_asset_amounts.push(return_amount)
                } else {
                    let price = get_price(
                        deps,
                        &state.oracle_hub_contract.clone(),
                        contract_addr.to_string(),
                    )?
                    .rate;

                    let return_amount =
                        asset_ratio.multiply_ratio(DECIMAL_FRACTIONAL, price * DECIMAL_FRACTIONAL);

                    create_asset_amounts.push(return_amount)
                }
            }
        }
    }

    let penalty = get_penalty_query_create(
        deps,
        &deps.api.addr_validate(cluster_state.penalty.as_ref())?,
        env.block.height,
        cluster_state.outstanding_balance_tokens,
        cluster_state.inv,
        create_asset_amounts.clone(),
        cluster_state.prices,
        cluster_state
            .target
            .iter()
            .map(|asset| asset.amount)
            .collect(),
    )?;

    Ok(SimulateMintResponse {
        create_tokens: penalty.create_tokens,
        penalty: penalty.penalty,
        attributes: penalty.attributes,
        create_asset_amounts,
    })
}
