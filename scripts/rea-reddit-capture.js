const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const OUTPUT_DIR = path.join(require('os').homedir(), 'rea-output');
const JS_DIR = path.join(OUTPUT_DIR, 'reddit-js');
fs.mkdirSync(OUTPUT_DIR, { recursive: true });
fs.mkdirSync(JS_DIR, { recursive: true });

async function main() {
    console.log('[1/5] Launching Chrome...');
    
    // Use the installed Google Chrome (not Playwright's bundled Chromium)
    const browser = await chromium.launch({
        headless: false,
        channel: 'chrome',  // Use system Google Chrome
    });
    
    const context = await browser.newContext({
        userAgent: 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36',
        viewport: { width: 1920, height: 1080 },
    });
    
    const page = await context.newPage();
    const jsUrls = [], cssUrls = [], apiCalls = [];
    
    // Intercept ALL responses — save JS and CSS files
    page.on('response', async (response) => {
        const url = response.url();
        const ct = response.headers()['content-type'] || '';
        const status = response.status();
        
        try {
            if (status !== 200) return;
            
            if (ct.includes('javascript') || url.match(/\.js($|\?)/)) {
                const body = await response.body();
                const hash = crypto.createHash('md5').update(url).digest('hex').slice(0, 12);
                fs.writeFileSync(path.join(JS_DIR, `${hash}.js`), body);
                jsUrls.push(url);
                console.log(`  ✓ JS [${jsUrls.length}] ${url.slice(0, 90)}`);
            } else if (ct.includes('text/css') || url.match(/\.css($|\?)/)) {
                const body = await response.body();
                const hash = crypto.createHash('md5').update(url).digest('hex').slice(0, 12);
                fs.writeFileSync(path.join(JS_DIR, `${hash}.css`), body);
                cssUrls.push(url);
                console.log(`  ✓ CSS [${cssUrls.length}] ${url.slice(0, 90)}`);
            } else if (url.includes('/api/') || url.includes('/svc/') || ct.includes('application/json')) {
                apiCalls.push(`${status} ${ct} ${url}`);
            }
        } catch (e) { /* skip unreadable responses */ }
    });
    
    // Navigate
    console.log('[2/5] Navigating to reddit.com/r/programming/ ...');
    await page.goto('https://www.reddit.com/r/programming/', { 
        waitUntil: 'networkidle', timeout: 60000 
    });
    console.log(`  Page title: "${await page.title()}"`);
    
    // Wait for JS to load
    await page.waitForTimeout(5000);
    
    // Scroll to trigger lazy loading
    console.log('[3/5] Scrolling to load more JS...');
    for (let i = 0; i < 3; i++) {
        await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
        await page.waitForTimeout(2000);
    }
    
    // Click a post to load post-detail JS
    console.log('[4/5] Clicking a post to load more JS...');
    try {
        await page.click('a[href*="/comments/"]', { timeout: 5000 });
        await page.waitForTimeout(5000);
    } catch (e) {
        console.log('  (no post link found, continuing)');
    }
    
    // Save screenshot + DOM
    await page.screenshot({ path: path.join(OUTPUT_DIR, 'reddit-screenshot.png') });
    fs.writeFileSync(path.join(OUTPUT_DIR, 'reddit-dom.html'), await page.content());
    console.log(`  Screenshot + DOM saved`);
    
    await browser.close();
    
    // Save URL lists
    fs.writeFileSync(path.join(OUTPUT_DIR, 'js-urls.txt'), jsUrls.join('\n'));
    fs.writeFileSync(path.join(OUTPUT_DIR, 'css-urls.txt'), cssUrls.join('\n'));
    fs.writeFileSync(path.join(OUTPUT_DIR, 'api-calls.txt'), apiCalls.join('\n'));
    
    console.log(`\n[5/5] Done!`);
    console.log(`  JS files: ${jsUrls.length}`);
    console.log(`  CSS files: ${cssUrls.length}`);
    console.log(`  API calls: ${apiCalls.length}`);
    console.log(`  Output: ${OUTPUT_DIR}/`);
    
    if (jsUrls.length > 0) {
        console.log(`\nNEXT: Run REA analysis:`);
        console.log(`  rea analyze-javascript-application ${JS_DIR} --format json > ${OUTPUT_DIR}/analysis-evidence.json`);
    }
}

main().catch(e => { console.error(e.message); process.exit(1); });
