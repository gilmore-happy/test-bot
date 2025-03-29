#!/bin/bash

# Set up variables
DOCS_URL="https://www.anchor-lang.com/docs/"
OUTPUT_DIR="C:/Users/tonal/Documents/1HFT Solana Bot/Additional Documentation/anchor_docs/improved"

# Create directory if it doesn't exist (Note: Using forward slashes for Windows bash)
mkdir -p "$OUTPUT_DIR"

# Download the documentation with improved options for JavaScript-based websites
echo "Downloading Anchor Framework documentation..."
wget --mirror \
     --recursive \
     --level=inf \
     --convert-links \
     --adjust-extension \
     --page-requisites \
     --span-hosts \
     --domains=anchor-lang.com \
     --include-directories=/docs/ \
     --no-parent \
     --wait=1 \
     --random-wait \
     --execute robots=off \
     --reject "index.html?*" \
     --directory-prefix="$OUTPUT_DIR" \
     --no-host-directories \
     --timeout=60 \
     --tries=5 \
     --user-agent="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36" \
     "$DOCS_URL"

# Clean up the file structure
echo "Organizing files..."
find "$OUTPUT_DIR" -type d -name "*\?*" -exec rm -rf {} + 2>/dev/null || true

echo "Download complete! Improved Anchor Framework documentation is available in $OUTPUT_DIR"