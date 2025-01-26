# Indexer1
A Rust library for indexing EVM chains

Indexer1 is a library that provides a mechanism to index EVM-compatible blockchains.  
It uses PostgreSQL to ensure consistent and fault-tolerant indexing, however re-implementing storage trait is available.
Something similar to The Graph as it also indexes your data, however Indexer1 is lightweight, self-hosted and more simple. 
If you need a more professional and extendable way of syncing, you should definately prefer Indexer1
```rust
pub struct TestProcessor;

impl Processor for TestProcessor {
    async fn process<Postgres>(
        &mut self,
        _logs: &[alloy::rpc::types::Log],
        _transaction: &mut sqlx::Transaction<'static, Postgres>,
        _chain_id: u64,
    ) -> anyhow::Result<()> {
        // put here any code to collect logs
        Ok(())
    }
}

Indexer::builder()
    .http_rpc_url(http_url)
    .ws_rpc_url(ws_url)
    .fetch_interval(Duration::from_secs(10))
    // Add event filter
    .filter(Filter::new().address(contract_address).events([
        MockERC20::Transfer::SIGNATURE,
        MockERC20::Approval::SIGNATURE,
    ]))
    // Set up a function to process events
    .set_processor(TestProcessor)
    .sqlite_storage(pool)
    .build()
    .await
    .unwrap()
    .run()
    .await?;
```

# Usage

## Import indexer1 as a library to your existing code
Add to your Cargo.toml
```
[dependencies]

indexer1 = "0.1.0"
```

## Start with a new indexer1 template
Ensure you have cargo-generate or install it with
```bash
$cargo install cargo-generate --locked
```
Then simply use the template
```bash
$cargo generate gh:badconfig/indexer1-template
```
Replate all todos and you are ready to go

# Features

## Reliable and consistent indexing

The core or Indexer1 is fetching block range via ```eth_getLogs``` and opening a PostgreSQL (or any other ACID storage) transaction 
for every new batch of logs received. Transaction will also update last observed block automatically. In case that 
something fails and events are also inserted into database, everything will be rolled back, including last seen block.
This allows to ensure that all events are inserted only once, keeping service tolerant to errors.

## Efficient indexing

Indexer1 can work with WS rpc to support HTTP indexing. In this case, it will subscribe to new logs and any time 
the data is received in WS it will count that there are changes that it can poll with HTTP. This allows to make 
HTTP polling interval significantly lower, as usually Websockets are enough reliable to notify you about every 
change happened.

## Uniqe filter id

Each filter is hashed into a unique filter id. It hashes the following parameters: 
* ChainId
* StartBlock
* Topics
* Addresses
In case that any of this data changed, Indexer1 will create a new filterId and will index this filter from start.

## Flexibility

Indexer1 is a library that can be used within any existing database and combined with other application code.
You can create several Indexers working with the same Postgres (or any other ACID) database. 
It only takes to spawn thread in tokio runtime and is very lightweight
