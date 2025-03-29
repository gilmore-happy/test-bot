#!/bin/bash

# Set up variables
DOCS_URL="https://www.anchor-lang.com/docs/"
OUTPUT_DIR="C:/Users/tonal/Documents/1HFT Solana Bot/Additional Documentation/anchor_docs"

# Create directory if it doesn't exist (Note: Using forward slashes for Windows bash)
mkdir -p "$OUTPUT_DIR"

# Download the documentation
echo "Downloading Anchor Framework documentation..."
wget --mirror \
     --convert-links \
     --adjust-extension \
     --page-requisites \
     --no-parent \
     --reject "index.html?*" \
     --directory-prefix="$OUTPUT_DIR" \
     --no-host-directories \
     --timeout=30 \
     --tries=3 \
     --user-agent="Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36" \
     "$DOCS_URL"

# Clean up the file structure
echo "Organizing files..."
find "$OUTPUT_DIR" -type d -name "*\?*" -exec rm -rf {} + 2>/dev/null || true

echo "Download complete! Anchor Framework documentation is available in $OUTPUT_DIR"