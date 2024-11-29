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
    sha1_file TEXT,
    sha256 TEXT NOT NULL,
    sha256_file TEXT,
    sha512 TEXT,
    sha512_file TEXT,
    "size" INTEGER,
    "url" TEXT NOT NULL,
    vendor TEXT,
    "version" TEXT,
    PRIMARY KEY ("url", sha256)
);
