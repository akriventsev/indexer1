//! Trait that describes the persistent storage used in indexing
use alloy::rpc::types::{Filter, Log};
use futures::Future;

use crate::indexer1::Processor;

pub mod postgres;
pub mod sqlite;

pub trait LogStorage {
    /// Type of transaction, &mut Self::Transaction will be propagated into processor function
    type Transaction: Sized;

    /// Method should open up a transaction, then invoke ```log_processor``` and update the ```last_observed_block```
    fn insert_logs<P: Processor<Self::Transaction>>(
        &self,
        chain_id: u64,
        logs: &[Log],
        filter_id: &str,
        prev_saved_block: u64,
        new_saved_block: u64,
        log_processor: &mut P,
    ) -> impl Future<Output = anyhow::Result<()>>;

    /// Method is used to lazily initialize filter store (if not) and extract filter from storage
    /// if it present. In case it doesn't it should create the instance and return
    /// ```from_block``` as a start point of indexing.
    /// Returns u64 block number and String filter_id
    fn get_or_create_filter(
        &self,
        filter: &Filter,
        chain_id: u64,
    ) -> impl Future<Output = anyhow::Result<(u64, String)>>;
}
