# Roast a JVM Data Crawler

![Test](https://github.com/roele/roast/workflows/Build%20and%20Test/badge.svg)
![Update JVM Data](https://github.com/roele/roast/workflows/Update%20Data/badge.svg)

Roast is a data crawler that collects and stores information about JVM distributions from various vendors. The project
is heavily based on the [Java Metadata](https://github.com/joschi/java-metadata) project.

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
* [Oracle GraalVM](https://www.graalvm.org/)
* [SapMachine](https://sap.github.io/SapMachine/)
* [Tencent Kona JDK](https://www.tencentcloud.com/document/product/845/48051)
* [Trava OpenJDK](https://github.com/TravaOpenJDK/)
* [Zulu Community](https://www.azul.com/downloads/)

## Schema

| Field name     | Description                           |
| -------------- | ------------------------------------- |
| `architecture` | Supported machine architecture        |
| `checksum`     | Checksum of the artifact              |
| `checksum_url` | Checksum URI of the artifact          |
| `features`     | Features of the distribution          |
| `file_type`    | File extension of the artifact        |
| `filename`     | Filename of the artifact              |
| `release_type` | `ga` (stable) or `ea` (early access)  |
| `image_type`   | JRE (`jre`) or JDK (`jdk`)            |
| `jvm_impl`     | JVM implementation                    |
| `java_version` | Java version the artifact is based on |
| `os`           | Supported operating system            |
| `size`         | Size of the artifact in bytes         |
| `url`          | Full source URL of the artifact       |
| `vendor`       | JVM vendor name                       |
| `version`      | Version of the JVM distribution       |

## Build & Run

### Create and initialize the database

#### Local Docker PostgreSQL

Assuming you have a PostgreSQL container `postgres` running with a user `postgres`.

```bash
docker exec -i -u postgres postgres psql -d postgres -c "DROP DATABASE roast;"
docker exec -i -u postgres postgres psql -d postgres -c "CREATE DATABASE roast;"
docker exec -i -u postgres postgres psql -d roast -c "CREATE USER roast WITH PASSWORD 'roast';"
docker exec -i -u postgres postgres psql -d roast < ./sql/schema.sql
```

## Run

### Environment variables

Roast uses a configuration file `config.toml` to configure the database connection and other settings.
You can use the following environment variables to override the default configuration in `config.toml`.

| Variable name              | Description                                  |
| -------------------------- | -------------------------------------------- |
| `ROAST_DATABASE_POOL_SIZE` | Number of threads to use for fetching data   |
| `ROAST_DATABASE_URL`       | PostgreSQL connection string                 |
| `ROAST_DATABASE_SSL_MODE`  | SSL mode for PostgreSQL connection           |
| `ROAST_DATABASE_SSL_CA`    | CA certificate for PostgreSQL connection     |
| `ROAST_DATABASE_SSL_CERT`  | Client certificate for PostgreSQL connection |
| `ROAST_DATABASE_SSL_KEY`   | Client key for PostgreSQL connection         |

Additionally, you can set the following environment variables to configure the logging and threading.

| Variable name       | Description                                                           |
| ------------------- | --------------------------------------------------------------------- |
| `RAYON_NUM_THREADS` | Number of threads to use by the Rayon module                          |
| `RUST_LOG`          | Log configuration (see https://docs.rs/env_logger/latest/env_logger/) |

### Fetch data from all vendors

```bash
env \
RAYON_NUM_THREADS=50 \
RUST_LOG=roast=INFO \
cargo run -- fetch 2>&1 | tee -a error.log
```

### Export data as release_type/os/arch triplet

```bash
env \
RAYON_NUM_THREADS=50 \
RUST_LOG=roast=INFO \
cargo run -- export triplet 2>&1 | tee -a error.log
```

## Disclaimer

This project is in no way affiliated with any of the companies or projects offering and distributing the actual JREs and JDKs.
All respective copyrights and trademarks are theirs.
