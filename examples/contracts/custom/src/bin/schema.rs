use cosmwasm_schema::write_api;

use custom::contract::{ContractExecMsg, ContractQueryMsg, InstantiateMsg};

#[cfg(not(tarpaulin_include))]
fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ContractExecMsg,
        query: ContractQueryMsg,
    }
}
