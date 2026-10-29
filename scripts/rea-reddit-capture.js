#!/usr/bin/env node
//
// rea-reddit-capture.js — Download ALL of Reddit's JavaScript using a real browser
//
// This is the CORRECT approach: Playwright launches your actual Chrome,
// navigates to reddit.com, and saves every JS/CSS file that loads.
// curl CANNOT do this because Reddit uses nonce-based script protection.
//
// Usage:
//   node rea-reddit-capture.js
//
// Then:
//   rea analyze-javascript-application ~/rea-output/reddit-js --format json > ~/rea-output/analysis.json
//

const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const OUTPUT_DIR = path.join(require('os').homedir(), 'rea-output');
const JS_DIR = path.join(OUTPUT_DIR, 'reddit-js');

fs.mkdirSync(OUTPUT_DIR, { recursive: true });
fs.mkdirSync(JS_DIR, { recursive: true });

async function main() {
    console.log('\n[1/6] Launching Chrome browser...');
    
    const browser = await chromium.launch({
        headless: false,
        channel: 'chrome',  // Use your installed Google Chrome
        args: ['--disable-blink-features=AutomationControlled']
    });
    
    const context = await browser.newContext({
        userAgent: 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36',
        viewport: { width: 1920, height: 1080 },
        locale: 'en-US',
    });
    
    const page = await context.newPage();
    
    // Track all downloaded files
    const downloadedFiles = [];
    const jsUrls = [];
    const cssUrls = [];
    const apiUrls = [];
    
    // Intercept ALL network responses
    console.log('[2/6] Setting up network interception...');
    
    page.on('response', async (response) => {
        const url = response.url();
        const contentType = response.headers()['content-type'] || '';
        const status = response.status();
        
        try {
            // Save JavaScript files
            if (url.includes('.js') || contentType.includes('javascript')) {
                if (status === 200) {
                    const body = await response.body();
                    const hash = crypto.createHash('md5').update(url).digest('hex').slice(0, 12);
                    const filename = path.join(JS_DIR, `${hash}.js`);
                    fs.writeFileSync(filename, body);
                    jsUrls.push(url);
                    downloadedFiles.push({ url, type: 'js', size: body.length, file: filename });
                    console.log(`  ✓ JS: ${url.slice(0, 80)}... (${body.length} bytes)`);
                }
            }
            // Save CSS files
            else if (url.includes('.css') || contentType.includes('text/css')) {
                if (status === 200) {
                    const body = await response.body();
                    const hash = crypto.createHash('md5').update(url).digest('hex').slice(0, 12);
                    const filename = path.join(JS_DIR, `${hash}.css`);
                    fs.writeFileSync(filename, body);
                    cssUrls.push(url);
                    downloadedFiles.push({ url, type: 'css', size: body.length, file: filename });
                    console.log(`  ✓ CSS: ${url.slice(0, 80)}... (${body.length} bytes)`);
                }
            }
            // Log API calls
            else if (url.includes('/api/') || url.includes('/svc/') || url.includes('.json')) {
                if (status === 200) {
                    apiUrls.push({ url, status, contentType });
                    console.log(`  📡 API: ${url.slice(0, 80)}... (${status})`);
                }
            }
        } catch (e) {
            // Some responses can't be read (redirects, etc.) — skip
        }
    });
    
    // Navigate to Reddit
    console.log('\n[3/6] Navigating to https://www.reddit.com/r/programming/ ...');
    await page.goto('https://www.reddit.com/r/programming/', { 
        waitUntil: 'networkidle',
        timeout: 60000 
    });
    
    console.log('  Page loaded! Waiting for JS to load...');
    await page.waitForTimeout(5000);
    
    // Scroll to trigger lazy-loaded JS
    console.log('  Scrolling to load more content...');
    for (let i = 0; i < 3; i++) {
        await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
        await page.waitForTimeout(2000);
    }
    
    // Click on a post to load post-detail JS
    console.log('  Clicking a post to load post-detail JS...');
    try {
        const firstPost = await page.$('article, [data-testid="post-container"], shreddit-post');
        if (firstPost) {
            await firstPost.click();
            await page.waitForTimeout(5000);
        }
    } catch (e) {
        console.log('  (could not click post — continuing)');
    }
    
    // Go back and try comments
    console.log('  Going back to try more pages...');
    await page.goBack().catch(() => {});
    await page.waitForTimeout(3000);
    
    // Save screenshot
    console.log('\n[4/6] Saving screenshot + DOM...');
    await page.screenshot({ path: path.join(OUTPUT_DIR, 'reddit-screenshot.png'), fullPage: false });
    console.log(`  Screenshot saved`);
    
    const html = await page.content();
    fs.writeFileSync(path.join(OUTPUT_DIR, 'reddit-dom.html'), html);
    console.log(`  DOM saved: ${html.length} bytes`);
    
    // Save the page's title to verify we got real content
    const title = await page.title();
    console.log(`  Page title: "${title}"`);
    
    // Close browser
    await browser.close();
    
    // Save URL lists
    console.log('\n[5/6] Saving URL lists...');
    fs.writeFileSync(path.join(OUTPUT_DIR, 'js-urls.txt'), jsUrls.join('\n'));
    fs.writeFileSync(path.join(OUTPUT_DIR, 'css-urls.txt'), cssUrls.join('\n'));
    fs.writeFileSync(path.join(OUTPUT_DIR, 'api-urls.txt'), 
        apiUrls.map(a => `${a.status} ${a.contentType} ${a.url}`).join('\n'));
    
    // Summary
    console.log('\n[6/6] Summary');
    console.log('========================================');
    console.log(`JavaScript files: ${jsUrls.length}`);
    console.log(`CSS files: ${cssUrls.length}`);
    console.log(`API calls: ${apiUrls.length}`);
    console.log(`Total downloaded: ${downloadedFiles.length}`);
    console.log(`Output: ${OUTPUT_DIR}/`);
    console.log(`JS files: ${JS_DIR}/`);
    console.log('');
    
    // List files
    const files = fs.readdirSync(JS_DIR);
    console.log(`Files in ${JS_DIR}/:`);
    files.forEach(f => {
        const size = fs.statSync(path.join(JS_DIR, f)).size;
        console.log(`  ${f} (${size} bytes)`);
    });
    console.log('');
    
    if (jsUrls.length > 0) {
        console.log('NEXT STEP: Run REA analysis on the JS files:');
        console.log('');
        console.log(`  rea analyze-javascript-application ${JS_DIR} --format json > ${OUTPUT_DIR}/analysis-evidence.json`);
        console.log('');
        console.log('Then send me the analysis-evidence.json file!');
    } else {
        console.log('No JS files captured. Reddit may have blocked the browser.');
        console.log('Try: Open Chrome manually, go to reddit.com, then re-run this script.');
    }
    
    console.log('');
}

main().catch(e => {
    console.error('Error:', e.message);
    process.exit(1);
});
