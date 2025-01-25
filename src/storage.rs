use alloy::{
    rpc::types::{Filter, Log},
    transports::BoxFuture,
};
use sqlx::{Pool, Postgres, Row, Transaction};

use crate::indexer1::filter_id;

#[derive(Clone)]
pub struct PostgresLogStorage {
    pool: Pool<sqlx::Postgres>,
}

impl PostgresLogStorage {
    pub fn new(pool: Pool<sqlx::Postgres>) -> Self {
        Self { pool }
    }
}

impl PostgresLogStorage {
    // TODO insert events in bulk
    pub async fn insert_logs<
        P: Fn(
            &[Log],
            &mut Transaction<'static, Postgres>,
            u64,
        ) -> BoxFuture<'static, anyhow::Result<()>>,
    >(
        &self,
        chain_id: u64,
        logs: &[Log],
        filter_id: &str,
        new_last_observed_block: u64,
        log_processor: P,
    ) -> anyhow::Result<()> {
        let mut transaction = self.pool.begin().await?;
        log_processor(logs, &mut transaction, chain_id).await?;
        sqlx::query("UPDATE filters SET last_observed_block=$1 WHERE filter_id=$2")
            .bind::<i64>(new_last_observed_block.try_into()?)
            .bind(filter_id)
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await.map_err(Into::into)
    }

    pub async fn get_or_create_filter(
        &self,
        filter: &Filter,
        chain_id: u64,
    ) -> anyhow::Result<(u64, String)> {
        sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS filters (
                filter_id TEXT PRIMARY KEY,
                last_observed_block BIGINT,
                filter JSON NOT NULL
            );
        ",
        )
        .execute(&self.pool)
        .await?;

        let filter_id = filter_id(filter, chain_id);
        let last_observed_block =
            sqlx::query("SELECT last_observed_block FROM filters WHERE filter_id=$1;")
                .bind(&filter_id)
                .fetch_optional(&self.pool)
                .await?
                .map(|row| {
                    row.get::<i64, _>(0)
                        .try_into()
                        .map(|v| (v, filter_id.clone()))
                })
                .transpose()?;
        println!("{:?}, {:?}", filter_id, last_observed_block);
        match last_observed_block {
            Some((block, filter_id)) => Ok((block, filter_id)),
            None => sqlx::query(
                "INSERT INTO filters(filter_id, last_observed_block, filter)
                    VALUES ($1,$2,$3);",
            )
            .bind(&filter_id)
            .bind::<i64>(filter.get_from_block().unwrap_or(1).try_into()?)
            .bind(serde_json::to_value(filter)?)
            .execute(&self.pool)
            .await
            .map_err(Into::into)
            .map(|_| (filter.get_from_block().unwrap_or(1), filter_id)),
        }
    }
}
