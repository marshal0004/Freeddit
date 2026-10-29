# REA Analysis: Notes App — Export Notes Feature Trace

**Evidence ID**: `ev_2c80d0c7fd86986e5c9fe635f3527efdba3aa21e85da2ea`
**Root artifact SHA-256**: `97540263ab87c02e5ce24cea7e1b5c308a941e62...`
**Analysis tool**: `rea analyze-javascript-application`
**Date**: 2026-10-29

## Feature: Export notes (CSV export)

### How it works — full trace with evidence

The Export feature flows through 4 files across the Electron main↔preload↔renderer boundary:

```
[User clicks button]
      ↓
index.html:8         <button id="export">Export notes</button>
      ↓
renderer.js:6        document.querySelector("#export").addEventListener("click", ...)
      ↓
renderer.js:1-4      async function exportNotes() {
                        const destination = await window.notes.exportCsv();
                        document.querySelector("#status").textContent = `Saved to ${destination}`;
                      }
      ↓
preload.js:1-5       contextBridge.exposeInMainWorld("notes", {
                        exportCsv: () => ipcRenderer.invoke("notes:export"),
                      });
      ↓ (IPC boundary: renderer → main)
main.js:8-12         ipcMain.handle("notes:export", () => {
                        const destination = path.join(app.getPath("downloads"), "notes.csv");
                        writeFileSync(destination, toCsv(notes), "utf8");
                        return destination;
                      });
      ↓
main.js:6            const notes = [{ id: 1, title: "First note" }];
      ↓
csv.js:1-9           function toCsv(notes) {
                        const quote = (value) => `"${String(value).replaceAll('"', '""')}"`;
                        return [
                          "id,title",
                          ...notes.map((note) => `${note.id},${quote(note.title)}`),
                        ].join("\n") + "\n";
                      }
      ↓
[File written to ~/Downloads/notes.csv]
      ↓
main.js:11           return destination;  ← path returned to renderer
      ↓ (IPC boundary: main → renderer)
renderer.js:3        document.querySelector("#status").textContent = `Saved to ${destination}`;
```

### Error handling

- **renderer.js:7-9**: If `exportNotes()` rejects, the `.catch()` handler sets `#status` to `"Export failed"` and logs to console.
- **main.js:8-12**: The IPC handler has NO try/catch. If `writeFileSync` throws, the error propagates through the IPC promise rejection to the renderer's `.catch()`.

### REA semantic graph evidence (call sites with file:line:column)

| Semantic node | File | Line | Col | Kind | What it does |
|---|---|---|---|---|---|
| `require("electron")` | main.js | 1 | 13 | expression | Imports Electron main process APIs |
| `require("node:fs")` | main.js | 2 | 26 | call-site | Imports `writeFileSync` |
| `require("node:path")` | main.js | 3 | 13 | expression | Imports `path.join` |
| `const notes = [...]` | main.js | 6 | — | expression | Hardcoded notes data |
| `ipcMain.handle("notes:export", ...)` | main.js | 8 | 15 | expression | Registers IPC handler for export |
| `path.join(app.getPath("downloads"), "notes.csv")` | main.js | 10 | 16 | expression | Builds destination file path |
| `app.getPath("downloads")` | main.js | 10 | 29 | call-site | Gets OS downloads directory |
| `writeFileSync(...)` | main.js | 11 | — | call-site | Writes CSV to disk (sync) |
| `return destination` | main.js | 11 | — | return-site | Returns path to renderer |
| `app.whenReady().then(...)` | main.js | 14 | 0 | promise | App lifecycle: creates window when ready |
| `new BrowserWindow(...)` | main.js | 15 | 8 | expression | Creates browser window with preload |
| `window.loadFile("index.html")` | main.js | 18 | 2 | expression | Loads the HTML UI |
| `require("electron")` | preload.js | 1 | 8 | expression | Imports Electron preload APIs |
| `contextBridge` | preload.js | 1 | 39 | expression | Gets contextBridge API |
| `notes` parameter | csv.js | 1 | 15 | parameter | Input to toCsv() |
| `return ...` | csv.js | 2 | 27 | return-site | Returns the quote() helper |
| `quote(note.title)` | csv.js | 6 | 20 | expression | Calls quote() on title |
| `note.title` | csv.js | 6 | 32 | expression | Accesses title property |
| `addEventListener("click", ...)` | renderer.js | 6 | 0 | expression | Registers click handler |
| `exportNotes()` callback | renderer.js | 6 | 60 | function | The click callback function |
| `window.notes.exportCsv()` | renderer.js | 7 | 2 | expression | Calls IPC bridge |

### REA graph evidence (Electron boundary nodes)

| Graph node kind | Count | What it represents |
|---|---|---|
| `electron-main` | 1 | main.js — the Electron main process entry point |
| `electron-preload` | 1 | preload.js — the preload script (bridge) |
| `electron-renderer` | 2 | renderer.js + index.html — the renderer process |
| `browser-window` | 1 | The BrowserWindow created in main.js:15 |
| `ipc-handler` | 1 | The `notes:export` IPC handler in main.js:8 |
| `ipc-channel` | 1 | The `notes:export` channel name |
| `context-bridge-api` | 1 | The `window.notes` API exposed in preload.js |
| `javascript-module` | 14 | Module-level scopes in the JS files |
| `javascript-asset` | 3 | HTML and package.json assets |
| `javascript-expression` | 4 | Top-level expressions |
| `package` | 1 | package.json |
| `artifact` | 3 | Container artifacts |

### IPC boundary (from REA summary)

| Metric | Value |
|---|---|
| Browser windows | 1 |
| Explicit webPreferences | 1 |
| Preload entrypoints | 1 |
| Context bridge APIs | 1 |
| Exposed API members | 1 (`exportCsv`) |
| IPC items | 9 |
| Sender validation observations | 0 |
| Utility processes | 0 |

## What is CONFIRMED (with evidence)

1. **The button triggers export** — `index.html:8` has `<button id="export">`, and `renderer.js:6` registers a click listener on `#export`.
2. **Export goes through IPC** — `renderer.js:7` calls `window.notes.exportCsv()`, which `preload.js:4` maps to `ipcRenderer.invoke("notes:export")`. REA's semantic graph confirms this call-site at `renderer.js:7:2`.
3. **The IPC handler is `notes:export`** — `main.js:8` registers `ipcMain.handle("notes:export", ...)`. REA's graph includes an `ipc-handler` node and an `ipc-channel` node for this.
4. **The export writes to the Downloads directory** — `main.js:10` calls `path.join(app.getPath("downloads"), "notes.csv")`. REA's semantic graph confirms the `app.getPath("downloads")` call-site at `main.js:10:29`.
5. **The file is written synchronously** — `main.js:11` calls `writeFileSync(destination, toCsv(notes), "utf8")`. REA confirms the `require("node:fs")` call-site at `main.js:2:26`.
6. **The notes data is hardcoded** — `main.js:6` has `const notes = [{ id: 1, title: "First note" }]`. No database, no API fetch, no user input — the data is a constant array.
7. **CSV format is `id,title` with quoting** — `csv.js:1-9`: header is `"id,title"`, each row is `${note.id},${quote(note.title)}`, and `quote()` wraps in double quotes with `"` escaped as `""`. REA confirms the `notes` parameter at `csv.js:1:15` and the return site at `csv.js:2:27`.
8. **The destination path is returned to the renderer** — `main.js:11` returns `destination`, and `renderer.js:3` displays it in `#status` as `Saved to ${destination}`.
9. **Errors are caught in the renderer** — `renderer.js:7-9` has a `.catch()` that sets `#status` to `"Export failed"` and logs to console.
10. **The preload exposes exactly one API member** — `preload.js:3-5` exposes `{ exportCsv: ... }` via `contextBridge.exposeInMainWorld("notes", ...)`. REA's summary confirms `exposed_api_members: 1`.
11. **No sender validation** — REA's summary shows `sender_validation_observations: 0`. The IPC handler does not validate the sender's origin or frame.

## What is INFERRED (not directly confirmed by static analysis)

1. **The file write succeeds at runtime** — Static analysis confirms `writeFileSync` is called with the right arguments, but cannot confirm the OS allows the write (permissions, disk space, etc.).
2. **The CSV output is valid CSV** — The code looks correct (RFC 4180 quoting), but static analysis cannot execute it to verify the output matches `id,title\n1,"First note"\n`.
3. **`app.getPath("downloads")` returns the OS Downloads directory** — This is Electron API behavior, confirmed by Electron docs but not by static analysis of this code.
4. **The IPC message reaches the handler** — The channel name `"notes:export"` matches on both sides (`preload.js:4` and `main.js:8`), but static analysis cannot confirm the IPC message is actually delivered at runtime.
5. **`app.whenReady()` fires before the user clicks Export** — The code assumes the app is ready when the button is clicked, but this is a runtime guarantee, not a static fact.
6. **No additional IPC handlers exist** — REA found 1 IPC handler, but dynamic registration at runtime (e.g., from eval'd code) cannot be ruled out by static analysis.

## What is STILL UNKNOWN

1. **What happens if `notes.csv` already exists** — `writeFileSync` overwrites by default, but this is Node.js runtime behavior, not visible in the code.
2. **What happens with concurrent exports** — Multiple rapid clicks could trigger multiple `writeFileSync` calls. The code has no debounce or lock.
3. **Whether the CSV encoding handles non-ASCII characters** — `utf8` is passed to `writeFileSync`, but `String(value)` and `.replaceAll()` behavior with unicode is not statically verifiable.
4. **Whether notes can have additional fields** — The hardcoded data only has `id` and `title`, but `toCsv()` hardcodes the CSV columns as `id,title`. If notes had a `body` field, it would not be exported.
5. **Whether the app runs at all** — Static analysis never executes the code. Runtime errors (missing Electron, wrong Node version, etc.) are outside REA's scope.
6. **Network behavior** — REA's route inventory found 0 routes and 0 occurrences. The app makes no network requests, but this only covers static code — runtime `fetch()` from eval'd code would be invisible.

## REA Limitations (from the analysis)

1. CommonJS/ESM binding relationships were recovered from inert syntax only.
2. Electron API identity follows proven lexical module aliases — other reassignments are unresolved.
3. Electron relationships are derived from inert syntax — runtime registration is unproven.
4. Graph coverage is incomplete — unavailable facts remain explicit.
5. JavaScript and HTML were parsed as inert text — bundle bootstrap was never executed.
6. Static imports, entrypoints, workers, endpoints, and storage relationships do not prove runtime execution.
7. Static paths and relationships may remain unresolved when expressions are dynamic or obfuscated.
8. Webpack/Rspack factories were recovered from AST literals — push handlers were not invoked.

## Test cases (from the trace)

| # | Input | Expected output |
|---|---|---|
| 1 | `[{ id: 1, title: "First note" }]` | `id,title\n1,"First note"\n` |
| 2 | `[{ id: 2, title: 'Has "quotes"' }]` | `id,title\n2,"Has ""quotes"""\n` |
| 3 | `[]` (empty array) | `id,title\n` (header only + trailing newline) |
| 4 | `[{ id: 1, title: "" }]` (empty title) | `id,title\n1,""\n` |
| 5 | `[{ id: 1, title: "Multi\nline" }]` | `id,title\n1,"Multi\nline"\n` (newline inside quotes — valid CSV per RFC 4180) |
| 6 | `[{ id: 1, title: "First" }, { id: 2, title: "Second" }]` | `id,title\n1,"First"\n2,"Second"\n` |
| 7 | Click Export with existing `notes.csv` in Downloads | File overwritten with same content |
| 8 | Click Export twice rapidly | Both writes complete (no lock), second overwrites first |
| 9 | Export fails (e.g., Downloads dir not writable) | `#status` shows "Export failed", error logged to console |
| 10 | Notes have extra fields `{ id: 1, title: "T", body: "B" }` | CSV contains only `id,title` (body is ignored) |
