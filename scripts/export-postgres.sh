#!/bin/bash
set -eo pipefail

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "Docker is not running. Please start Docker and try again."
    exit 1
fi

db='postgresql://postgres:postgres@localhost:5432/meta'
container='postgres'
user='postgres'

function print_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -h, --help            Show this help message and exit"
    echo "  -db, --database-url   Specify the database URL (default: postgresql://postgres:postgres@localhost:5432/meta)"
    echo "  -c, --container       Specify the Docker container name (default: postgres)"
    echo "  -u, --user            Specify the user for the export (default: postgres)"
}

# Parse command line arguments
while [[ "$#" -gt 0 ]]; do
    case $1 in
        -h|--help) print_usage; exit 0 ;;
        -db|--database-url) db="$2"; shift ;;
        -c|--container) container="$2"; shift ;;
        -u|--user) user="$2"; shift ;;
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
    ";
}

for t in "${release_type[@]}"; do
  for o in "${os[@]}"; do
    for a in "${arch[@]}"; do
      path="data/$t/$o"
      mkdir -p $path
      file="$path/$a.json"
      csv_file="$path/$a.csv"
      sql=$(sql $t $o $a)

      docker exec -i "$container" psql "$db" -U "$user" -c "\copy ($sql) TO STDOUT WITH CSV HEADER" | jq -R -s -f scripts/csv_to_json.jq > $file
    done
  done
done
