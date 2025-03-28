--
-- Create Table JVM
--
DROP TABLE IF EXISTS JVM;
CREATE TABLE JVM (
    architecture TEXT NOT NULL,
    "checksum" TEXT,
    checksum_url TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    features TEXT,
    file_type TEXT NOT NULL,
    "filename" TEXT,
    image_type TEXT NOT NULL,
    java_version TEXT,
    jvm_impl TEXT,
    modified_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
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
-- Allow read/write for user roast
--
GRANT SELECT, INSERT, UPDATE, DELETE ON JVM TO roast;
