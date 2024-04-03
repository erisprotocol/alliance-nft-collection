use crate::contract::execute::execute;
use crate::tests::helpers::{
    break_nft, claim_alliance_emissions, mint, query_config, query_nft, query_rewards,
    setup_contract, MOCK_DAO_TREASURY_ADDRESS, MOCK_LST,
};
use alliance_nft_packages::eris::{AssetInfoExt, Hub};
use alliance_nft_packages::errors::ContractError;
use alliance_nft_packages::execute::{
    ExecuteCollectionMsg, MintMsg, UpdateConfigMsg, UpdateRewardsCallbackMsg,
};
use alliance_nft_packages::query::RewardsResponse;
use alliance_nft_packages::state::{Config, Trait, ALLOWED_DENOM};
use alliance_nft_packages::Extension;
use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{attr, Addr, Decimal, OwnedDeps, Response, Uint128};
use cw721::NftInfoResponse;
use cw_asset::{Asset, AssetInfo, AssetInfoUnchecked};

use super::custom_querier::CustomQuerier;

#[test]
fn mint_and_query_nft() {
    let mut deps = mock_dependencies();
    setup_contract(&mut deps);

    let res = mint(deps.as_mut(), "1");

    assert_eq!(
        res,
        Response::default().add_attributes(vec![
            ("action", "mint"),
            ("minter", "minter"),
            ("owner", "owner"),
            ("token_id", "1"),
        ])
    );

    let nft = query_nft(deps.as_ref(), "1");
    assert_eq!(
        nft,
        NftInfoResponse {
            token_uri: None,
            extension: Extension {
                image: Some("image".to_string()),
                image_data: None,
                external_url: None,
                description: None,
                name: None,
                attributes: Some(vec![
                    Trait {
                        display_type: None,
                        trait_type: "trait_type".to_string(),
                        value: "value".to_string()
                    },
                    Trait {
                        display_type: None,
                        trait_type: "broken".to_string(),
                        value: "false".to_string()
                    },
                    Trait {
                        display_type: None,
                        trait_type: "rewards".to_string(),
                        value: "0".to_string()
                    },
                ]),
                background_color: None,
                animation_url: None,
                youtube_url: None
            }
        }
    );
}

#[test]
fn mint_invalid() {
    let mut deps = mock_dependencies();
    setup_contract(&mut deps);
    mint(deps.as_mut(), "1");

    // Mint with the same token id
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info("minter", &[]),
        ExecuteCollectionMsg::Mint(MintMsg {
            owner: "owner".to_string(),
            token_id: "1".to_string(),
            token_uri: None,
            extension: Extension {
                image: Some("image".to_string()),
                image_data: None,
                external_url: None,
                description: None,
                name: None,
                attributes: Some(vec![Trait {
                    display_type: None,
                    trait_type: "trait_type".to_string(),
                    value: "value".to_string(),
                }]),
                background_color: None,
                animation_url: None,
                youtube_url: None,
            },
        }),
    )
    .unwrap_err();

    // Mint with the wrong minter
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info("wrong_minter", &[]),
        ExecuteCollectionMsg::Mint(MintMsg {
            owner: "owner".to_string(),
            token_id: "2".to_string(),
            token_uri: None,
            extension: Extension {
                image: Some("image".to_string()),
                image_data: None,
                external_url: None,
                description: None,
                name: None,
                attributes: Some(vec![Trait {
                    display_type: None,
                    trait_type: "trait_type".to_string(),
                    value: "value".to_string(),
                }]),
                background_color: None,
                animation_url: None,
                youtube_url: None,
            },
        }),
    )
    .unwrap_err();
}

#[test]
fn break_nft_without_rewards() {
    let mut deps = mock_dependencies();
    setup_contract(&mut deps);

    mint(deps.as_mut(), "1");

    let res = break_nft(deps.as_mut(), "1");
    assert_eq!(
        res,
        Response::default().add_attributes(vec![
            ("action", "break_nft"),
            ("token_id", "1"),
            ("rewards", "0"),
        ])
    );

    let nft = query_nft(deps.as_ref(), "1");
    assert_eq!(
        nft,
        NftInfoResponse {
            token_uri: None,
            extension: Extension {
                image: Some("image".to_string()),
                image_data: None,
                external_url: None,
                description: None,
                name: None,
                attributes: Some(vec![
                    Trait {
                        display_type: None,
                        trait_type: "trait_type".to_string(),
                        value: "value".to_string()
                    },
                    Trait {
                        display_type: None,
                        trait_type: "broken".to_string(),
                        value: "true".to_string()
                    },
                    Trait {
                        display_type: None,
                        trait_type: "rewards".to_string(),
                        value: "0".to_string()
                    },
                ]),
                background_color: None,
                animation_url: None,
                youtube_url: None
            }
        }
    );
}

#[test]
fn break_nft_with_rewards() {
    let mut deps = mock_dependencies();

    setup_contract(&mut deps);
    mint(deps.as_mut(), "1");
    mint(deps.as_mut(), "2");
    claim_alliance_emissions(&mut deps, Uint128::new(500_000_000));

    let nft = query_nft(deps.as_ref(), "1");
    assert_eq!(
        nft,
        NftInfoResponse {
            token_uri: None,
            extension: Extension {
                image: Some("image".to_string()),
                image_data: None,
                external_url: None,
                description: None,
                name: None,
                attributes: Some(vec![
                    Trait {
                        display_type: None,
                        trait_type: "trait_type".to_string(),
                        value: "value".to_string()
                    },
                    Trait {
                        display_type: None,
                        trait_type: "broken".to_string(),
                        value: "false".to_string()
                    },
                    Trait {
                        display_type: None,
                        trait_type: "rewards".to_string(),
                        // 500 LUNA -> 416.66 ampLUNA
                        // 41.66 ampLUNA treasury share
                        // 375 ampLUNA total rewards
                        // 187.5 ampLUNA per NFT
                        value: "187500000".to_string()
                    },
                ]),
                background_color: None,
                animation_url: None,
                youtube_url: None
            }
        }
    );

    let res = break_nft(deps.as_mut(), "1");
    assert_eq!(
        res,
        Response::default()
            .add_message(
                AssetInfo::cw20(Addr::unchecked(MOCK_LST))
                    .with_balance(Uint128::new(187500000))
                    .transfer_msg("owner")
                    .unwrap()
            )
            .add_attributes(vec![
                attr("action", "break_nft"),
                attr("token_id", "1"),
                attr("rewards", "187500000"),
                attr("user_share", "0.5"),
            ])
    );

    // remove share
    let previouse_balance = deps.querier.get_cw20_balance(MOCK_LST, MOCK_CONTRACT_ADDR);
    deps.querier.set_cw20_balance(
        MOCK_LST,
        MOCK_CONTRACT_ADDR,
        previouse_balance - 187500000u128,
    );

    let nft = query_nft(deps.as_ref(), "1");
    assert_eq!(
        nft,
        NftInfoResponse {
            token_uri: None,
            extension: Extension {
                image: Some("image".to_string()),
                image_data: None,
                external_url: None,
                description: None,
                name: None,
                attributes: Some(vec![
                    Trait {
                        display_type: None,
                        trait_type: "trait_type".to_string(),
                        value: "value".to_string()
                    },
                    Trait {
                        display_type: None,
                        trait_type: "broken".to_string(),
                        value: "true".to_string()
                    },
                    Trait {
                        display_type: None,
                        trait_type: "rewards".to_string(),
                        value: "0".to_string()
                    },
                ]),
                background_color: None,
                animation_url: None,
                youtube_url: None
            }
        }
    );

    // Claim more rewards from alliance module. All rewards should go to remaining NFTs
    claim_alliance_emissions(&mut deps, Uint128::new(500_000_000));
    let nft = query_nft(deps.as_ref(), "1");
    assert_eq!(
        nft,
        NftInfoResponse {
            token_uri: None,
            extension: Extension {
                image: Some("image".to_string()),
                image_data: None,
                external_url: None,
                description: None,
                name: None,
                attributes: Some(vec![
                    Trait {
                        display_type: None,
                        trait_type: "trait_type".to_string(),
                        value: "value".to_string()
                    },
                    Trait {
                        display_type: None,
                        trait_type: "broken".to_string(),
                        value: "true".to_string()
                    },
                    Trait {
                        display_type: None,
                        trait_type: "rewards".to_string(),
                        value: "0".to_string()
                    },
                ]),
                background_color: None,
                animation_url: None,
                youtube_url: None
            }
        }
    );

    let nft = query_nft(deps.as_ref(), "2");
    assert_eq!(
        nft,
        NftInfoResponse {
            token_uri: None,
            extension: Extension {
                image: Some("image".to_string()),
                image_data: None,
                external_url: None,
                description: None,
                name: None,
                attributes: Some(vec![
                    Trait {
                        display_type: None,
                        trait_type: "trait_type".to_string(),
                        value: "value".to_string()
                    },
                    Trait {
                        display_type: None,
                        trait_type: "broken".to_string(),
                        value: "false".to_string()
                    },
                    Trait {
                        display_type: None,
                        trait_type: "rewards".to_string(),
                        // 187.5 (from before) + 375 (total added) for the single NFT
                        value: "562500000".to_string()
                    },
                ]),
                background_color: None,
                animation_url: None,
                youtube_url: None
            }
        }
    );

    let nft = query_rewards(deps.as_ref(), "2");
    assert_eq!(
        nft,
        RewardsResponse {
            rewards: vec![Asset::cw20(Addr::unchecked(MOCK_LST), 562500000u128)]
        }
    );
}

#[test]
fn break_nft_with_multi_rewards() {
    let mut deps = mock_dependencies();

    setup_contract(&mut deps);
    mint(deps.as_mut(), "1");
    mint(deps.as_mut(), "2");
    claim_alliance_emissions(&mut deps, Uint128::new(500_000_000));

    // add multi-coin rewards
    let info = mock_info("owner", &[]);
    let env = mock_env();
    let msg = ExecuteCollectionMsg::UpdateConfig(UpdateConfigMsg {
        set_whitelisted_reward_assets: Some(vec![AssetInfoUnchecked::cw20("random")]),
        dao_treasury_address: None,
        dao_treasury_share: None,
        add_whitelisted_reward_assets: None,
    });
    execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    deps.querier
        .set_cw20_balance("random", MOCK_CONTRACT_ADDR, 10_000000u128);

    let nft = query_nft(deps.as_ref(), "1");
    assert_eq!(
        nft,
        NftInfoResponse {
            token_uri: None,
            extension: Extension {
                image: Some("image".to_string()),
                image_data: None,
                external_url: None,
                description: None,
                name: None,
                attributes: Some(vec![
                    Trait {
                        display_type: None,
                        trait_type: "trait_type".to_string(),
                        value: "value".to_string()
                    },
                    Trait {
                        display_type: None,
                        trait_type: "broken".to_string(),
                        value: "false".to_string()
                    },
                    Trait {
                        display_type: None,
                        trait_type: "rewards".to_string(),
                        // 500 LUNA -> 416.66 ampLUNA
                        // 41.66 ampLUNA treasury share
                        // 375 ampLUNA total rewards
                        // 187.5 ampLUNA per NFT
                        value: "187500000".to_string()
                    },
                ]),
                background_color: None,
                animation_url: None,
                youtube_url: None
            }
        }
    );

    let res = break_nft(deps.as_mut(), "1");
    assert_eq!(
        res,
        Response::default()
            .add_message(
                AssetInfo::cw20(Addr::unchecked(MOCK_LST))
                    .with_balance(Uint128::new(187500000))
                    .transfer_msg("owner")
                    .unwrap()
            )
            .add_message(
                AssetInfo::cw20(Addr::unchecked("random"))
                    .with_balance(Uint128::new(5_000000u128))
                    .transfer_msg("owner")
                    .unwrap()
            )
            .add_attributes(vec![
                attr("action", "break_nft"),
                attr("token_id", "1"),
                attr("rewards", "187500000"),
                attr("user_share", "0.5"),
            ])
    );

    // remove share
    let previouse_balance = deps.querier.get_cw20_balance(MOCK_LST, MOCK_CONTRACT_ADDR);
    deps.querier.set_cw20_balance(
        MOCK_LST,
        MOCK_CONTRACT_ADDR,
        previouse_balance - 187500000u128,
    );
    let previouse_balance = deps.querier.get_cw20_balance("random", MOCK_CONTRACT_ADDR);
    deps.querier.set_cw20_balance(
        "random",
        MOCK_CONTRACT_ADDR,
        previouse_balance - 5_000000u128,
    );

    let nft = query_nft(deps.as_ref(), "1");
    assert_eq!(
        nft,
        NftInfoResponse {
            token_uri: None,
            extension: Extension {
                image: Some("image".to_string()),
                image_data: None,
                external_url: None,
                description: None,
                name: None,
                attributes: Some(vec![
                    Trait {
                        display_type: None,
                        trait_type: "trait_type".to_string(),
                        value: "value".to_string()
                    },
                    Trait {
                        display_type: None,
                        trait_type: "broken".to_string(),
                        value: "true".to_string()
                    },
                    Trait {
                        display_type: None,
                        trait_type: "rewards".to_string(),
                        value: "0".to_string()
                    },
                ]),
                background_color: None,
                animation_url: None,
                youtube_url: None
            }
        }
    );

    // Claim more rewards from alliance module. All rewards should go to remaining NFTs
    claim_alliance_emissions(&mut deps, Uint128::new(500_000_000));
    let nft = query_nft(deps.as_ref(), "1");
    assert_eq!(
        nft,
        NftInfoResponse {
            token_uri: None,
            extension: Extension {
                image: Some("image".to_string()),
                image_data: None,
                external_url: None,
                description: None,
                name: None,
                attributes: Some(vec![
                    Trait {
                        display_type: None,
                        trait_type: "trait_type".to_string(),
                        value: "value".to_string()
                    },
                    Trait {
                        display_type: None,
                        trait_type: "broken".to_string(),
                        value: "true".to_string()
                    },
                    Trait {
                        display_type: None,
                        trait_type: "rewards".to_string(),
                        value: "0".to_string()
                    },
                ]),
                background_color: None,
                animation_url: None,
                youtube_url: None
            }
        }
    );

    let nft = query_nft(deps.as_ref(), "2");
    assert_eq!(
        nft,
        NftInfoResponse {
            token_uri: None,
            extension: Extension {
                image: Some("image".to_string()),
                image_data: None,
                external_url: None,
                description: None,
                name: None,
                attributes: Some(vec![
                    Trait {
                        display_type: None,
                        trait_type: "trait_type".to_string(),
                        value: "value".to_string()
                    },
                    Trait {
                        display_type: None,
                        trait_type: "broken".to_string(),
                        value: "false".to_string()
                    },
                    Trait {
                        display_type: None,
                        trait_type: "rewards".to_string(),
                        // 187.5 (from before) + 375 (total added) for the single NFT
                        value: "562500000".to_string()
                    },
                ]),
                background_color: None,
                animation_url: None,
                youtube_url: None
            }
        }
    );

    let nft = query_rewards(deps.as_ref(), "2");
    assert_eq!(
        nft,
        RewardsResponse {
            rewards: vec![
                Asset::cw20(Addr::unchecked(MOCK_LST), 562500000u128),
                Asset::cw20(Addr::unchecked("random"), 5_000000u128)
            ]
        }
    );
}

#[test]
fn break_nft_invalid() {
    let mut deps = mock_dependencies();
    setup_contract(&mut deps);
    mint(deps.as_mut(), "1");

    // Cannot break as a different owner
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner2", &[]),
        ExecuteCollectionMsg::BreakNft("1".to_string()),
    )
    .unwrap_err();
    break_nft(deps.as_mut(), "1");

    // Cannot break a broken NFT
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteCollectionMsg::BreakNft("1".to_string()),
    )
    .unwrap_err();
}

#[test]
fn stake_reward_callback() {
    let mut deps = mock_dependencies();
    setup_contract(&mut deps);
    mint(deps.as_mut(), "1");

    // Cannot break as a different owner
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("anyone", &[]),
        ExecuteCollectionMsg::StakeRewardsCallback {},
    )
    .unwrap_err();

    assert_eq!(
        err,
        ContractError::Unauthorized(
            Addr::unchecked("anyone"),
            Addr::unchecked(MOCK_CONTRACT_ADDR)
        )
    );

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteCollectionMsg::StakeRewardsCallback {},
    )
    .unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "stake_reward_callback"),
            attr("tokens_to_stake", "0")
        ]
    );
    deps.querier.set_bank_balance(100u128);

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteCollectionMsg::StakeRewardsCallback {},
    )
    .unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "stake_reward_callback"),
            attr("tokens_to_stake", "100")
        ]
    );
}

#[test]
fn update_rewards_callback() {
    let mut deps = mock_dependencies();
    setup_contract(&mut deps);
    mint(deps.as_mut(), "1");

    // Cannot break as a different owner
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("anyone", &[]),
        ExecuteCollectionMsg::UpdateRewardsCallback(UpdateRewardsCallbackMsg {
            previous_lst_balance: Uint128::zero(),
        }),
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::Unauthorized(
            Addr::unchecked("anyone"),
            Addr::unchecked(MOCK_CONTRACT_ADDR)
        )
    );

    deps.querier
        .set_cw20_balance(MOCK_LST, MOCK_CONTRACT_ADDR, 100u128);

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteCollectionMsg::UpdateRewardsCallback(UpdateRewardsCallbackMsg {
            previous_lst_balance: Uint128::zero(),
        }),
    )
    .unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "update_rewards_callback"),
            attr("rewards_collected", "90"),
            attr("treasury_amount", "10")
        ]
    );
}

#[test]
fn update_config_invalid() {
    let mut deps = mock_dependencies();
    setup_contract(&mut deps);

    let info = mock_info("user", &[]);
    let env = mock_env();
    let msg = ExecuteCollectionMsg::UpdateConfig(UpdateConfigMsg {
        dao_treasury_address: None,
        dao_treasury_share: None,
        set_whitelisted_reward_assets: None,
        add_whitelisted_reward_assets: Some(vec![AssetInfoUnchecked::cw20("random")]),
    });
    let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();

    assert_eq!(
        err,
        ContractError::Unauthorized(Addr::unchecked("user"), Addr::unchecked("owner"))
    );

    let info = mock_info("owner", &[]);
    let env = mock_env();
    let msg = ExecuteCollectionMsg::UpdateConfig(UpdateConfigMsg {
        dao_treasury_address: None,
        dao_treasury_share: None,
        set_whitelisted_reward_assets: None,
        add_whitelisted_reward_assets: Some(vec![
            AssetInfoUnchecked::cw20("random"),
            AssetInfoUnchecked::cw20(MOCK_LST),
        ]),
    });
    let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(err, ContractError::InvalidWhitelistedAssetInfo {});

    let msg = ExecuteCollectionMsg::UpdateConfig(UpdateConfigMsg {
        dao_treasury_address: None,
        dao_treasury_share: None,
        set_whitelisted_reward_assets: Some(vec![AssetInfoUnchecked::native(ALLOWED_DENOM)]),
        add_whitelisted_reward_assets: Some(vec![AssetInfoUnchecked::cw20("random")]),
    });
    let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(err, ContractError::InvalidWhitelistedAssetInfo {});

    let msg = ExecuteCollectionMsg::UpdateConfig(UpdateConfigMsg {
        dao_treasury_address: None,
        dao_treasury_share: Some(Decimal::percent(21)),
        set_whitelisted_reward_assets: None,
        add_whitelisted_reward_assets: None,
    });
    let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(err, ContractError::InvalidDaoTreasuryShare {});
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies();
    setup_contract(&mut deps);

    let config = query_config(deps.as_ref());
    assert_eq!(
        config,
        Config {
            asset_denom: format!("factory/{}/{}", MOCK_CONTRACT_ADDR, "AllianceNFT"),
            owner: Addr::unchecked("owner"),
            dao_treasury_share: Decimal::percent(10),
            lst_asset_info: cw_asset::AssetInfoBase::Cw20(Addr::unchecked(MOCK_LST)),
            lst_hub_address: Hub(Addr::unchecked("hub")),
            dao_treasury_address: Addr::unchecked(MOCK_DAO_TREASURY_ADDRESS),
            whitelisted_reward_assets: vec![]
        }
    );

    let info = mock_info("owner", &[]);
    let env = mock_env();
    let msg = ExecuteCollectionMsg::UpdateConfig(UpdateConfigMsg {
        dao_treasury_address: Some("new_treasury".to_string()),
        dao_treasury_share: Some(Decimal::zero()),
        set_whitelisted_reward_assets: Some(vec![
            AssetInfoUnchecked::cw20("random"),
            AssetInfoUnchecked::cw20("random2"),
            AssetInfoUnchecked::native("usdc"),
        ]),
        add_whitelisted_reward_assets: Some(vec![
            AssetInfoUnchecked::cw20("random2"),
            AssetInfoUnchecked::native("usdc"),
            AssetInfoUnchecked::native("usdc2"),
        ]),
    });

    execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let config = query_config(deps.as_ref());
    assert_eq!(
        config,
        Config {
            asset_denom: format!("factory/{}/{}", MOCK_CONTRACT_ADDR, "AllianceNFT"),
            owner: Addr::unchecked("owner"),
            dao_treasury_share: Decimal::percent(0),
            lst_asset_info: cw_asset::AssetInfoBase::Cw20(Addr::unchecked(MOCK_LST)),
            lst_hub_address: Hub(Addr::unchecked("hub")),
            dao_treasury_address: Addr::unchecked("new_treasury"),
            whitelisted_reward_assets: vec![
                AssetInfo::cw20(Addr::unchecked("random")),
                AssetInfo::cw20(Addr::unchecked("random2")),
                AssetInfo::native("usdc"),
                AssetInfo::native("usdc2"),
            ]
        }
    );
}

pub(super) fn mock_dependencies() -> OwnedDeps<MockStorage, MockApi, CustomQuerier> {
    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: CustomQuerier::default(),
        custom_query_type: std::marker::PhantomData,
    }
}
