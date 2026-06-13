--
-- Create Table JVM
--
DROP TABLE IF EXISTS JVM;
CREATE TABLE JVM (
    architecture TEXT NOT NULL,
    "checksum" TEXT,
    checksum_url TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    features TEXT,
    file_type TEXT NOT NULL,
    "filename" TEXT,
    image_type TEXT NOT NULL,
    java_version TEXT,
    jvm_impl TEXT,
    modified_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    os TEXT NOT NULL,
    release_type TEXT NOT NULL,
    "size" INTEGER,
    "url" TEXT NOT NULL,
    vendor TEXT NOT NULL,
    "version" TEXT NOT NULL,
    /* should match the Hash/PartialEq implementation of JvmData (src/jvm/mod.rs) */
    PRIMARY KEY(url)
);

--
-- Create Indexes on JVM
--
DROP INDEX IF EXISTS JVM_IDX_ARCHITECTURE;
CREATE INDEX JVM_IDX_ARCHITECTURE ON JVM (architecture);

DROP INDEX IF EXISTS JVM_IDX_OS;
CREATE INDEX JVM_IDX_OS ON JVM (os);

DROP INDEX IF EXISTS JVM_IDX_VENDOR;
CREATE INDEX JVM_IDX_VENDOR ON JVM (vendor);

DROP INDEX IF EXISTS JVM_IDX_VERSION;
CREATE INDEX JVM_IDX_VERSION ON JVM ("version");

--
-- Create View JVM_VIEW for data mappings
--
-- Maps linux rows with musl feature to os=alpine-linux
--
DROP VIEW IF EXISTS JVM_VIEW;
CREATE VIEW JVM_VIEW AS
SELECT
    architecture,
    checksum,
    checksum_url,
    created_at,
    features,
    file_type,
    filename,
    image_type,
    java_version,
    jvm_impl,
    modified_at,
    CASE
        WHEN os = 'linux' AND features LIKE '%musl%' THEN 'alpine-linux'
        ELSE os
    END AS os,
    release_type,
    size,
    url,
    vendor,
    version
FROM JVM
--
-- For backwards compatibility (remove in near future)
--
UNION
SELECT
    architecture,
    checksum,
    checksum_url,
    created_at,
    features,
    file_type,
    filename,
    image_type,
    java_version,
    jvm_impl,
    modified_at,
    os,
    release_type,
    size,
    url,
    vendor,
    version
FROM JVM
;
