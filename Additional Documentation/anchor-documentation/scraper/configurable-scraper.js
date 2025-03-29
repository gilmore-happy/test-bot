/**
 * Configurable Documentation Scraper
 * 
 * This script can download documentation from any website and organize it
 * in a structured format. It can be configured to download specific pages
 * and create a navigation index.
 */

const puppeteer = require('puppeteer');
const fs = require('fs-extra');
const path = require('path');
const url = require('url');

// Configuration (can be modified for any website)
const CONFIG = {
  // Base URL of the documentation
  baseUrl: "https://www.anchor-lang.com/docs",
  
  // Output directory relative to the project root
  outputDir: "Additional Documentation/docs/anchor",
  
  // Navigation structure (sections and pages)
  sections: [
    {
      title: "Getting Started",
      pages: [
        { title: "Introduction", path: "" },
        { title: "Installation", path: "/installation" },
        { title: "Quickstart", path: "/quickstart" },
        { title: "Quickstart: Solana Playground", path: "/quickstart/solpg" },
        { title: "Quickstart: Local Development", path: "/quickstart/local" }
      ]
    },
    {
      title: "Core Concepts",
      pages: [
        { title: "The Basics", path: "/basics" },
        { title: "Program Structure", path: "/basics/program-structure" },
        { title: "Program IDL File", path: "/basics/idl" },
        { title: "Program Derived Address", path: "/basics/pda" },
        { title: "Cross Program Invocation", path: "/basics/cpi" }
      ]
    },
    {
      title: "Client Libraries",
      pages: [
        { title: "Client Libraries", path: "/clients" },
        { title: "TypeScript", path: "/clients/typescript" },
        { title: "Rust", path: "/clients/rust" }
      ]
    },
    {
      title: "Testing Libraries",
      pages: [
        { title: "Testing Libraries", path: "/testing" },
        { title: "LiteSVM", path: "/testing/litesvm" },
        { title: "Mollusk", path: "/testing/mollusk" }
      ]
    },
    {
      title: "Additional Features",
      pages: [
        { title: "Additional Features", path: "/features" },
        { title: "Dependency Free Composability", path: "/features/declare-program" },
        { title: "Custom Errors", path: "/features/errors" },
        { title: "Emit Events", path: "/features/events" },
        { title: "Zero Copy", path: "/features/zero-copy" }
      ]
    },
    {
      title: "SPL Tokens",
      pages: [
        { title: "Interacting with Tokens", path: "/tokens" },
        { title: "Basics", path: "/tokens/basics" },
        { title: "Extensions", path: "/tokens/extensions" }
      ]
    },
    {
      title: "References",
      pages: [
        { title: "Program Development", path: "/references" },
        { title: "Account Types", path: "/references/account-types" },
        { title: "Account Constraints", path: "/references/account-constraints" },
        { title: "Anchor.toml Configuration", path: "/references/anchor-toml" },
        { title: "Anchor CLI", path: "/references/cli" },
        { title: "Anchor Version Manager", path: "/references/avm" },
        { title: "Account Space", path: "/references/space" },
        { title: "Rust to JS Type Conversion", path: "/references/type-conversion" },
        { title: "Verifiable Builds", path: "/references/verifiable-builds" },
        { title: "Sealevel Attacks", path: "/references/security-exploits" },
        { title: "Example Programs", path: "/references/examples" }
      ]
    }
  ],
  
  // Browser settings
  browser: {
    headless: true,
    timeout: 60000,
    waitUntil: 'networkidle2'
  },
  
  // Navigation page settings
  navPage: {
    title: "Documentation Navigation",
    filename: "index.html",
    description: "Local documentation copy"
  }
};

// Utility to sanitize filenames
function sanitizeFilename(filename) {
  return filename.replace(/[^a-z0-9]/gi, '-').toLowerCase();
}

// Create the output directory structure
async function createDirectoryStructure() {
  const outputDir = path.resolve(CONFIG.outputDir);
  await fs.ensureDir(outputDir);
  console.log(`Created output directory: ${outputDir}`);
  
  // Create section directories
  for (const section of CONFIG.sections) {
    const sectionDir = path.join(outputDir, sanitizeFilename(section.title));
    await fs.ensureDir(sectionDir);
  }
  
  return outputDir;
}

// Download a single page
async function downloadPage(browser, pageUrl, outputPath) {
  console.log(`Downloading: ${pageUrl}`);
  
  const page = await browser.newPage();
  try {
    await page.goto(pageUrl, { 
      waitUntil: CONFIG.browser.waitUntil,
      timeout: CONFIG.browser.timeout
    });
    
    // Get the page content
    const content = await page.content();
    
    // Save the content
    await fs.writeFile(outputPath, content);
    console.log(`Saved: ${outputPath}`);
    
    return content;
  } catch (error) {
    console.error(`Error downloading ${pageUrl}:`, error.message);
    return null;
  } finally {
    await page.close();
  }
}

// Create navigation page
async function createNavigationPage(outputDir) {
  const navFilePath = path.join(outputDir, CONFIG.navPage.filename);
  
  let navContent = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>${CONFIG.navPage.title}</title>
    <style>
        body {
            font-family: Arial, sans-serif;
            line-height: 1.6;
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
        }
        h1, h2, h3 {
            color: #333;
        }
        a {
            color: #0066cc;
            text-decoration: none;
        }
        a:hover {
            text-decoration: underline;
        }
        .section {
            margin-bottom: 30px;
            padding: 15px;
            background-color: #f9f9f9;
            border-radius: 5px;
        }
        .section h2 {
            margin-top: 0;
            border-bottom: 1px solid #ddd;
            padding-bottom: 10px;
        }
        ul {
            padding-left: 20px;
        }
        li {
            margin-bottom: 8px;
        }
    </style>
</head>
<body>
    <h1>${CONFIG.navPage.title}</h1>
    <p>${CONFIG.navPage.description} downloaded from <a href="${CONFIG.baseUrl}" target="_blank" rel="noopener">${new URL(CONFIG.baseUrl).hostname}</a>.</p>
`;

  // Add sections and links
  for (const section of CONFIG.sections) {
    navContent += `
    <div class="section">
        <h2>${section.title}</h2>
        <ul>`;
        
    for (const page of section.pages) {
      const sectionDir = sanitizeFilename(section.title);
      const pageFilename = page.path ? sanitizeFilename(page.path) + '.html' : 'index.html';
      const relativePath = `${sectionDir}/${pageFilename}`;
      
      navContent += `
            <li><a href="${relativePath}">${page.title}</a></li>`;
    }
    
    navContent += `
        </ul>
    </div>`;
  }
  
  navContent += `
    <footer>
        <p>Downloaded on ${new Date().toLocaleDateString()}</p>
    </footer>
</body>
</html>`;

  await fs.writeFile(navFilePath, navContent);
  console.log(`Created navigation page: ${navFilePath}`);
}

// Main function
async function main() {
  console.log('Starting documentation scraper...');
  console.log(`Base URL: ${CONFIG.baseUrl}`);
  console.log(`Output directory: ${CONFIG.outputDir}`);
  
  // Create directory structure
  const outputDir = await createDirectoryStructure();
  
  // Launch browser
  const browser = await puppeteer.launch({ 
    headless: CONFIG.browser.headless 
  });
  
  try {
    // Download all pages
    for (const section of CONFIG.sections) {
      console.log(`Processing section: ${section.title}`);
      const sectionDir = path.join(outputDir, sanitizeFilename(section.title));
      
      for (const page of section.pages) {
        const pageUrl = `${CONFIG.baseUrl}${page.path}`;
        const pageFilename = page.path ? sanitizeFilename(page.path) + '.html' : 'index.html';
        const outputPath = path.join(sectionDir, pageFilename);
        
        await downloadPage(browser, pageUrl, outputPath);
      }
    }
    
    // Create navigation page
    await createNavigationPage(outputDir);
    
    console.log('Documentation scraping completed successfully!');
  } catch (error) {
    console.error('Error during scraping:', error);
  } finally {
    await browser.close();
  }
}

// Run the script
main().catch(console.error);