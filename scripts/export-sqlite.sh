#!/bin/bash
set -eo pipefail

db='data/meta.sqlite3'

function print_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -h, --help            Show this help message and exit"
    echo "  -db, --database-url   Specify the database URL (default: data/meta.sqlite3)"
}

# Parse command line arguments
while [[ "$#" -gt 0 ]]; do
    case $1 in
        -h|--help) print_usage; exit 0 ;;
        -db|--database-url) db="$2"; shift ;;
        *) echo "Unknown parameter passed: $1"; print_usage; exit 1 ;;
    esac
    shift
done

release_type=("ea" "ga")
os=("linux" "macosx" "windows")
arch=("aarch64" "arm32" "x86_64")

function sql() {
echo "SELECT
        architecture,
        features,
        file_type,
        filename,
        image_type,
        java_version,
        jvm_impl,
        md5,
        md5_url,
        os,
        release_type,
        sha1,
        sha1_url,
        sha256,
        sha256_url,
        sha512,
        sha512_url,
        url,
        vendor,
        version
    FROM
        JAVA_META_DATA
    WHERE
            file_type IN ('tar.gz','zip')
        AND release_type = '$1'
        AND os = '$2'
        AND architecture = '$3'
    ;";
}

for t in "${release_type[@]}"; do
  for o in "${os[@]}"; do
    for a in "${arch[@]}"; do
      path="data/$t/$o"
      mkdir -p $path
      file="$path/$a.json"
      sql=$(sql $t $o $a)

      sqlite3 $db ".mode json" ".once $file" "$sql"
    done
  done
done
