/**
 * Documentation MCP Server
 * 
 * This MCP server provides access to locally stored documentation
 * through the Model Context Protocol.
 */

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  ErrorCode,
  ListResourcesRequestSchema,
  ListResourceTemplatesRequestSchema,
  McpError,
  ReadResourceRequestSchema,
} from '@modelcontextprotocol/sdk/types.js';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

// Get the directory name
const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Define the documentation directory (relative to this script)
const DOCS_DIR = path.resolve(__dirname, '../docs');

// Documentation sets configuration
const DOC_SETS = [
  {
    id: 'anchor',
    name: 'Anchor Framework',
    description: 'Documentation for the Anchor Framework for Solana development',
    dir: 'anchor'
  }
  // Add more documentation sets here as needed
];

class DocumentationServer {
  constructor() {
    this.server = new Server(
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

    this.setupResourceHandlers();
    
    // Error handling
    this.server.onerror = (error) => console.error('[MCP Error]', error);
    process.on('SIGINT', async () => {
      await this.server.close();
      process.exit(0);
    });
  }

  setupResourceHandlers() {
    // List available documentation resources
    this.server.setRequestHandler(ListResourcesRequestSchema, async () => {
      const resources = [];
      
      // Add main resources for each documentation set
      for (const docSet of DOC_SETS) {
        const docDir = path.join(DOCS_DIR, docSet.dir);
        
        if (fs.existsSync(docDir)) {
          resources.push({
            uri: `${docSet.id}://docs`,
            name: `${docSet.name} Documentation`,
            description: docSet.description,
            mimeType: 'text/html'
          });
        }
      }
      
      return { resources };
    });

    // List resource templates
    this.server.setRequestHandler(ListResourceTemplatesRequestSchema, async () => {
      const resourceTemplates = [];
      
      // Add templates for each documentation set
      for (const docSet of DOC_SETS) {
        const docDir = path.join(DOCS_DIR, docSet.dir);
        
        if (fs.existsSync(docDir)) {
          resourceTemplates.push({
            uriTemplate: `${docSet.id}://docs/{section}`,
            name: `${docSet.name} Documentation Section`,
            description: `Section from ${docSet.name} documentation`,
            mimeType: 'text/html'
          });
          
          resourceTemplates.push({
            uriTemplate: `${docSet.id}://docs/{section}/{page}`,
            name: `${docSet.name} Documentation Page`,
            description: `Specific page from ${docSet.name} documentation`,
            mimeType: 'text/html'
          });
        }
      }
      
      return { resourceTemplates };
    });

    // Handle resource requests
    this.server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
      const uri = request.params.uri;
      
      // Process each documentation set
      for (const docSet of DOC_SETS) {
        const prefix = `${docSet.id}://docs`;
        
        if (uri.startsWith(prefix)) {
          const docDir = path.join(DOCS_DIR, docSet.dir);
          
          // Main documentation index
          if (uri === prefix) {
            const indexPath = path.join(docDir, 'index.html');
            if (fs.existsSync(indexPath)) {
              const content = fs.readFileSync(indexPath, 'utf8');
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
          // Section index
          else if (uri.match(new RegExp(`^${prefix}/[^/]+$`))) {
            const section = uri.substring(prefix.length + 1);
            const sectionPath = path.join(docDir, section, 'index.html');
            
            if (fs.existsSync(sectionPath)) {
              const content = fs.readFileSync(sectionPath, 'utf8');
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
          // Specific page
          else if (uri.match(new RegExp(`^${prefix}/[^/]+/[^/]+$`))) {
            const parts = uri.substring(prefix.length + 1).split('/');
            const section = parts[0];
            const page = parts[1];
            
            let pagePath = path.join(docDir, section, `${page}.html`);
            
            // If the specific page doesn't exist, try index.html
            if (!fs.existsSync(pagePath) && page === 'index') {
              pagePath = path.join(docDir, section, 'index.html');
            }
            
            if (fs.existsSync(pagePath)) {
              const content = fs.readFileSync(pagePath, 'utf8');
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
  }

  async run() {
    // Check if documentation directory exists
    if (!fs.existsSync(DOCS_DIR)) {
      console.error(`Documentation directory not found: ${DOCS_DIR}`);
      console.error('Please run the scraper first to download documentation.');
      process.exit(1);
    }
    
    // Check each documentation set
    for (const docSet of DOC_SETS) {
      const docDir = path.join(DOCS_DIR, docSet.dir);
      if (!fs.existsSync(docDir)) {
        console.warn(`Documentation set not found: ${docSet.id} (${docDir})`);
        console.warn(`This documentation set will not be available.`);
      } else {
        console.log(`Found documentation set: ${docSet.id}`);
      }
    }
    
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
    console.error('Documentation MCP server running on stdio');
  }
}

const server = new DocumentationServer();
server.run().catch(console.error);