--
--
--
DROP TABLE IF EXISTS JAVA_META_DATA;

--
--
--
CREATE TABLE JAVA_META_DATA (
    architecture TEXT,
    features TEXT,
    file_type TEXT,
    "filename" TEXT,
    image_type TEXT,
    java_version TEXT,
    jvm_impl TEXT,
    md5 TEXT,
    md5_file TEXT,
    os TEXT,
    release_type TEXT,
    sha1 TEXT,
    sha1_url TEXT,
    sha256 TEXT,
    sha256_url TEXT,
    sha512 TEXT,
    sha512_url TEXT,
    "size" INTEGER,
    "url" TEXT NOT NULL,
    vendor TEXT,
    "version" TEXT,
    PRIMARY KEY ("url")
);

-- Create Index on JAVA_META_DATA for
-- * architecture
-- * os
-- * vendor
CREATE INDEX JAVA_META_DATA_IDX_ARCHITECTURE ON JAVA_META_DATA (architecture);
CREATE INDEX JAVA_META_DATA_IDX_OS ON JAVA_META_DATA (os);
CREATE INDEX JAVA_META_DATA_IDX_VENDOR ON JAVA_META_DATA (vendor);
