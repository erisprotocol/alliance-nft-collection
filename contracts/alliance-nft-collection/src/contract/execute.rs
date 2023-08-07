use cosmwasm_std::{coins, entry_point, to_binary, Addr, BankMsg, Binary, Coin as CwCoin, CosmosMsg, Order, SubMsg, Uint128, WasmMsg};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use cw721::Cw721Query;
use terra_proto_rs::alliance::alliance::{MsgClaimDelegationRewards, MsgRedelegate, MsgUndelegate};
use terra_proto_rs::{
    alliance::alliance::MsgDelegate, cosmos::base::v1beta1::Coin, traits::Message,
};

use crate::state::{
    reduce_val_stake, upsert_val, BROKEN_NFTS, CONFIG, NFT_BALANCE_CLAIMED, NUM_ACTIVE_NFTS,
    REWARD_BALANCE, TEMP_BALANCE, VALS,
};
use crate::types::execute::{
    AllianceDelegateMsg, AllianceRedelegateMsg, AllianceUndelegateMsg, MintMsg,
};
use crate::types::{errors::ContractError, execute::ExecuteMsg, AllianceNftCollection};

const CLAIM_REWARD_ERROR_REPLY_ID: u64 = 1;
const DENOM: &str = "uluna";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let parent: AllianceNftCollection = AllianceNftCollection::default();
    match msg {
        ExecuteMsg::AllianceDelegate(msg) => try_alliance_delegate(deps, env, info, msg),
        ExecuteMsg::AllianceUndelegate(msg) => try_alliance_undelegate(deps, env, info, msg),
        ExecuteMsg::AllianceRedelegate(msg) => try_alliance_redelegate(deps, env, info, msg),

        ExecuteMsg::AllianceClaimRewards {} => try_alliance_claim_rewards(deps, env, info),
        ExecuteMsg::UpdateRewardsCallback {} => update_reward_callback(deps, env, info),

        ExecuteMsg::BreakNft(token_id) => try_breaknft(deps, env, info, parent, token_id),
        ExecuteMsg::Mint(mint_msg) => try_mint(deps, info, parent, mint_msg),

        _ => Ok(parent.execute(deps, env, info, msg.into())?),
    }
}

fn try_alliance_claim_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let num_of_active = NUM_ACTIVE_NFTS.load(deps.storage)?;
    if num_of_active == 0 {
       return Err(ContractError::NoActiveNfts {})
    }

    let reward_sent_in_tx: Option<&CwCoin> = info.funds.iter().find(|c| c.denom == DENOM);
    let sent_balance = if let Some(coin) = reward_sent_in_tx {
        coin.amount
    } else {
        Uint128::zero()
    };
    let contract_balance = deps
        .querier
        .query_balance(env.contract.address.clone(), DENOM)?
        .amount;

    TEMP_BALANCE.save(deps.storage, &(contract_balance - sent_balance))?;
    let validators = VALS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|v| v.unwrap().0)
        .collect::<Vec<String>>();
    let sub_msgs: Vec<SubMsg> = validators
        .iter()
        .map(|v| {
            let msg = MsgClaimDelegationRewards {
                delegator_address: env.contract.address.to_string(),
                validator_address: v.to_string(),
                denom: cfg.asset_denom.clone(),
            };
            let msg = CosmosMsg::Stargate {
                type_url: "/alliance.alliance.MsgClaimDelegationRewards".to_string(),
                value: Binary::from(msg.encode_to_vec()),
            };
            // Reply on error here is used to ignore errors from claiming rewards with validators that we did not delegate to
            SubMsg::reply_on_error(msg, CLAIM_REWARD_ERROR_REPLY_ID)
        })
        .collect();
    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::UpdateRewardsCallback {}).unwrap(),
        funds: vec![],
    });

    Ok(Response::new()
        .add_attributes(vec![("action", "update_rewards")])
        .add_submessages(sub_msgs)
        .add_message(msg))
}

fn update_reward_callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    authorize_execution(env.contract.address.clone(), info.sender)?;

    let current_balance = deps
        .querier
        .query_balance(env.contract.address, DENOM)?
        .amount;
    let previous_balance = TEMP_BALANCE.load(deps.storage)?;
    let rewards_collected = current_balance - previous_balance;
    let num_of_active_nfts = NUM_ACTIVE_NFTS.load(deps.storage)?;
    let average_reward = rewards_collected / Uint128::from(num_of_active_nfts);
    REWARD_BALANCE.update(deps.storage, |balance| -> Result<_, ContractError> {
        Ok(balance + average_reward)
    })?;

    TEMP_BALANCE.remove(deps.storage);
    Ok(Response::new().add_attributes(vec![("action", "update_rewards_callback")]))
}

fn try_alliance_delegate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: AllianceDelegateMsg,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.as_ref().storage)?;
    authorize_execution(cfg.owner.clone(), info.sender)?;
    let mut cosmos_msg: Vec<CosmosMsg> = Vec::new();

    for del in msg.delegations.iter() {
        let msg = CosmosMsg::Stargate {
            type_url: "/alliance.alliance.MsgDelegate".to_string(),
            value: Binary::from(
                MsgDelegate {
                    delegator_address: env.contract.address.to_string(),
                    validator_address: del.validator.clone(),
                    amount: Some(Coin {
                        denom: cfg.asset_denom.clone(),
                        amount: del.amount.to_string(),
                    }),
                }
                .encode_to_vec(),
            ),
        };

        upsert_val(deps.storage, del.validator.clone(), del.amount)?;
        cosmos_msg.push(msg);
    }

    Ok(Response::default()
        .add_attribute("method", "try_alliance_delegate")
        .add_messages(cosmos_msg))
}

fn try_alliance_undelegate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: AllianceUndelegateMsg,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    authorize_execution(cfg.owner.clone(), info.sender)?;

    if msg.undelegations.is_empty() {
        return Err(ContractError::EmptyDelegation {});
    }
    let mut msgs = vec![];
    for delegation in msg.undelegations {
        let undelegate_msg = MsgUndelegate {
            amount: Some(Coin {
                denom: cfg.asset_denom.clone(),
                amount: delegation.amount.to_string(),
            }),
            delegator_address: env.contract.address.to_string(),
            validator_address: delegation.validator.to_string(),
        };
        let msg = CosmosMsg::Stargate {
            type_url: "/alliance.alliance.MsgUndelegate".to_string(),
            value: Binary::from(undelegate_msg.encode_to_vec()),
        };
        msgs.push(msg);
        reduce_val_stake(deps.storage, delegation.validator, delegation.amount)?;
    }
    Ok(Response::new()
        .add_attributes(vec![("action", "try_alliance_undelegate")])
        .add_messages(msgs))
}

fn try_alliance_redelegate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: AllianceRedelegateMsg,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    authorize_execution(cfg.owner.clone(), info.sender)?;

    if msg.redelegations.is_empty() {
        return Err(ContractError::EmptyDelegation {});
    }
    let mut msgs = vec![];
    for redelegation in msg.redelegations {
        let src_validator = redelegation.src_validator;
        let dst_validator = redelegation.dst_validator;
        let redelegate_msg = MsgRedelegate {
            amount: Some(Coin {
                denom: cfg.asset_denom.clone(),
                amount: redelegation.amount.to_string(),
            }),
            delegator_address: env.contract.address.to_string(),
            validator_src_address: src_validator.to_string(),
            validator_dst_address: dst_validator.to_string(),
        };
        let msg = CosmosMsg::Stargate {
            type_url: "/alliance.alliance.MsgRedelegate".to_string(),
            value: Binary::from(redelegate_msg.encode_to_vec()),
        };
        msgs.push(msg);
        upsert_val(deps.storage, dst_validator, redelegation.amount)?;
        reduce_val_stake(deps.storage, src_validator, redelegation.amount)?;
    }
    Ok(Response::new()
        .add_attributes(vec![("action", "try_alliance_redelegate")])
        .add_messages(msgs))
}

fn try_breaknft(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    parent: AllianceNftCollection,
    token_id: String,
) -> Result<Response, ContractError> {
    let owner_res = parent.owner_of(deps.as_ref(), env, token_id.clone(), false)?;
    let owner = deps.api.addr_validate(&owner_res.owner)?;
    authorize_execution(owner.clone(), info.sender)?;

    BROKEN_NFTS.update(deps.storage, token_id.clone(), |b| -> Result<_, ContractError> {
        match b {
            Some(b) => if b { Err(ContractError::AlreadyBroken {}) } else { Ok(true) },
            None => Ok(true)
        }
    })?;

    let rewards_claimed = NFT_BALANCE_CLAIMED.load(deps.storage, token_id.to_string())?;
    let average_rewards = REWARD_BALANCE.load(deps.storage)?;
    let rewards_claimable = average_rewards - rewards_claimed;

    NFT_BALANCE_CLAIMED.save(deps.storage, token_id.clone(), &average_rewards)?;
    NUM_ACTIVE_NFTS.update(deps.storage, |n| -> Result<_, ContractError> { Ok(n - 1) })?;
    if rewards_claimable.is_zero() {
        Ok(Response::default().add_attributes(vec![
            ("action", "break_nft"),
            ("token_id", token_id.as_str()),
            ("rewards", rewards_claimable.to_string().as_str()),
        ]))
    } else {
        let send_msg = CosmosMsg::Bank(BankMsg::Send {
            amount: coins(rewards_claimable.u128(), DENOM),
            to_address: owner.to_string(),
        });
        Ok(Response::default()
            .add_message(send_msg)
            .add_attributes(vec![
                ("action", "break_nft"),
                ("token_id", token_id.as_str()),
                ("rewards", rewards_claimable.to_string().as_str()),
            ]))
    }
}

fn try_mint(
    deps: DepsMut,
    info: MessageInfo,
    parent: AllianceNftCollection,
    mint_msg: MintMsg,
) -> Result<Response, ContractError> {
    // authorization will be checked by the NFT parent contract
    NUM_ACTIVE_NFTS.update(deps.storage, |n| -> Result<_, ContractError> { Ok(n + 1) })?;
    let reward_balance = REWARD_BALANCE.load(deps.storage)?;
    NFT_BALANCE_CLAIMED.save(deps.storage, mint_msg.token_id.clone(), &reward_balance)?;
    parent
        .mint(
            deps,
            info,
            mint_msg.token_id,
            mint_msg.owner,
            mint_msg.token_uri,
            mint_msg.extension,
        )
        .map_err(ContractError::FromContractError)
}

fn authorize_execution(owner: Addr, sender: Addr) -> Result<Response, ContractError> {
    if sender != owner {
        return Err(ContractError::Unauthorized(sender, owner));
    }
    Ok(Response::default())
}
