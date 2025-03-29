#!/bin/bash
# Script to install Anchor documentation to the solana-hft-knowledge MCP server

# Define paths
MCP_SERVER_DIR="C:/Users/tonal/AppData/Roaming/Roo-Code/MCP/solana-hft-knowledge"
DOCS_DIR="$MCP_SERVER_DIR/docs/anchor"
SRC_DIR="$MCP_SERVER_DIR/src"
ANCHOR_DOCS_DIR="$(pwd)/simple-pages"

echo "Installing Anchor documentation to solana-hft-knowledge MCP server..."

# Create docs directory if it doesn't exist
echo "Creating documentation directory..."
mkdir -p "$DOCS_DIR"

# Copy documentation files
echo "Copying documentation files..."
cp -r "$ANCHOR_DOCS_DIR"/* "$DOCS_DIR/"

# Backup original files
echo "Backing up original files..."
cp "$SRC_DIR/knowledge.ts" "$SRC_DIR/knowledge.ts.bak"
cp "$SRC_DIR/index.ts" "$SRC_DIR/index.ts.bak"

# Apply changes to knowledge.ts
echo "Updating knowledge.ts..."
# Check if imports already exist
if ! grep -q "import fs from 'fs';" "$SRC_DIR/knowledge.ts"; then
  # Add imports at the top of the file
  sed -i '1i import fs from '\''fs'\'';\nimport path from '\''path'\'';\n' "$SRC_DIR/knowledge.ts"
fi

# Add getAnchorDocumentation function
cat << 'EOF' >> "$SRC_DIR/knowledge.ts"

/**
 * Get Anchor documentation content
 */
export function getAnchorDocumentation(docPath: string): string {
  // Base directory for Anchor documentation
  const baseDir = path.resolve(__dirname, '../docs/anchor');
  
  let filePath: string;
  
  if (!docPath || docPath === '') {
    // Main documentation page
    filePath = path.join(baseDir, 'index.html');
  } else if (docPath.includes('/')) {
    // Section with page
    filePath = path.join(baseDir, `${docPath.replace('/', '-')}.html`);
  } else {
    // Just a section
    filePath = path.join(baseDir, `${docPath}.html`);
  }
  
  try {
    if (fs.existsSync(filePath)) {
      return fs.readFileSync(filePath, 'utf8');
    } else {
      return `<html><body><h1>Documentation Not Found</h1><p>The requested Anchor documentation page "${docPath}" was not found.</p></body></html>`;
    }
  } catch (error) {
    console.error(`Error reading Anchor documentation: ${error}`);
    return `<html><body><h1>Error</h1><p>An error occurred while reading the Anchor documentation: ${error}</p></body></html>`;
  }
}
EOF

# Add anchor case to getRepositoryInfo function
sed -i '/switch (repo) {/a \    case '\''anchor'\'':\n      return {\n        name: '\''Anchor Framework'\'',\n        url: '\''https://github.com/coral-xyz/anchor'\'',\n        description: '\''Anchor is a framework for Solana's Sealevel runtime providing several convenient developer tools for writing smart contracts.'\'',\n        key_components: [\n          {\n            name: '\''lang'\'',\n            description: '\''Rust eDSL for writing Solana programs'\'',\n            potential_use: '\''Simplified development of Solana programs with built-in security features'\''\n          },\n          {\n            name: '\''client'\'',\n            description: '\''TypeScript and Rust client libraries'\'',\n            potential_use: '\''Strongly typed interfaces for interacting with Anchor programs'\''\n          },\n          {\n            name: '\''spl'\'',\n            description: '\''CPI clients for SPL programs'\'',\n            potential_use: '\''Easy integration with SPL tokens and other SPL programs'\''\n          },\n          {\n            name: '\''cli'\'',\n            description: '\''Command line interface for managing Anchor projects'\'',\n            potential_use: '\''Streamlined workflow for building, testing, and deploying Solana programs'\''\n          }\n        ],\n        status: '\''Active development'\''\n      };' "$SRC_DIR/knowledge.ts"

# Add Anchor search to searchKnowledgeBase function
sed -i '/export function searchKnowledgeBase/,/return results;/ s/return results;/  \/\/ Search Anchor documentation\n  if (query.toLowerCase().includes('\''anchor'\'')) {\n    results.push({\n      type: '\''documentation'\'',\n      title: '\''Anchor Framework Documentation'\'',\n      description: '\''Complete documentation for the Anchor Framework'\'',\n      url: '\''solana-hft:\/\/docs\/anchor'\'',\n      relevance: 0.9\n    });\n  }\n\n  return results;/' "$SRC_DIR/knowledge.ts"

# Update index.ts
echo "Updating index.ts..."

# Update import statement
sed -i "s/import { getRepositoryInfo, getComponentInfo, getRecommendations, searchKnowledgeBase } from '.\/knowledge.js';/import { getRepositoryInfo, getComponentInfo, getRecommendations, searchKnowledgeBase, getAnchorDocumentation } from '.\/knowledge.js';/" "$SRC_DIR/index.ts"

# Add resources to resources list
sed -i '/resources: \[/a \        {\n          uri: `solana-hft:\/\/repositories\/anchor`,\n          name: `Anchor Framework`,\n          mimeType: '\''application\/json'\'',\n          description: '\''Information about the Anchor Framework for Solana development'\'',\n        },\n        {\n          uri: `solana-hft:\/\/docs\/anchor`,\n          name: `Anchor Documentation`,\n          mimeType: '\''text\/html'\'',\n          description: '\''Complete documentation for the Anchor Framework'\'',\n        },' "$SRC_DIR/index.ts"

# Add resource templates
sed -i '/resourceTemplates: \[/a \          {\n            uriTemplate: '\''solana-hft:\/\/docs\/anchor\/{section}'\'',\n            name: '\''Anchor Documentation Section'\'',\n            mimeType: '\''text\/html'\'',\n            description: '\''Documentation section for the Anchor Framework'\'',\n          },\n          {\n            uriTemplate: '\''solana-hft:\/\/docs\/anchor\/{section}\/{page}'\'',\n            name: '\''Anchor Documentation Page'\'',\n            mimeType: '\''text\/html'\'',\n            description: '\''Specific documentation page for the Anchor Framework'\'',\n          },' "$SRC_DIR/index.ts"

# Add handler for Anchor documentation
sed -i '/ReadResourceRequestSchema/,/throw new McpError/ s/throw new McpError/        \/\/ Handle Anchor documentation\n        if (uri.startsWith('\''solana-hft:\/\/docs\/anchor\/'\'')) {\n          const path = uri.replace('\''solana-hft:\/\/docs\/anchor\/'\'', '\'''\'');\n          return {\n            contents: [\n              {\n                uri: request.params.uri,\n                mimeType: '\''text\/html'\'',\n                text: getAnchorDocumentation(path),\n              },\n            ],\n          };\n        }\n\n        \/\/ Handle Anchor main documentation page\n        if (uri === '\''solana-hft:\/\/docs\/anchor'\'') {\n          return {\n            contents: [\n              {\n                uri: request.params.uri,\n                mimeType: '\''text\/html'\'',\n                text: getAnchorDocumentation('\''\/'\''),\n              },\n            ],\n          };\n        }\n\n        throw new McpError/' "$SRC_DIR/index.ts"

# Add tool to tools list
sed -i '/tools: \[/a \        {\n          name: '\''get_anchor_documentation'\'',\n          description: '\''Get Anchor Framework documentation'\'',\n          inputSchema: {\n            type: '\''object'\'',\n            properties: {\n              path: {\n                type: '\''string'\'',\n                description: '\''Documentation path (e.g., "installation", "basics\/pda")'\'',\n              },\n            },\n            required: [],\n          },\n        },' "$SRC_DIR/index.ts"

# Add case to CallToolRequestSchema handler
sed -i '/CallToolRequestSchema/,/default:/ s/default:/        case '\''get_anchor_documentation'\'': {\n          const { path = '\'''\'' } = request.params.arguments as { path?: string };\n          return {\n            content: [\n              {\n                type: '\''text'\'',\n                text: getAnchorDocumentation(path),\n              },\n            ],\n          };\n        }\n        \n        default:/' "$SRC_DIR/index.ts"

# Rebuild the server
echo "Rebuilding the server..."
cd "$MCP_SERVER_DIR"
npm run build

echo "Installation complete!"
echo "You can now access the Anchor documentation through the MCP server."
echo "- Repository information: solana-hft://repositories/anchor"
echo "- Main documentation page: solana-hft://docs/anchor"
echo "- Specific documentation sections: solana-hft://docs/anchor/installation"
echo "- Specific documentation pages: solana-hft://docs/anchor/basics/pda"