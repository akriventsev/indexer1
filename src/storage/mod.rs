use alloy::rpc::types::{Filter, Log};
use futures::Future;

use crate::indexer1::Processor;

pub mod postgres;
pub mod sqlite;

pub trait LogStorage {
    fn insert_logs<P: Processor>(
        &self,
        chain_id: u64,
        logs: &[Log],
        filter_id: &str,
        new_last_observed_block: u64,
        log_processor: &mut P,
    ) -> impl Future<Output = anyhow::Result<()>>;

    fn get_or_create_filter(
        &self,
        filter: &Filter,
        chain_id: u64,
    ) -> impl Future<Output = anyhow::Result<(u64, String)>>;
}
