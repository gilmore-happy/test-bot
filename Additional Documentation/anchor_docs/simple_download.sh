#!/bin/bash

# Set up variables
BASE_URL="https://www.anchor-lang.com/docs"
OUTPUT_DIR="C:/Users/tonal/Documents/1HFT Solana Bot/Additional Documentation/anchor_docs/simple-pages"

# Create directory if it doesn't exist
mkdir -p "$OUTPUT_DIR"

# List of important pages to download
PAGES=(
  ""  # Main docs page
  "/installation"
  "/quickstart"
  "/quickstart/solpg"
  "/quickstart/local"
  "/basics"
  "/basics/program-structure"
  "/basics/idl"
  "/basics/pda"
  "/basics/cpi"
  "/clients"
  "/clients/typescript"
  "/clients/rust"
  "/testing"
  "/testing/litesvm"
  "/testing/mollusk"
  "/features"
  "/features/declare-program"
  "/features/errors"
  "/features/events"
  "/features/zero-copy"
  "/tokens"
  "/tokens/basics"
  "/tokens/extensions"
  "/references"
  "/references/account-types"
  "/references/account-constraints"
  "/references/anchor-toml"
  "/references/cli"
  "/references/avm"
  "/references/space"
  "/references/type-conversion"
  "/references/verifiable-builds"
  "/references/security-exploits"
  "/references/examples"
)

# Download each page
echo "Downloading specific Anchor Framework documentation pages..."
for page in "${PAGES[@]}"; do
  url="${BASE_URL}${page}"
  
  # If it's an empty string (main page), use index.html
  if [[ -z "$page" ]]; then
    output_file="${OUTPUT_DIR}/index.html"
  else
    # Remove leading slash and replace remaining slashes with hyphens
    sanitized_page=$(echo "$page" | sed 's/^\///' | sed 's/\//-/g')
    output_file="${OUTPUT_DIR}/${sanitized_page}.html"
  fi
  
  echo "Downloading $url to $output_file"
  wget --timeout=30 \
       --tries=3 \
       --user-agent="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36" \
       -O "$output_file" \
       "$url"
done

echo "Download complete! Specific Anchor Framework documentation pages are available in $OUTPUT_DIR"