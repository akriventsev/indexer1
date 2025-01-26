use std::time::Duration;

use alloy::{
    providers::{ProviderBuilder, RootProvider, WsConnect},
    pubsub::PubSubFrontend,
    rpc::types::Filter,
    transports::http::{reqwest::Url, Client, Http},
};
use anyhow::{anyhow, Context};
use futures::future::Either;
use sqlx::{PgPool, SqlitePool};

use crate::{
    indexer1::{Indexer, Processor},
    storage::LogStorage,
};

pub struct IndexerBuilder<P: Processor, S: LogStorage> {
    http_provider: Option<Either<Url, RootProvider<Http<Client>>>>,
    ws_provider: Option<Either<Url, RootProvider<PubSubFrontend>>>,
    poll_interval: Option<Duration>,
    filter: Option<Filter>,
    processor: Option<P>,
    storage: Option<S>,
}

impl<P: Processor, S: LogStorage> Default for IndexerBuilder<P, S> {
    fn default() -> Self {
        Self {
            http_provider: None,
            ws_provider: None,
            poll_interval: None,
            filter: None,
            processor: None,
            storage: None,
        }
    }
}

impl<P: Processor> IndexerBuilder<P, PgPool> {
    pub fn pg_storage(mut self, pool: PgPool) -> Self {
        self.storage = Some(pool);
        self
    }
}

impl<P: Processor> IndexerBuilder<P, SqlitePool> {
    pub fn sqlite_storage(mut self, pool: SqlitePool) -> Self {
        self.storage = Some(pool);
        self
    }
}

impl<P: Processor, S: LogStorage> IndexerBuilder<P, S> {
    pub fn http_rpc_url(mut self, url: Url) -> Self {
        self.http_provider = Some(Either::Left(url));
        self
    }

    pub fn filter(mut self, filter_data: Filter) -> Self {
        self.filter = Some(filter_data);
        self
    }

    pub fn set_processor(mut self, function: P) -> Self {
        self.processor = Some(function);
        self
    }

    pub fn ws_rpc_url(mut self, url: Url) -> Self {
        self.ws_provider = Some(Either::Left(url));
        self
    }

    pub fn ws_rpc_url_opt(mut self, url: Option<Url>) -> Self {
        self.ws_provider = url.map(Either::Left);
        self
    }

    pub fn fetch_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = Some(interval);
        self
    }

    pub async fn build(self) -> anyhow::Result<Indexer<P, S>> {
        let http_provider = match self
            .http_provider
            .ok_or(anyhow!("Http porvider is missing"))?
        {
            Either::Left(url) => ProviderBuilder::new().on_http(url),
            Either::Right(p) => p,
        };

        let ws_provider = match self.ws_provider {
            Some(d) => match d {
                Either::Left(url) => Some(
                    ProviderBuilder::new()
                        .on_ws(WsConnect::new(url.to_string()))
                        .await
                        .with_context(|| anyhow!("Failed to connect to rpc via WS"))?,
                ),
                Either::Right(p) => Some(p),
            },
            None => None,
        };

        let processor = self.processor.ok_or(anyhow!("Processor is missing"))?;
        let filter = self.filter.ok_or(anyhow!("Filter is missing"))?;
        let fetch_interval = self
            .poll_interval
            .ok_or(anyhow!("Fetch interval is missing"))?;

        let storage = self.storage.ok_or(anyhow!("Storage is missing"))?;

        Indexer::new(
            processor,
            filter,
            http_provider,
            ws_provider,
            fetch_interval,
            storage,
        )
        .await
    }
}
