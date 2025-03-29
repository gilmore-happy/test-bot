# Documentation Scraper

This tool allows you to download documentation from any website and organize it in a structured format. It can be configured to download specific pages and create a navigation index.

## Setup

1. Make sure you have Node.js installed
2. Install dependencies:
   ```
   cd "Additional Documentation/docs-scraper"
   npm install
   ```

## Usage

### Configuration

Edit the `CONFIG` object in `configurable-scraper.js` to customize:

- `baseUrl`: The base URL of the documentation website
- `outputDir`: Where to save the documentation (relative to project root)
- `sections`: The structure of the documentation (sections and pages)
- `browser`: Puppeteer browser settings
- `navPage`: Settings for the navigation page

Example configuration for Anchor documentation:

```javascript
const CONFIG = {
  baseUrl: "https://www.anchor-lang.com/docs",
  outputDir: "Additional Documentation/docs/anchor",
  sections: [
    {
      title: "Getting Started",
      pages: [
        { title: "Introduction", path: "" },
        { title: "Installation", path: "/installation" },
        // ...more pages
      ]
    },
    // ...more sections
  ],
  // ...other settings
};
```

### Running the Scraper

```
cd "Additional Documentation/docs-scraper"
npm run scrape
```

This will:
1. Create the directory structure based on your configuration
2. Download all specified pages
3. Create a navigation page for easy browsing

## Adding Documentation to MCP Server

To add the downloaded documentation to your MCP server:

1. Create a new MCP server or modify the existing `solana-docs-server`:

```javascript
// In your MCP server file
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import fs from 'fs';
import path from 'path';

// Define the documentation directory
const DOCS_DIR = path.resolve('Additional Documentation/docs');

// Create resources for each documentation set
const resources = [];
const resourceTemplates = [];

// Add Anchor documentation resources
if (fs.existsSync(path.join(DOCS_DIR, 'anchor'))) {
  // Add direct resource for the main index
  resources.push({
    uri: 'anchor://docs',
    name: 'Anchor Documentation',
    description: 'Anchor Framework Documentation',
    mimeType: 'text/html'
  });
  
  // Add resource template for specific pages
  resourceTemplates.push({
    uriTemplate: 'anchor://docs/{section}/{page}',
    name: 'Anchor Documentation Page',
    description: 'Specific page from Anchor documentation',
    mimeType: 'text/html'
  });
}

// Set up the server
const server = new Server(
  {
    name: 'documentation-server',
    version: '1.0.0',
  },
  {
    capabilities: {
      resources: {},
      tools: {},
    },
  }
);

// Handle resource requests
server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
  const uri = request.params.uri;
  
  // Handle Anchor documentation
  if (uri.startsWith('anchor://docs')) {
    if (uri === 'anchor://docs') {
      // Return the main index
      const content = fs.readFileSync(path.join(DOCS_DIR, 'anchor', 'index.html'), 'utf8');
      return {
        contents: [
          {
            uri: uri,
            mimeType: 'text/html',
            text: content,
          },
        ],
      };
    } else {
      // Parse the URI to get section and page
      const match = uri.match(/^anchor:\/\/docs\/([^\/]+)\/([^\/]+)$/);
      if (match) {
        const [_, section, page] = match;
        const filePath = path.join(DOCS_DIR, 'anchor', section, page + '.html');
        
        if (fs.existsSync(filePath)) {
          const content = fs.readFileSync(filePath, 'utf8');
          return {
            contents: [
              {
                uri: uri,
                mimeType: 'text/html',
                text: content,
              },
            ],
          };
        }
      }
    }
  }
  
  throw new McpError(ErrorCode.NotFound, `Resource not found: ${uri}`);
});

// Start the server
const transport = new StdioServerTransport();
server.connect(transport).catch(console.error);
```

2. Add the server to your MCP settings file:

```json
{
  "mcpServers": {
    "documentation-server": {
      "command": "node",
      "args": ["/path/to/documentation-server.js"],
      "env": {}
    }
  }
}
```

3. Restart your application to load the new MCP server

## Recommended Documentation Structure

For best organization, use this structure:

```
Additional Documentation/
├── docs/
│   ├── anchor/           # Anchor documentation
│   │   ├── index.html    # Main navigation page
│   │   ├── getting-started/
│   │   │   ├── index.html
│   │   │   └── ...
│   │   ├── core-concepts/
│   │   │   ├── index.html
│   │   │   └── ...
│   │   └── ...
│   ├── solana/           # Solana documentation (if added)
│   │   └── ...
│   └── ...
└── docs-scraper/         # The scraper tool
    ├── configurable-scraper.js
    ├── package.json
    └── README.md
```

This structure keeps all documentation organized and makes it easy to add new documentation sets in the future.