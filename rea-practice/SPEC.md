# Feature: Export notes (CSV)

What it does (one sentence): Exports all notes to a CSV file in the user's Downloads directory.

Inputs:
- None from the user (the button click is the only input)
- Notes data is a hardcoded array: `[{ id: 1, title: "First note" }]`

Outputs:
- A file named `notes.csv` in the OS Downloads directory
- CSV content: `id,title\n1,"First note"\n`
- The destination file path is displayed in the UI status element

Rules we confirmed (with evidence):
- The export is triggered by clicking a button (`index.html:8`, `renderer.js:6`)
- The export goes through Electron IPC: renderer → preload → main (`renderer.js:7`, `preload.js:4`, `main.js:8`)
- The IPC channel name is `notes:export` (`preload.js:4` and `main.js:8` match)
- The destination is always `~/Downloads/notes.csv` (`main.js:10`: `path.join(app.getPath("downloads"), "notes.csv")`)
- The file is written synchronously (`main.js:11`: `writeFileSync`)
- CSV header is `id,title` (`csv.js:5`)
- Each note becomes one CSV row: `${note.id},${quote(note.title)}` (`csv.js:6`)
- Titles are wrapped in double quotes with `"` escaped as `""` (`csv.js:2`: `quote()` function)
- The CSV ends with a trailing newline (`csv.js:8`: `+ "\n"`)
- The destination path is returned to the renderer and displayed (`main.js:11` returns it, `renderer.js:3` shows it)
- Errors are caught in the renderer and displayed as "Export failed" (`renderer.js:7-9`)
- Only `id` and `title` fields are exported — no other note fields appear in the CSV (`csv.js:5` hardcodes the columns)
- No sender validation on the IPC handler (REA summary: `sender_validation_observations: 0`)

Rules we inferred (not confirmed):
- The file write succeeds at runtime (static analysis confirms the call but cannot verify OS behavior)
- The CSV output is valid RFC 4180 CSV (the quoting logic looks correct but was not executed)
- `app.getPath("downloads")` returns the OS Downloads directory (Electron API behavior, confirmed by docs)
- The IPC message is delivered at runtime (channel names match, but delivery is a runtime guarantee)
- `writeFileSync` overwrites existing files (Node.js runtime behavior, not visible in code)

Edge cases:
- Empty notes array → CSV is `id,title\n` (header only + trailing newline)
- Title containing double quotes → escaped as `""` (e.g., `Has "quotes"` becomes `"Has ""quotes"""`)
- Title containing newlines → preserved inside quotes (valid per RFC 4180)
- Empty title → becomes `""` in the CSV
- File already exists → overwritten (writeFileSync default behavior)
- Downloads directory not writable → error propagates to renderer's `.catch()`, shows "Export failed"
- Multiple rapid clicks → multiple writes, second overwrites first (no debounce/lock)
- Notes with extra fields (e.g., `body`) → extra fields are ignored (only `id` and `title` in CSV)

Still unknown:
- What happens with very large note arrays (memory/performance)
- Whether the app handles non-ASCII characters correctly in the CSV (utf8 is specified but untested)
- Whether any runtime errors could occur from Electron API changes between versions
- Whether concurrent exports from multiple windows could cause issues

Test cases (input, expected output):
1. `[{ id: 1, title: "First note" }]` → `id,title\n1,"First note"\n`
2. `[{ id: 2, title: 'Has "quotes"' }]` → `id,title\n2,"Has ""quotes"""\n`
3. `[]` → `id,title\n`
4. `[{ id: 1, title: "" }]` → `id,title\n1,""\n`
5. `[{ id: 1, title: "Multi\nline" }]` → `id,title\n1,"Multi\nline"\n`
6. `[{ id: 1, title: "First" }, { id: 2, title: "Second" }]` → `id,title\n1,"First"\n2,"Second"\n`
7. Click Export with existing `notes.csv` → file overwritten
8. Click Export twice rapidly → second write overwrites first
9. Export with unwritable Downloads dir → "Export failed" shown, error logged
10. `[{ id: 1, title: "T", body: "B" }]` → CSV is `id,title\n1,"T"\n` (body ignored)
