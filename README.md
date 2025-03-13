# Roast a JVM Data Crawler

## Setup

### Create and initialize the database

#### Docker PostgreSQL

Assuming you have a PostgreSQL container `postgres` running with a user `postgres`.

```bash
docker exec -i -u postgres postgres psql -d postgres -c "DROP DATABASE roast;"
docker exec -i -u postgres postgres psql -d postgres -c "CREATE DATABASE roast;"
docker exec -i -u postgres postgres psql -d roast < ./sql/schema.sql
```

## Run

### Fetch JVM data from all vendors

```bash
env RUST_LOG=roast=DEBUG \
cargo run -- fetch 2>&1 | tee -a error.log
```

### Export JVM data as release_type/os/arch triplet

```bash
env RUST_LOG=roast=DEBUG \
cargo run -- export triplet 2>&1 | tee -a error.log
```
