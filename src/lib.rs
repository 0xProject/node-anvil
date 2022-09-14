use anvil::{
    eth::{
        backend::info::StorageInfo,
        fees::FeeHistoryService,
        miner::{Miner, MiningMode},
        pool::Pool,
        sign::{DevSigner, Signer as EthSigner},
        EthApi,
    },
    filter::Filters,
    NodeConfig,
};
use neon::prelude::*;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use tokio::runtime::Runtime;

use anvil_core::eth::EthRequest;

use std::{future::Future, sync::Arc, time::Duration};

// Return a global tokio runtime or create one if it doesn't exist.
// Throws a JavaScript exception if the `Runtime` fails to create.
fn runtime<'a, C: Context<'a>>(cx: &mut C) -> NeonResult<&'static Runtime> {
    static RUNTIME: OnceCell<Runtime> = OnceCell::new();

    RUNTIME.get_or_try_init(|| Runtime::new().or_else(|err| cx.throw_error(err.to_string())))
}

pub fn block_on<F: Future>(future: F) -> F::Output {
    let rt = tokio::runtime::Runtime::new().expect("could not start tokio rt");
    rt.block_on(future)
}

pub struct NodeAnvil {
    api: EthApi,
}

impl Finalize for NodeAnvil {}

impl NodeAnvil {
    pub fn js_new(mut cx: FunctionContext) -> JsResult<JsBox<NodeAnvil>> {
        let instance = NodeAnvil::new();
        Ok(cx.boxed(instance))
    }

    pub fn new() -> Self {
        let a = block_on(async move {
            let eth_api = init().await;
            eth_api
        });
        Self { api: a }
    }

    fn js_handle_request(mut cx: FunctionContext) -> JsResult<JsPromise> {
        let req = cx.argument::<JsString>(0)?.value(&mut cx);
        let value: serde_json::Value = serde_json::from_str(&req).unwrap();
        let request = serde_json::from_value::<EthRequest>(value).unwrap();
        let instance = cx
            .this()
            .downcast_or_throw::<JsBox<NodeAnvil>, _>(&mut cx)?;

        // Boilerplate for Neon/Promises/Tokio
        let rt = runtime(&mut cx)?;
        let channel = cx.channel();
        let (deferred, promise) = cx.promise();
        // hack: don't even use a promise straight up block
        let a = block_on(async move {
            let result = instance.api.execute(request).await;
            result
        });
        rt.spawn(async move {
            // Perform the actual work in an sync block
            // let result = instance.api.execute(request).await;
            deferred.settle_with(&channel, move |mut cx| {
                // Ok(cx.string("1234"))
                Ok(cx.string(serde_json::to_string(&a).unwrap()))
            });
        });
        // Return our promise initially
        Ok(promise)
    }
}

async fn init() -> EthApi {
    let mut config = NodeConfig::test();
    let logger = Default::default();

    let backend = Arc::new(config.setup().await);

    let fork = backend.get_fork().cloned();

    config.block_time = Some(Duration::new(0,0));

    let NodeConfig {
        signer_accounts,
        block_time,
        port,
        max_transactions,
        server_config,
        no_mining,
        transaction_order,
        ..
    } = config.clone();

    let pool = Arc::new(Pool::default());

    // let mode = if let Some(block_time) = block_time {
    //     MiningMode::interval(block_time)
    // } else if no_mining {
    //     MiningMode::None
    // } else {
    //     // get a listener for ready transactions
    //     let listener = pool.add_ready_listener();
    //     MiningMode::instant(max_transactions, listener)
    // };
    let listener = pool.add_ready_listener();
    let mode =    MiningMode::instant(max_transactions, listener);
    let miner = Miner::new(mode);

    let dev_signer: Box<dyn EthSigner> = Box::new(DevSigner::new(signer_accounts));
    let fees = backend.fees().clone();
    let fee_history_cache = Arc::new(Mutex::new(Default::default()));
    let fee_history_service = FeeHistoryService::new(
        backend.new_block_notifications(),
        Arc::clone(&fee_history_cache),
        fees,
        StorageInfo::new(Arc::clone(&backend)),
    );

    let filters = Filters::default();
    let api = EthApi::new(
        Arc::clone(&pool),
        Arc::clone(&backend),
        Arc::new(vec![dev_signer]),
        fee_history_cache,
        fee_history_service.fee_history_limit(),
        miner.clone(),
        logger,
        filters.clone(),
        transaction_order,
    );
    api
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("nodeAnvilNew", NodeAnvil::js_new)?;
    cx.export_function("nodeAnvilHandleRequest", NodeAnvil::js_handle_request)?;
    Ok(())
}
