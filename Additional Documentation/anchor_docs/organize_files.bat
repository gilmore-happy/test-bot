@echo off
REM Script to organize Anchor documentation files

echo Organizing Anchor documentation files...

REM Define paths
set BASE_DIR=Additional Documentation
set SOURCE_ANCHOR_DIR=%BASE_DIR%\anchor_docs
set SOURCE_SCRAPER_DIR=%BASE_DIR%\docs-scraper
set TARGET_DIR=%BASE_DIR%\anchor-documentation

REM Create the directory structure
echo Creating directory structure...
if not exist "%TARGET_DIR%\docs\simple-pages" mkdir "%TARGET_DIR%\docs\simple-pages"
if not exist "%TARGET_DIR%\scripts" mkdir "%TARGET_DIR%\scripts"
if not exist "%TARGET_DIR%\mcp-integration" mkdir "%TARGET_DIR%\mcp-integration"
if not exist "%TARGET_DIR%\scraper" mkdir "%TARGET_DIR%\scraper"

REM Move documentation files
echo Moving documentation files...
if exist "%SOURCE_ANCHOR_DIR%\simple-pages" (
  xcopy /E /I /Y "%SOURCE_ANCHOR_DIR%\simple-pages\*" "%TARGET_DIR%\docs\simple-pages\"
  echo   - Copied simple-pages to docs\simple-pages
)

REM Move scripts
echo Moving scripts...
if exist "%SOURCE_ANCHOR_DIR%\docs_anchor_script.sh" (
  copy "%SOURCE_ANCHOR_DIR%\docs_anchor_script.sh" "%TARGET_DIR%\scripts\"
  echo   - Copied docs_anchor_script.sh
)

if exist "%SOURCE_ANCHOR_DIR%\download_specific_pages.sh" (
  copy "%SOURCE_ANCHOR_DIR%\download_specific_pages.sh" "%TARGET_DIR%\scripts\"
  echo   - Copied download_specific_pages.sh
)

if exist "%SOURCE_ANCHOR_DIR%\simple_download.sh" (
  copy "%SOURCE_ANCHOR_DIR%\simple_download.sh" "%TARGET_DIR%\scripts\"
  echo   - Copied simple_download.sh
)

if exist "%SOURCE_ANCHOR_DIR%\install_to_mcp.sh" (
  copy "%SOURCE_ANCHOR_DIR%\install_to_mcp.sh" "%TARGET_DIR%\scripts\"
  echo   - Copied install_to_mcp.sh
)

if exist "%SOURCE_ANCHOR_DIR%\install_to_mcp.bat" (
  copy "%SOURCE_ANCHOR_DIR%\install_to_mcp.bat" "%TARGET_DIR%\scripts\"
  echo   - Copied install_to_mcp.bat
)

REM Move MCP integration files
echo Moving MCP integration files...
if exist "%SOURCE_ANCHOR_DIR%\knowledge.ts.update" (
  copy "%SOURCE_ANCHOR_DIR%\knowledge.ts.update" "%TARGET_DIR%\mcp-integration\"
  echo   - Copied knowledge.ts.update
)

if exist "%SOURCE_ANCHOR_DIR%\index.ts.update" (
  copy "%SOURCE_ANCHOR_DIR%\index.ts.update" "%TARGET_DIR%\mcp-integration\"
  echo   - Copied index.ts.update
)

if exist "%SOURCE_ANCHOR_DIR%\anchor_knowledge_update.ts" (
  copy "%SOURCE_ANCHOR_DIR%\anchor_knowledge_update.ts" "%TARGET_DIR%\mcp-integration\"
  echo   - Copied anchor_knowledge_update.ts
)

REM Move scraper files
echo Moving scraper files...
if exist "%SOURCE_SCRAPER_DIR%\configurable-scraper.js" (
  copy "%SOURCE_SCRAPER_DIR%\configurable-scraper.js" "%TARGET_DIR%\scraper\"
  echo   - Copied configurable-scraper.js
)

if exist "%SOURCE_SCRAPER_DIR%\documentation-server.js" (
  copy "%SOURCE_SCRAPER_DIR%\documentation-server.js" "%TARGET_DIR%\scraper\"
  echo   - Copied documentation-server.js
)

if exist "%SOURCE_SCRAPER_DIR%\package.json" (
  copy "%SOURCE_SCRAPER_DIR%\package.json" "%TARGET_DIR%\scraper\"
  echo   - Copied package.json
)

if exist "%SOURCE_SCRAPER_DIR%\README.md" (
  copy "%SOURCE_SCRAPER_DIR%\README.md" "%TARGET_DIR%\scraper\README.md"
  echo   - Copied scraper README.md
)

REM Copy README
echo Copying README...
if exist "%SOURCE_ANCHOR_DIR%\main_README.md" (
  copy "%SOURCE_ANCHOR_DIR%\main_README.md" "%TARGET_DIR%\README.md"
  echo   - Copied main_README.md to README.md
) else if exist "%SOURCE_ANCHOR_DIR%\README.md" (
  copy "%SOURCE_ANCHOR_DIR%\README.md" "%TARGET_DIR%\"
  echo   - Copied main README.md
)

REM Copy organization plan
if exist "%SOURCE_ANCHOR_DIR%\organization_plan.md" (
  copy "%SOURCE_ANCHOR_DIR%\organization_plan.md" "%TARGET_DIR%\"
  echo   - Copied organization_plan.md
)

echo Organization complete!
echo All files have been organized in: %TARGET_DIR%
echo.
echo Note: The original files have not been deleted. After verifying the
echo organized files, you may want to delete the original directories:
echo   - %SOURCE_ANCHOR_DIR%
echo   - %SOURCE_SCRAPER_DIR%