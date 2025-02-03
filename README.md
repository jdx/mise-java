# Java Metadata Database

## Setup

### Create and initialize the database

```bash
sqlite3 data/meta.sqlite3 < data/schema.sql
```

## Run

```bash
cargo build --all-features
env RUST_LOG=jmeta=DEBUG target/debug/jmeta fetch 2>&1 | tee error.log
```
