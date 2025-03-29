# Anchor Documentation for Solana HFT Knowledge MCP Server

This directory contains the Anchor Framework documentation and the necessary files to add it to the solana-hft-knowledge MCP server.

## Directory Structure

- `simple-pages/`: Contains the downloaded Anchor documentation HTML files
- `knowledge.ts.update`: Contains the updates to add to the knowledge.ts file
- `index.ts.update`: Contains the updates to add to the index.ts file

## How to Add Anchor Documentation to the MCP Server

Follow these steps to add the Anchor documentation to the solana-hft-knowledge MCP server:

### 1. Copy the Documentation Files

First, make sure the documentation files are accessible to the MCP server:

```bash
# Create a directory for the documentation in a location accessible to the MCP server
mkdir -p "C:/Users/tonal/AppData/Roaming/Roo-Code/MCP/solana-hft-knowledge/docs/anchor"

# Copy the documentation files
cp -r "Additional Documentation/anchor_docs/simple-pages/"* "C:/Users/tonal/AppData/Roaming/Roo-Code/MCP/solana-hft-knowledge/docs/anchor/"
```

### 2. Update the MCP Server Files

#### Update knowledge.ts

1. Open the knowledge.ts file:
   ```bash
   code "C:/Users/tonal/AppData/Roaming/Roo-Code/MCP/solana-hft-knowledge/src/knowledge.ts"
   ```

2. Add the imports at the top of the file:
   ```typescript
   import fs from 'fs';
   import path from 'path';
   ```

3. Add the getAnchorDocumentation function:
   ```typescript
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
   ```

4. Add the 'anchor' case to the getRepositoryInfo function:
   ```typescript
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
   ```

5. Add the Anchor search to the searchKnowledgeBase function:
   ```typescript
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
   ```

#### Update index.ts

1. Open the index.ts file:
   ```bash
   code "C:/Users/tonal/AppData/Roaming/Roo-Code/MCP/solana-hft-knowledge/src/index.ts"
   ```

2. Update the import statement to include getAnchorDocumentation:
   ```typescript
   import { 
     getRepositoryInfo, 
     getComponentInfo, 
     getRecommendations, 
     searchKnowledgeBase,
     getAnchorDocumentation
   } from './knowledge.js';
   ```

3. Add these resources to the resources list in setupResourceHandlers():
   ```typescript
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
   ```

4. Add these resource templates to the resourceTemplates list:
   ```typescript
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
   ```

5. Add this code to the ReadResourceRequestSchema handler before the final throw statement:
   ```typescript
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
   ```

6. Add this tool to the tools list in setupToolHandlers():
   ```typescript
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
   ```

7. Add this case to the CallToolRequestSchema handler switch statement:
   ```typescript
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
   ```

### 3. Rebuild the Server

After making the changes, rebuild the server:

```bash
cd "C:/Users/tonal/AppData/Roaming/Roo-Code/MCP/solana-hft-knowledge"
npm run build
```

### 4. Test the Integration

You can now access the Anchor documentation through the MCP server:

- Repository information: `solana-hft://repositories/anchor`
- Main documentation page: `solana-hft://docs/anchor`
- Specific documentation sections: `solana-hft://docs/anchor/installation`
- Specific documentation pages: `solana-hft://docs/anchor/basics/pda`

You can also use the `get_anchor_documentation` tool to retrieve documentation content.

## Troubleshooting

If you encounter any issues:

1. Check that the documentation files are correctly copied to the MCP server's docs directory
2. Verify that the file paths in the getAnchorDocumentation function are correct
3. Make sure the server was rebuilt after making the changes
4. Check the server logs for any error messages