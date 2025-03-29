/**
 * Anchor Documentation Knowledge Update
 * 
 * This file contains the changes needed to add Anchor documentation
 * to the solana-hft-knowledge MCP server.
 */

// ===== CHANGES TO index.ts =====

// Add to the resources list in setupResourceHandlers()
/*
{
  uri: `solana-hft://repositories/anchor`,
  name: `Anchor Framework`,
  mimeType: 'application/json',
  description: 'Information about the Anchor Framework for Solana development',
},
{
  uri: `solana-hft://docs/anchor`,
  name: `Anchor Documentation`,
  mimeType: 'text/html',
  description: 'Complete documentation for the Anchor Framework',
},
*/

// Add to the resourceTemplates list in setupResourceHandlers()
/*
{
  uriTemplate: 'solana-hft://docs/anchor/{section}',
  name: 'Anchor Documentation Section',
  mimeType: 'text/html',
  description: 'Documentation section for the Anchor Framework',
},
{
  uriTemplate: 'solana-hft://docs/anchor/{section}/{page}',
  name: 'Anchor Documentation Page',
  mimeType: 'text/html',
  description: 'Specific documentation page for the Anchor Framework',
},
*/

// Add to the ReadResourceRequestSchema handler
/*
// Handle Anchor documentation
if (uri.startsWith('solana-hft://docs/anchor/')) {
  const path = uri.replace('solana-hft://docs/anchor/', '');
  return {
    contents: [
      {
        uri: request.params.uri,
        mimeType: 'text/html',
        text: getAnchorDocumentation(path),
      },
    ],
  };
}

// Handle Anchor main documentation page
if (uri === 'solana-hft://docs/anchor') {
  return {
    contents: [
      {
        uri: request.params.uri,
        mimeType: 'text/html',
        text: getAnchorDocumentation(''),
      },
    ],
  };
}
*/

// Add to the tools list in setupToolHandlers()
/*
{
  name: 'get_anchor_documentation',
  description: 'Get Anchor Framework documentation',
  inputSchema: {
    type: 'object',
    properties: {
      path: {
        type: 'string',
        description: 'Documentation path (e.g., "installation", "basics/pda")',
      },
    },
    required: [],
  },
},
*/

// Add to the CallToolRequestSchema handler
/*
case 'get_anchor_documentation': {
  const { path = '' } = request.params.arguments as { path?: string };
  return {
    content: [
      {
        type: 'text',
        text: getAnchorDocumentation(path),
      },
    ],
  };
}
*/

// ===== CHANGES TO knowledge.ts =====

// Add import for fs and path
/*
import fs from 'fs';
import path from 'path';
*/

// Add the getAnchorDocumentation function
/*
/**
 * Get Anchor documentation content
 */
export function getAnchorDocumentation(docPath: string): string {
  // Base directory for Anchor documentation
  const baseDir = path.resolve(__dirname, '../../Additional Documentation/anchor_docs/simple-pages');
  
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
*/

// Add to the getRepositoryInfo function
/*
case 'anchor':
  return {
    name: 'Anchor Framework',
    url: 'https://github.com/coral-xyz/anchor',
    description: 'Anchor is a framework for Solana's Sealevel runtime providing several convenient developer tools for writing smart contracts.',
    key_components: [
      {
        name: 'lang',
        description: 'Rust eDSL for writing Solana programs',
        potential_use: 'Simplified development of Solana programs with built-in security features'
      },
      {
        name: 'client',
        description: 'TypeScript and Rust client libraries',
        potential_use: 'Strongly typed interfaces for interacting with Anchor programs'
      },
      {
        name: 'spl',
        description: 'CPI clients for SPL programs',
        potential_use: 'Easy integration with SPL tokens and other SPL programs'
      },
      {
        name: 'cli',
        description: 'Command line interface for managing Anchor projects',
        potential_use: 'Streamlined workflow for building, testing, and deploying Solana programs'
      }
    ],
    status: 'Active development'
  };
*/

// Add to the searchKnowledgeBase function
/*
// Search Anchor documentation
if (query.toLowerCase().includes('anchor')) {
  results.push({
    type: 'documentation',
    title: 'Anchor Framework Documentation',
    description: 'Complete documentation for the Anchor Framework',
    url: 'solana-hft://docs/anchor',
    relevance: 0.9
  });
}
*/

// ===== IMPLEMENTATION STEPS =====

// 1. Copy the downloaded Anchor documentation to the appropriate location
//    - The documentation is currently in "Additional Documentation/anchor_docs/simple-pages"
//    - Make sure this location is accessible to the MCP server

// 2. Update the solana-hft-knowledge MCP server files:
//    - Add the changes to index.ts
//    - Add the changes to knowledge.ts

// 3. Rebuild the server:
//    cd C:\Users\tonal\AppData\Roaming\Roo-Code\MCP\solana-hft-knowledge
//    npm run build

// 4. The Anchor documentation will now be available through the MCP server