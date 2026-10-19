// Freeddit Electron main process.
// Loads the Next.js dev server in development, or the built static export in production.
// In production, this would also spawn the Rust backend as a sidecar binary.

const { app, BrowserWindow, shell } = require('electron');
const path = require('path');

const isDev = process.argv.includes('--dev');
const isProd = !isDev;

// In production, the Rust backend ships as a sidecar. Spawn it on app start.
let backendProcess = null;
function spawnBackend() {
  if (isProd) {
    const { spawn } = require('child_process');
    const backendPath = path.join(process.resourcesPath, 'backend', 'freeddit-backend');
    const dbPath = path.join(app.getPath('userData'), 'freeddit.db');
    backendProcess = spawn(backendPath, [], {
      env: {
        ...process.env,
        DATABASE_URL: `sqlite://${dbPath}?mode=rwc`,
        PORT: '3001',
        JWT_SECRET: 'change-in-production',
      },
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    backendProcess.stdout.on('data', (d) => console.log('[backend]', d.toString()));
    backendProcess.stderr.on('data', (d) => console.error('[backend]', d.toString()));
    backendProcess.on('exit', (code) => console.log(`[backend] exited with ${code}`));
  }
}

async function createWindow() {
  const win = new BrowserWindow({
    width: 1400,
    height: 900,
    minWidth: 800,
    minHeight: 600,
    backgroundColor: '#030303',
    title: 'Freeddit',
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
    titleBarStyle: process.platform === 'darwin' ? 'hiddenInset' : 'default',
    trafficLightPosition: { x: 12, y: 14 },
  });

  // Open external links (reddit.com, etc.) in the system browser, not inside Electron.
  win.webContents.setWindowOpenHandler(({ url }) => {
    if (url.startsWith('http://') || url.startsWith('https://')) {
      shell.openExternal(url);
      return { action: 'deny' };
    }
    return { action: 'allow' };
  });

  if (isDev) {
    // In dev, load the Next.js dev server (running on :3000 by default).
    await win.loadURL('http://localhost:3000/');
    win.webContents.openDevTools({ mode: 'detach' });
  } else {
    // In production, load the built Next.js static export.
    const serve = require('electron-serve');
    const loadURL = serve({ directory: path.join(__dirname, '..', 'out') });
    await loadURL(win);
  }
}

app.whenReady().then(async () => {
  spawnBackend();
  await createWindow();

  app.on('activate', async () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      await createWindow();
    }
  });
});

app.on('window-all-closed', () => {
  if (backendProcess) {
    backendProcess.kill('SIGTERM');
  }
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('before-quit', () => {
  if (backendProcess) {
    backendProcess.kill('SIGTERM');
  }
});
