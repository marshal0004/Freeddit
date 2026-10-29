# Feature Map Template (from the guide's section 05)

Fill this in for any business you want to build custom software for.
One row per feature the business actually uses.

Business: ________   Tool: ________   Cost per year: ________

| Feature they use | Who uses it | How often | Where it runs (their machine or the vendor's server) | Can we see it legally? |
|------------------|-------------|-----------|-------------------------------------------------------|------------------------|
|                  |             |           |                                                       |                        |
|                  |             |           |                                                       |                        |
|                  |             |           |                                                       |                        |

## How to use this template

### Step 1: Understand the business problem (guide section 05, step 1)
Sit with the owner or team for 30 minutes. Ask where their time and money go.
List every paid tool they use, what it costs per year, and who uses it.
You are looking for one expensive tool that does a simple job for them.

### Step 2: Ask which features they actually use (guide section 05, step 2)
Ask directly:
- Which apps do you open every single day?
- If you could keep only 3 features of this tool, which 3?
- What do you wish it did that it does not?
- What do you export or copy out of it by hand?

Write the answers into the feature map above. One row per feature.

### Step 3: Reverse engineer those features (guide section 05, step 3)
For each feature on the map that you are allowed to examine, give your agent
one narrow question. "How does the export decide which fields to include?" works.
"Clone this app" does not.

Ask for evidence: the file, function, or request behind the answer, plus what it
could not confirm. For features that live on a vendor's server, use behavior,
exports, and public docs instead.

Then turn each feature into a SPEC.md (see SPEC.md for the format).

### Step 4: Build custom software from the spec (guide section 05, step 4)
Build fresh code from the spec, not from anything you found inside the app.
Test it against the behavior you recorded.

### Step 5: Charge well (guide section 05, step 5)
Price on the value you create, not on your hours. The business compares you to
the yearly bill they already pay.

## Example: Notes app feature map

Business: Example Business   Tool: Notes (Electron app)   Cost per year: $0 (open source)

| Feature they use | Who uses it | How often | Where it runs | Can we see it legally? |
|------------------|-------------|-----------|---------------|------------------------|
| Export notes to CSV | All users | Daily | Their machine (Electron main process writes file locally) | Yes — we own the app and have the source |

## Safety rules (from guide section 04)

1. **Start with what the business owns.** Their own old app, their internal tools,
   their own exports and data. That is the easy, clean case.
2. **For anyone else's software, get permission and read the license.** Many
   licenses and terms of service limit reverse engineering. If it is not allowed,
   you do not do it.
3. **Understand behavior, then write fresh code.** Use the findings to write a
   plain specification, then build from that spec. Never paste their code,
   images, brand, or design into your project.
4. **Never unlock paid features** or bypass anything that asks you to log in or pay.
5. **Ask a lawyer when money is involved.** This is not legal advice.
