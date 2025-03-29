#!/bin/bash
# Script to organize Anchor documentation files

echo "Organizing Anchor documentation files..."

# Define paths
BASE_DIR="Additional Documentation"
SOURCE_ANCHOR_DIR="$BASE_DIR/anchor_docs"
SOURCE_SCRAPER_DIR="$BASE_DIR/docs-scraper"
TARGET_DIR="$BASE_DIR/anchor-documentation"

# Create the directory structure
echo "Creating directory structure..."
mkdir -p "$TARGET_DIR/docs/simple-pages"
mkdir -p "$TARGET_DIR/scripts"
mkdir -p "$TARGET_DIR/mcp-integration"
mkdir -p "$TARGET_DIR/scraper"

# Move documentation files
echo "Moving documentation files..."
if [ -d "$SOURCE_ANCHOR_DIR/simple-pages" ]; then
  cp -r "$SOURCE_ANCHOR_DIR/simple-pages/"* "$TARGET_DIR/docs/simple-pages/"
  echo "  - Copied simple-pages to docs/simple-pages"
fi

# Move scripts
echo "Moving scripts..."
if [ -f "$SOURCE_ANCHOR_DIR/docs_anchor_script.sh" ]; then
  cp "$SOURCE_ANCHOR_DIR/docs_anchor_script.sh" "$TARGET_DIR/scripts/"
  echo "  - Copied docs_anchor_script.sh"
fi

if [ -f "$SOURCE_ANCHOR_DIR/download_specific_pages.sh" ]; then
  cp "$SOURCE_ANCHOR_DIR/download_specific_pages.sh" "$TARGET_DIR/scripts/"
  echo "  - Copied download_specific_pages.sh"
fi

if [ -f "$SOURCE_ANCHOR_DIR/simple_download.sh" ]; then
  cp "$SOURCE_ANCHOR_DIR/simple_download.sh" "$TARGET_DIR/scripts/"
  echo "  - Copied simple_download.sh"
fi

if [ -f "$SOURCE_ANCHOR_DIR/install_to_mcp.sh" ]; then
  cp "$SOURCE_ANCHOR_DIR/install_to_mcp.sh" "$TARGET_DIR/scripts/"
  echo "  - Copied install_to_mcp.sh"
fi

if [ -f "$SOURCE_ANCHOR_DIR/install_to_mcp.bat" ]; then
  cp "$SOURCE_ANCHOR_DIR/install_to_mcp.bat" "$TARGET_DIR/scripts/"
  echo "  - Copied install_to_mcp.bat"
fi

# Move MCP integration files
echo "Moving MCP integration files..."
if [ -f "$SOURCE_ANCHOR_DIR/knowledge.ts.update" ]; then
  cp "$SOURCE_ANCHOR_DIR/knowledge.ts.update" "$TARGET_DIR/mcp-integration/"
  echo "  - Copied knowledge.ts.update"
fi

if [ -f "$SOURCE_ANCHOR_DIR/index.ts.update" ]; then
  cp "$SOURCE_ANCHOR_DIR/index.ts.update" "$TARGET_DIR/mcp-integration/"
  echo "  - Copied index.ts.update"
fi

if [ -f "$SOURCE_ANCHOR_DIR/anchor_knowledge_update.ts" ]; then
  cp "$SOURCE_ANCHOR_DIR/anchor_knowledge_update.ts" "$TARGET_DIR/mcp-integration/"
  echo "  - Copied anchor_knowledge_update.ts"
fi

# Move scraper files
echo "Moving scraper files..."
if [ -f "$SOURCE_SCRAPER_DIR/configurable-scraper.js" ]; then
  cp "$SOURCE_SCRAPER_DIR/configurable-scraper.js" "$TARGET_DIR/scraper/"
  echo "  - Copied configurable-scraper.js"
fi

if [ -f "$SOURCE_SCRAPER_DIR/documentation-server.js" ]; then
  cp "$SOURCE_SCRAPER_DIR/documentation-server.js" "$TARGET_DIR/scraper/"
  echo "  - Copied documentation-server.js"
fi

if [ -f "$SOURCE_SCRAPER_DIR/package.json" ]; then
  cp "$SOURCE_SCRAPER_DIR/package.json" "$TARGET_DIR/scraper/"
  echo "  - Copied package.json"
fi

if [ -f "$SOURCE_SCRAPER_DIR/README.md" ]; then
  cp "$SOURCE_SCRAPER_DIR/README.md" "$TARGET_DIR/scraper/README.md"
  echo "  - Copied scraper README.md"
fi

# Copy README
echo "Copying README..."
if [ -f "$SOURCE_ANCHOR_DIR/main_README.md" ]; then
  cp "$SOURCE_ANCHOR_DIR/main_README.md" "$TARGET_DIR/README.md"
  echo "  - Copied main_README.md to README.md"
elif [ -f "$SOURCE_ANCHOR_DIR/README.md" ]; then
  cp "$SOURCE_ANCHOR_DIR/README.md" "$TARGET_DIR/"
  echo "  - Copied main README.md"
fi

# Copy organization plan
if [ -f "$SOURCE_ANCHOR_DIR/organization_plan.md" ]; then
  cp "$SOURCE_ANCHOR_DIR/organization_plan.md" "$TARGET_DIR/"
  echo "  - Copied organization_plan.md"
fi

echo "Organization complete!"
echo "All files have been organized in: $TARGET_DIR"
echo ""
echo "Note: The original files have not been deleted. After verifying the"
echo "organized files, you may want to delete the original directories:"
echo "  - $SOURCE_ANCHOR_DIR"
echo "  - $SOURCE_SCRAPER_DIR"