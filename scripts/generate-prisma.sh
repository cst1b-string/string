#!/usr/bin/env bash
set -euo pipefail

echo "Generate prisma client..."

# get the current directory
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"

# go to project root
cd $DIR/..

cargo prisma generate --schema=./crates/cache-prisma/prisma/schema.prisma
cargo prisma generate --schema=./crates/lighthouse-prisma/prisma/schema.prisma
