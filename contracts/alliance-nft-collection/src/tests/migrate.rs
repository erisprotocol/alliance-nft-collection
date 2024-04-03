use std::borrow::BorrowMut;

use crate::{
    contract::{instantiate::CONTRACT_VERSION, migrate::migrate},
    state::REWARD_BALANCE,
    tests::helpers::{MOCK_DAO_TREASURY_ADDRESS, MOCK_LST},
};
use alliance_nft_packages::{
    eris::Hub,
    migrate::MigrateMsg,
    state::{ConfigV100, ALLOWED_DENOM},
};
use cosmwasm_std::{attr, Addr, Decimal, Uint128};
use cw_storage_plus::Item;

use super::instantiate::intantiate_with_reply;

#[test]
fn test_migrate() {
    let (mut deps, env, _) = intantiate_with_reply();

    // mock old storage
    let old = ConfigV100 {
        owner: Addr::unchecked("owner"),
        asset_denom: "factory/cosmos2contract/AllianceNFT".to_string(),
    };
    let item: Item<ConfigV100> = Item::new("cfg");
    item.save(deps.storage.borrow_mut(), &old).unwrap();
    REWARD_BALANCE
        .save(deps.storage.borrow_mut(), &Uint128::new(120000u128))
        .unwrap();
    deps.querier.set_bank_balance(120000u128 * 10000u128);

    // migrate to different version (without applying 1.1.0)
    let res = migrate(
        deps.as_mut(),
        env.clone(),
        MigrateMsg {
            nft_collection_code_id: Some(4711),
            version: "1.0.0".to_string(),
            version110_data: Some(alliance_nft_packages::migrate::Version110MigrateData {
                dao_treasury_share: Decimal::percent(10),
                lst_asset_info: cw_asset::AssetInfoBase::Cw20(MOCK_LST.to_string()),
                lst_hub: "hub".to_string(),
                dao_treasury_address: MOCK_DAO_TREASURY_ADDRESS.to_string(),
            }),
        },
    )
    .unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("method", "try_migrate"),
            // is already 1.1.0 as we only mock the item state
            attr("version", CONTRACT_VERSION)
        ]
    );
    assert_eq!(res.messages.len(), 0);

    // migrate to 1.1.0
    let res = migrate(
        deps.as_mut(),
        env,
        MigrateMsg {
            nft_collection_code_id: Some(4711),
            version: "1.1.0".to_string(),
            version110_data: Some(alliance_nft_packages::migrate::Version110MigrateData {
                dao_treasury_share: Decimal::percent(10),
                lst_asset_info: cw_asset::AssetInfoBase::Cw20(MOCK_LST.to_string()),
                lst_hub: "hub".to_string(),
                dao_treasury_address: MOCK_DAO_TREASURY_ADDRESS.to_string(),
            }),
        },
    )
    .unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("method", "migrate_to_1_1_0"),
            // always the previously set version
            attr("version", "1.0.0"),
            attr("balance_native", "1200000000"),
            attr("rewards_current", "120000"),
            attr("rewards_in_lst", "100000")
        ],
    );

    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0].msg,
        Hub(Addr::unchecked("hub"))
            .bond_msg(ALLOWED_DENOM, 1200000000u128, None)
            .unwrap()
    );
}
