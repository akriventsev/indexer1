CREATE TABLE IF NOT EXISTS filters (
    filter_id TEXT PRIMARY KEY,
    last_observed_block BIGINT,
    filter JSON NOT NULL
);
