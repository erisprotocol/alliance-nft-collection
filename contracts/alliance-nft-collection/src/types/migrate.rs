use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct MigrateMsg {
    pub version: String,
}
