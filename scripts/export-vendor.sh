#!/bin/bash
set -eo pipefail

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "Docker is not running. Please start Docker and try again."
    exit 1
fi

db='postgresql://postgres:postgres@localhost:5432/roast'
container='postgres'
output_dir='data'
user='postgres'

function print_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -h, --help            Show this help message and exit"
    echo "  -db, --database-url   Specify the database URL (default: postgresql://roast:roast@localhost:5432/roast)"
    echo "  -c, --container       Specify the Docker container name (default: roast)"
    echo "  -o, --output-dir      Specify the output directory (default: data)"
    echo "  -u, --user            Specify the user for the export (default: roast)"
}

# Parse command line arguments
while [[ "$#" -gt 0 ]]; do
    case $1 in
        -h|--help) print_usage; exit 0 ;;
        -db|--database-url) db="$2"; shift ;;
        -c|--container) container="$2"; shift ;;
        -o|--output-dir) output_dir="$2"; shift ;;
        -u|--user) user="$2"; shift ;;
        *) echo "Unknown parameter passed: $1"; print_usage; exit 1 ;;
    esac
    shift
done

function sql() {
echo "SELECT
        architecture,
        checksum,
        checksum_url,
        features,
        file_type,
        filename,
        image_type,
        java_version,
        jvm_impl,
        os,
        release_type,
        url,
        vendor,
        version
    FROM
        JVM
    WHERE
        vendor = '$1'
        AND os = '$2'
        AND architecture = '$3'
        AND file_type IN ('tar.gz','zip')
    ";
}

vendor=("corretto" "dragonwell" "graalvm" "jetbrains" "kona" "liberica" "mandrel" "microsoft" "openjdk" "oracle-graalvm" "oracle" "sapmachine" "semeru" "temurin" "trava" "zulu")
os=("linux" "macosx" "windows")
arch=("aarch64" "arm32" "i686" "x86_64")

for v in "${vendor[@]}"; do
  for o in "${os[@]}"; do
    for a in "${arch[@]}"; do
      path="$output_dir/$v/$o"
      mkdir -p $path
      file="$path/$a.json"
      csv_file="$path/$a.csv"
      sql=$(sql $v $o $a)

      docker exec -i "$container" psql "$db" -U "$user" -c "\copy ($sql) TO STDOUT WITH CSV HEADER" | jq -R -s -f scripts/csv_to_json.jq > $file
    done
  done
done
