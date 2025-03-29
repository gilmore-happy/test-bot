# Anchor Documentation for Solana HFT Knowledge

This project contains the Anchor Framework documentation and tools to integrate it with the solana-hft-knowledge MCP server.

## Directory Structure

```
anchor-documentation/
├── docs/                      # The actual documentation files
│   └── simple-pages/          # HTML documentation files
│
├── scripts/                   # Scripts for downloading and processing
│   ├── docs_anchor_script.sh  # Original script
│   ├── download_specific_pages.sh
│   ├── simple_download.sh
│   ├── install_to_mcp.sh      # MCP installation script (bash)
│   └── install_to_mcp.bat     # MCP installation script (Windows)
│
├── mcp-integration/           # MCP server integration files
│   ├── knowledge.ts.update
│   ├── index.ts.update
│   └── anchor_knowledge_update.ts
│
├── scraper/                   # Configurable documentation scraper
│   ├── configurable-scraper.js
│   ├── documentation-server.js
│   └── package.json
│
└── README.md                  # This file
```

## Documentation

The `docs/simple-pages/` directory contains the HTML documentation files for the Anchor Framework. These files were downloaded from the official Anchor documentation website.

A navigation page is available at `docs/simple-pages/index-nav.html` to browse the documentation.

## Integration with solana-hft-knowledge MCP Server

The `mcp-integration/` directory contains the files needed to integrate the Anchor documentation with the solana-hft-knowledge MCP server.

### Installation

To add the Anchor documentation to your solana-hft-knowledge MCP server:

#### Automated Installation

1. For Git Bash:
   ```bash
   ./scripts/install_to_mcp.sh
   ```

2. For Windows Command Prompt:
   ```cmd
   scripts\install_to_mcp.bat
   ```

#### Manual Installation

Follow the instructions in the README.md file in the root directory.

## Documentation Scraper

The `scraper/` directory contains a configurable documentation scraper that can be used to download documentation from any website.

### Usage

1. Install dependencies:
   ```bash
   cd scraper
   npm install
   ```

2. Configure the scraper by editing the `CONFIG` object in `configurable-scraper.js`.

3. Run the scraper:
   ```bash
   node configurable-scraper.js
   ```

## Scripts

The `scripts/` directory contains various scripts for downloading and processing the Anchor documentation:

- `docs_anchor_script.sh`: The original script used to download the documentation
- `download_specific_pages.sh`: Script to download specific pages from the Anchor documentation
- `simple_download.sh`: Simplified script to download the documentation
- `install_to_mcp.sh`: Script to install the documentation to the solana-hft-knowledge MCP server (Bash)
- `install_to_mcp.bat`: Script to install the documentation to the solana-hft-knowledge MCP server (Windows)

## Organization

This directory structure was created using the organization scripts:

- `organize_files.sh`: Bash script to organize the files
- `organize_files.bat`: Windows batch script to organize the files