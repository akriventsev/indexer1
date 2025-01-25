use std::time::Duration;

use alloy::{
    providers::{ProviderBuilder, RootProvider, WsConnect},
    pubsub::PubSubFrontend,
    rpc::types::{Filter, Log},
    transports::http::{reqwest::Url, Client, Http},
};
use anyhow::{anyhow, Context, Result};
use futures::future::BoxFuture;
use sqlx::{PgPool, Postgres, Transaction};

use crate::{indexer1::Indexer, storage::PostgresLogStorage};

pub struct IndexerBuilder<
    P: Fn(&[Log], &mut Transaction<'static, Postgres>, u64) -> BoxFuture<'static, anyhow::Result<()>>,
> {
    http_provider: Option<RootProvider<Http<Client>>>,
    ws_provider: Option<RootProvider<PubSubFrontend>>,
    poll_interval: Option<Duration>,
    filter: Option<Filter>,
    processor: Option<P>,
    postgres_pool: Option<PgPool>,
}

impl<P> Default for IndexerBuilder<P>
where
    P: Fn(
        &[Log],
        &mut Transaction<'static, Postgres>,
        u64,
    ) -> BoxFuture<'static, anyhow::Result<()>>,
{
    fn default() -> Self {
        Self {
            http_provider: None,
            ws_provider: None,
            poll_interval: None,
            filter: None,
            processor: None,
            postgres_pool: None,
        }
    }
}

impl<
        P: Fn(
            &[Log],
            &mut Transaction<'static, Postgres>,
            u64,
        ) -> BoxFuture<'static, anyhow::Result<()>>,
    > IndexerBuilder<P>
{
    pub fn http_rpc_url(mut self, url: Url) -> Self {
        let provider = ProviderBuilder::new().on_http(url);
        self.http_provider = Some(provider);
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

    pub async fn ws_rpc_url(mut self, url: Url) -> Result<Self> {
        let ws_provider = ProviderBuilder::new()
            .on_ws(WsConnect::new(url.to_string()))
            .await
            .with_context(|| anyhow!("Failed to connect to rpc via WS"))?;
        self.ws_provider = Some(ws_provider);
        Ok(self)
    }

    pub fn fetch_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = Some(interval);
        self
    }

    pub async fn pg_url(mut self, url: &str) -> Result<Self> {
        let pg_pool = PgPool::connect(url)
            .await
            .with_context(|| anyhow!("Failed to connect to postgres"))?;
        self.postgres_pool = Some(pg_pool);
        Ok(self)
    }

    pub fn pg_connection(mut self, pool: PgPool) -> Self {
        self.postgres_pool = Some(pool);
        self
    }

    pub async fn build(self) -> anyhow::Result<Indexer<P>> {
        let http_provider = self
            .http_provider
            .ok_or(anyhow!("Http porvider is missing"))?;
        let ws_provider = self.ws_provider;

        let processor = self.processor.ok_or(anyhow!("Processor is missing"))?;
        let filter = self.filter.ok_or(anyhow!("Filter is missing"))?;
        let fetch_interval = self
            .poll_interval
            .ok_or(anyhow!("Fetch interval is missing"))?;
        let postgres =
            PostgresLogStorage::new(self.postgres_pool.ok_or(anyhow!("Postgres is missing"))?);

        Indexer::new(
            processor,
            filter,
            http_provider,
            ws_provider,
            fetch_interval,
            postgres,
        )
        .await
    }
}
