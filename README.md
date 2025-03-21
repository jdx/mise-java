# Roast a JVM Data Crawler

![Test](https://github.com/roele/roast/workflows/Build%20and%20Test/badge.svg)
![Update JVM Data](https://github.com/roele/roast/workflows/Update%20Data/badge.svg)

Roast is a data crawler that collects and stores information about JVM distributions from various vendors.

Supported distributions:

* [Corretto](https://aws.amazon.com/corretto/)
* [Dragonwell](https://cn.aliyun.com/product/dragonwell)
* [Eclipse Temurin](https://adoptium.net/)
* [GraalVM Community Edition](https://www.graalvm.org/)
* [JetBrains Runtime](https://github.com/JetBrains/JetBrainsRuntime/)
* [IBM Semeru](https://developer.ibm.com/languages/java/semeru-runtimes/)
* [Liberica](https://bell-sw.com/pages/downloads)
* [Mandrel](https://github.com/graalvm/mandrel)
* [Microsoft OpenJDK](https://www.microsoft.com/openjdk)
* [OpenJDK](https://jdk.java.net/)
* [Oracle JDK](https://www.oracle.com/java/)
* [SapMachine](https://sap.github.io/SapMachine/)
* [Tencent Kona JDK](https://www.tencentcloud.com/document/product/845/48051)
* [Trava OpenJDK](https://github.com/TravaOpenJDK/)
* [Zulu Community](https://www.azul.com/downloads/)

## Schema

See [OpenAPI Specification](public/api/) for the schema of the JVM data.

## Build & Run

### Create and initialize the database

#### Local Docker PostgreSQL

Assuming you have a PostgreSQL container `postgres` running with a user `postgres`.

```bash
docker exec -i -u postgres postgres psql -d postgres -c "DROP DATABASE roast;"
docker exec -i -u postgres postgres psql -d postgres -c "CREATE DATABASE roast;"
docker exec -i -u postgres postgres psql -d roast < ./sql/schema.sql
```

## Run

### Fetch JVM data from all vendors

```bash
env RUST_LOG=roast=INFO \
cargo run -- fetch 2>&1 | tee -a error.log
```

### Export JVM data as release_type/os/arch triplet

```bash
env RUST_LOG=roast=INFO \
cargo run -- export triplet 2>&1 | tee -a error.log
```

## Disclaimer

This project is in no way affiliated with any of the companies or projects offering and distributing the actual JREs and JDKs.

All respective copyrights and trademarks are theirs.
