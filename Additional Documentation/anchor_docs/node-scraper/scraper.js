const puppeteer = require('puppeteer');
const fs = require('fs-extra');
const path = require('path');

const BASE_URL = 'https://www.anchor-lang.com/docs';
const OUTPUT_DIR = path.join(__dirname, '../anchor-docs-scraped');

// URLs to visit (will be populated as we discover links)
const urlsToVisit = new Set([BASE_URL]);
const visitedUrls = new Set();

// Create output directory
fs.ensureDirSync(OUTPUT_DIR);

async function savePageContent(url, content, browser) {
  // Extract relative path from URL
  let relativePath = url.replace(BASE_URL, '').replace(/^\/?/, '');
  
  // If it's the base URL or empty path, use 'index.html'
  if (!relativePath) {
    relativePath = 'index.html';
  } else {
    // Add .html extension if not present
    if (!relativePath.endsWith('.html') && !relativePath.includes('.')) {
      relativePath = `${relativePath}/index.html`;
    }
  }
  
  // Create full file path
  const filePath = path.join(OUTPUT_DIR, relativePath);
  
  // Ensure directory exists
  fs.ensureDirSync(path.dirname(filePath));
  
  // Save the content
  fs.writeFileSync(filePath, content);
  console.log(`Saved: ${filePath}`);
  
  // Extract and queue new links
  const page = await browser.newPage();
  await page.setContent(content);
  
  const links = await page.evaluate(() => {
    return Array.from(document.querySelectorAll('a[href]'))
      .map(a => a.href)
      .filter(href => href.startsWith('https://www.anchor-lang.com/docs'));
  });
  
  await page.close();
  
  // Add new links to the queue
  for (const link of links) {
    if (!visitedUrls.has(link) && !urlsToVisit.has(link)) {
      urlsToVisit.add(link);
    }
  }
}

async function scrapeAnchorDocs() {
  console.log('Starting Anchor documentation scraper...');
  
  const browser = await puppeteer.launch();
  
  try {
    while (urlsToVisit.size > 0) {
      // Get next URL to visit
      const url = Array.from(urlsToVisit)[0];
      urlsToVisit.delete(url);
      visitedUrls.add(url);
      
      console.log(`Processing: ${url}`);
      
      // Open page
      const page = await browser.newPage();
      await page.goto(url, { waitUntil: 'networkidle2', timeout: 60000 });
      
      // Wait for content to load (adjust selectors based on the website structure)
      await page.waitForSelector('body', { timeout: 10000 });
      
      // Get page content
      const content = await page.content();
      
      // Save page content and extract links
      await savePageContent(url, content, browser);
      
      await page.close();
    }
    
    console.log('Scraping completed successfully!');
  } catch (error) {
    console.error('Error during scraping:', error);
  } finally {
    await browser.close();
  }
}

// Start the scraping process
scrapeAnchorDocs().catch(console.error);