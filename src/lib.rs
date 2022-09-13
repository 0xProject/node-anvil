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

use anvil_rpc::response::ResponseResult;

use std::sync::Arc;

// Return a global tokio runtime or create one if it doesn't exist.
// Throws a JavaScript exception if the `Runtime` fails to create.
fn runtime<'a, C: Context<'a>>(cx: &mut C) -> NeonResult<&'static Runtime> {
    static RUNTIME: OnceCell<Runtime> = OnceCell::new();

    RUNTIME.get_or_try_init(|| Runtime::new().or_else(|err| cx.throw_error(err.to_string())))
}

async fn init() -> EthApi {
    let mut config = NodeConfig::test();
    let logger = Default::default();

    let backend = Arc::new(config.setup().await);

    let fork = backend.get_fork().cloned();

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

    let mode = if let Some(block_time) = block_time {
        MiningMode::interval(block_time)
    } else if no_mining {
        MiningMode::None
    } else {
        // get a listener for ready transactions
        let listener = pool.add_ready_listener();
        MiningMode::instant(max_transactions, listener)
    };
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

async fn handle_request(request: EthRequest) -> ResponseResult {
    println!("{:?}", request);
    let api = init().await;
    let result = api.execute(request).await;
    println!("{:?}", result);
    result
}

fn perform_request(mut cx: FunctionContext) -> JsResult<JsPromise> {
    let req = cx.argument::<JsString>(0)?.value(&mut cx);
    let value: serde_json::Value = serde_json::from_str(&req).unwrap();
    let request = serde_json::from_value::<EthRequest>(value).unwrap();

    // Boilerplate for Neon/Promises/Tokio
    let rt = runtime(&mut cx)?;
    let channel = cx.channel();
    let (deferred, promise) = cx.promise();
    rt.spawn(async move {
        // Perform the actual work in an sync block
        let result = handle_request(request).await;
        // Fulfill the promise with the awaited value
        deferred.settle_with(&channel, move |mut cx| {
            Ok(cx.string(serde_json::to_string(&result).unwrap()))
        });
    });
    // Return our promise initially
    Ok(promise)
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("performRequest", perform_request)?;
    Ok(())
}
