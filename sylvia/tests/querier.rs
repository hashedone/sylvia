#![cfg(feature = "mt")]

use cosmwasm_std::{Addr, Response, StdResult};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sylvia::contract;
use sylvia::types::InstantiateCtx;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug, Default)]
pub struct CountResponse {
    pub count: u64,
}

pub mod counter {
    use cosmwasm_std::{Response, StdError, StdResult};
    use sylvia::types::{ExecCtx, QueryCtx};
    use sylvia::{contract, interface};

    use crate::CountResponse;

    #[interface]
    pub trait Counter {
        type Error: From<StdError>;

        #[msg(query)]
        fn count(&self, ctx: QueryCtx) -> StdResult<CountResponse>;

        #[msg(exec)]
        fn copy_count(&self, ctx: ExecCtx) -> StdResult<Response>;

        #[msg(exec)]
        fn set_count(&self, ctx: ExecCtx, new_count: u64) -> StdResult<Response>;

        #[msg(exec)]
        fn decrease_by_count(&self, ctx: ExecCtx) -> StdResult<Response>;
    }

    #[contract(module=super)]
    #[messages(crate::counter as Counter)]
    impl Counter for super::CounterContract<'_> {
        type Error = StdError;

        #[msg(query)]
        fn count(&self, ctx: QueryCtx) -> StdResult<CountResponse> {
            let count = self.count.load(ctx.deps.storage)?;
            Ok(CountResponse { count })
        }

        #[msg(exec)]
        fn set_count(&self, ctx: ExecCtx, new_count: u64) -> StdResult<Response> {
            self.count.save(ctx.deps.storage, &new_count)?;
            Ok(Response::new())
        }

        #[msg(exec)]
        fn copy_count(&self, ctx: ExecCtx) -> StdResult<Response> {
            let other_count = self
                .remote
                .load(ctx.deps.storage)?
                .querier(&ctx.deps.querier)
                .count()?
                .count;

            self.count.save(ctx.deps.storage, &other_count)?;
            Ok(Response::new())
        }

        #[msg(exec)]
        fn decrease_by_count(&self, ctx: ExecCtx) -> StdResult<Response> {
            let remote = self.remote.load(ctx.deps.storage)?;
            let other_count = BoundQuerier::borrowed(&remote.0, &ctx.deps.querier)
                .count()?
                .count;
            self.count.update(ctx.deps.storage, |count| {
                let count = count.saturating_sub(other_count);
                Ok::<_, StdError>(count)
            })?;

            Ok(Response::new())
        }
    }
}

pub struct CounterContract<'a> {
    pub count: Item<'static, u64>,
    pub remote: Item<'static, Remote<'a>>,
}

#[contract]
#[messages(counter as Counter)]
impl CounterContract<'_> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            count: Item::new("count"),
            remote: Item::new("remote"),
        }
    }

    #[msg(instantiate)]
    fn instantiate(&self, ctx: InstantiateCtx, remote_addr: Addr) -> StdResult<Response> {
        self.count.save(ctx.deps.storage, &0)?;
        self.remote
            .save(ctx.deps.storage, &Remote::new(remote_addr))?;
        Ok(Response::new())
    }
}

#[cfg(test)]
mod tests {

    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::{Addr, Empty, QuerierWrapper};
    use sylvia::multitest::App;

    use crate::counter::test_utils::Counter;
    use crate::multitest_utils::CodeId;

    #[test]
    fn querier_generation() {
        let deps = mock_dependencies();
        let querier_wrapper = QuerierWrapper::<Empty>::new(&deps.querier);
        let remote_addr = Addr::unchecked("remote");

        // Remote generation
        let remote = super::counter::Remote::new(remote_addr.clone());
        let _: super::counter::BoundQuerier<_> = remote.querier(&querier_wrapper);
        let remote = super::Remote::new(remote_addr.clone());
        let _: super::BoundQuerier<_> = remote.querier(&querier_wrapper);

        // Querier generation
        let _ = super::counter::BoundQuerier::borrowed(&remote_addr, &querier_wrapper);
        let querier = super::BoundQuerier::borrowed(&remote_addr, &querier_wrapper);

        let _ = super::counter::BoundQuerier::from(&querier);
    }

    #[test]
    fn call_querier() {
        let app = App::default();
        let code_id = CodeId::store_code(&app);

        let owner = "owner";

        let first_contract = code_id
            .instantiate(Addr::unchecked("remote"))
            .with_label("First Counter")
            .call(owner)
            .unwrap();

        let second_contract = code_id
            .instantiate(first_contract.contract_addr.clone())
            .with_label("Second Counter")
            .call(owner)
            .unwrap();

        first_contract
            .counter_proxy()
            .set_count(42)
            .call(owner)
            .unwrap();

        let resp = second_contract.counter_proxy().count().unwrap();
        assert_eq!(resp.count, 0);

        second_contract
            .counter_proxy()
            .copy_count()
            .call(owner)
            .unwrap();

        let resp = second_contract.counter_proxy().count().unwrap();
        assert_eq!(resp.count, 42);

        second_contract
            .counter_proxy()
            .decrease_by_count()
            .call(owner)
            .unwrap();

        let resp = second_contract.counter_proxy().count().unwrap();
        assert_eq!(resp.count, 0);
    }
}
