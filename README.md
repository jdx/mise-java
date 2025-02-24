# Java Metadata Database

## Setup

### Create and initialize the database

#### SQLite

Assuming you have `sqlite3` installed.

```bash
sqlite3 data/meta.sqlite3 < sql/schema.sql
```

#### Docker PostgreSQL

Assuming you have a PostgreSQL container `postgres` running with a user `postgres` and a database `postgres`.

```bash
docker exec -it -u postgres postgres psql -d postgres -c "DROP DATABASE jmeta;"
docker exec -it -u postgres postgres psql -d postgres -c "CREATE DATABASE jmeta;"
docker exec -it -u postgres postgres psql -d jmeta < ./sql/schema.sql
```

## Run

### Fetch meta data from all vendors

```bash
env RUST_LOG=jmeta=DEBUG \
cargo run -- fetch 2>&1 | tee -a error.log
```

### Export meta data as release_type/os/arch triplet

```bash
env RUST_LOG=jmeta=DEBUG \
cargo run -- export triplet 2>&1 | tee -a error.log
```
