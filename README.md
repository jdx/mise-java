# Java Metadata Database

## Setup

### Create and initialize the database

#### Docker PostgreSQL

Assuming you have a PostgreSQL container `postgres` running with a user `postgres` and a database `postgres`.

```bash
docker exec -i -u postgres postgres psql -d postgres -c "DROP DATABASE meta;"
docker exec -i -u postgres postgres psql -d postgres -c "CREATE DATABASE meta;"
docker exec -i -u postgres postgres psql -d meta < ./sql/schema.sql
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
