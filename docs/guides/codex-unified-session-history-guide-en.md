# Unified Codex Session History: Use and Troubleshooting

> Applies to FyAgent v3.16.x and later. This guide is based on the current code; every command and path can be verified by hand. Examples use de-identified data and contain no real session content or API keys.

## What this feature is

"Unified Codex session history" is a switch that FyAgent v3.16.x adds for Codex. You'll find it under **Settings -> General -> the "Codex App Enhancements" group** ("Codex App Enhancements" is the group title; the switch itself is called "Unified Codex session history"). Once enabled, **sessions from your official subscription (ChatGPT login / OpenAI API key) appear in the same history / resume list as sessions from every third-party provider FyAgent manages**—they are no longer split into two lists that can't see each other.

## What problem it solves

Codex classifies sessions by a "provider tag" (a field called `model_provider`), and **the resume / history list only shows sessions whose tag matches your currently active provider**. As a result, sessions are naturally sorted into two separate "drawers":

- Sessions from your official subscription go under Codex's built-in **`openai`** tag;
- Every third-party provider FyAgent manages goes under the **`custom`** tag.

The two drawers can't see each other. If you **switch frequently between official and third-party**, you'll hit this kind of fragmentation: "the session I was just chatting in with the official account disappeared from the history list after I switched to a third-party provider"—it isn't actually gone, it's just been sorted into the other drawer. This split both makes it easy to believe a session was lost, and makes it inconvenient to review and resume all your sessions in one place.

When this switch is enabled, the official subscription also uses the `custom` tag, so official and third-party sessions appear in the same history list.

> **Change boundary:** This feature adjusts the `model_provider` classification tag and creates a backup before rewriting it. Migration and restore do not use deletion of conversation content as a step. If the history list does not match your expectation, use the [symptom table](#sessions-are-missing-from-the-list-symptom-table), then follow the [read-only checks](#check-session-files-and-backups). Keep a separate backup of important sessions.

## How it works (one-line version)

Think of it as **two drawers + automatic backup**:

- By default, official sessions live in the `openai` drawer and third-party sessions live in the `custom` drawer, invisible to each other;
- The switch makes **the official side use the `custom` drawer too**, merging the two drawers into one shared list;
- You can optionally migrate **existing official sessions** into the shared list. FyAgent creates a backup first; when disabling the feature, it can restore the sessions recorded in that backup;
- The official subscription continues to use its existing ChatGPT login and official backend; this switch changes session classification.

For implementation details and recovery boundaries, see [Two drawers and automatic backup](#two-drawers-and-automatic-backup) and the [technical appendix](#technical-appendix).

## How to use it (at a glance)

1. **Enable**: Settings -> General -> Codex App Enhancements -> turn on "Unified Codex session history" -> in the dialog decide whether to check "Also migrate existing official session history" (check it if you want your **earlier** official sessions merged into the unified list too; leave it unchecked if you only want unification from now on) -> confirm. See [What happens when you enable it](#what-happens-when-you-enable-it-step-by-step).
2. **Disable**: turn the same switch off -> in the dialog keep "restore exactly from backup" checked (it's checked by default) -> confirm, and the official sessions you migrated in will be precisely flipped back to the official list. See [What happens when you disable it](#what-happens-when-you-disable-it-step-by-step).
3. **Sessions are missing from the list**: use the [symptom table](#sessions-are-missing-from-the-list-symptom-table), then follow the [read-only checks](#check-session-files-and-backups) to inspect session files, the index, and backups.

---

## Two drawers and automatic backup

This feature involves session classification and pre-write backups. This guide uses “drawer” as a plain-language name for a `model_provider` classification.

### Drawers: how Codex classifies sessions

Every time you start a Codex session, Codex records a tag `model_provider` in the session file header, marking "which provider this session was chatted with." Codex's **resume / history list is filtered precisely by the currently active tag**—it only shows sessions whose tag matches "the provider you're on right now."

- Sessions from your official subscription (ChatGPT login / OpenAI API key) carry the built-in tag **`openai`**.
- Every third-party provider FyAgent manages uses the tag **`custom`**.

So by default, official sessions and third-party sessions are inherently invisible to each other—they live in two different drawers. This is **Codex's own design**, not FyAgent losing anything.

```text
Default state (unified switch off):
   ┌───────────────────────┐     ┌──────────────────────────┐
   │  openai drawer        │     │  custom drawer           │
   │  (official sessions)  │     │  (third-party sessions)  │
   └───────────────────────┘     └──────────────────────────┘
       ▲                             ▲
     visible only while            visible only while
     on the official provider       on a third-party provider

   The two drawers can't see each other.
```

**What the "Unified Codex session history" switch does is make the official subscription run under the `custom` tag too, merging the two drawers into one**, so official and third-party sessions appear in the same resume list. Note: **authentication doesn't change**—your official subscription still uses your ChatGPT login and still goes through the official backend; only the session's "classification tag" changes from `openai` to `custom`.

```text
After the unified switch is on:
   ┌──────────────────────────────────────────────┐
   │             custom shared drawer             │
   │  official sessions  +  third-party sessions  │
   │  (appear in the same history / resume list)  │
   └──────────────────────────────────────────────┘
```

### Backups: a copy is made before every tag change

"Merging the drawers" requires changing the tag of some official sessions from `openai` to `custom` (this step is called **migration**, and it's **optional and requires you to opt in**). And **before any rewrite, FyAgent first copies the original file untouched** to here:

```text
~/.fyagent/backups/codex-official-history-unify-v1/<timestamp>/
```

This backup is the sole basis for "restore exactly from backup" later. It makes the whole process **reversible**: at any time you can turn off the switch and precisely flip the official sessions you migrated in back to the `openai` drawer.

Restore applies only to migrated sessions recorded in the backup ledger. Sessions created while the switch is enabled are not in that ledger.

---

## What happens when you enable it: step by step

### Step 1: Find the switch

```text
Settings -> General -> Codex App Enhancements
```

In the "Codex App Enhancements" block there are two rows of switches; the **second row** (the blue history icon) is the subject of this guide:

> **Unified Codex session history**

Below it is a line of description text (verbatim):

> When enabled, the official subscription runs under the shared "custom" provider id so official and third-party sessions appear in one history list, optionally migrating existing official sessions in (backed up first). When turning it off, the migrated sessions can be restored from backup. Note: resuming an old session across providers may fail because its encrypted_content reasoning can only be decrypted by the backend that created it.

> **Note**: this single line of description already previews three things—sessions will appear in one list, you can optionally migrate them in with an automatic backup, and resuming across providers "may fail." Here, "fail" means **you can't resume / can't generate a new turn**, not "the record is lost." This is exactly the core misunderstanding we'll dig into below.

### Step 2: Flip the switch from off to on -> a confirmation dialog pops up

The moment you flip the switch on, FyAgent **does not save immediately**; instead it first pops up a confirmation dialog. The dialog text reads as follows (verbatim):

- **Title**: Unified Codex session history
- **Body**:

  > When enabled, the official subscription and third-party providers share one session history list. Note: resuming an old session across providers may fail because its encrypted_content reasoning cannot be decrypted by another backend.
  >
  > You can also migrate your existing official session history into the shared list (originals are backed up to ~/.fyagent/backups first and can be restored when you turn this off).

- **Checkbox**: Also migrate existing official session history
- **Confirm button**: I understand, enable
- **Cancel button**: Cancel

**This checkbox is unchecked by default.** This is an important fork in the road:

| Your choice | Effect | Where your data is right now |
|---|---|---|
| **Unchecked** (default) | Only switches the tag. **Only official sessions created after enabling** land in the `custom` shared drawer | Your official sessions from **before** enabling keep the `openai` tag, stay exactly where they were, still in `~/.codex/sessions/` |
| **Checked** | In addition to switching the tag, also migrates your **existing official sessions** from the `openai` drawer into the `custom` drawer | After being **copied to backup**, the old sessions' tag is rewritten to `custom`; the original data is covered by the backup |

> **If you want "my earlier official sessions to appear in the unified list too," you must opt in by checking this box.** Otherwise you'll run into "scenario A" in the reference table below—the old sessions look "gone," when in fact they're just sitting in the original drawer.

Click "Cancel" or click outside the dialog: the switch flips straight back to off and nothing happens.
Click "I understand, enable": the switch is saved as on, and FyAgent persists the configuration in the background (and runs the migration if you checked it).

### Step 3 (only if you checked migration): how migration runs + data safety

If you check "Also migrate existing official session history," FyAgent runs this procedure on your existing official sessions:

```text
For each official (openai tag) session file:
   ① First copy the original file untouched into the backup directory   <- data now has its first safety net
   ② Using "write a temp file -> replace the whole thing" atomic style,
      change only the model_provider in the session_meta line at the header
      from "openai" to "custom"                                          <- not a single byte of the conversation body is touched
   ③ Update the index database state_5.sqlite to switch the tag in the same transaction
```

- **Backup location**: `~/.fyagent/backups/codex-official-history-unify-v1/<timestamp>/`. Each migration produces one timestamped "generation directory," containing `jsonl/` (session copies), `state/` (index DB copy), and `meta.json` (recording which Codex directory this migration belongs to).
- **What's changed**: the migration code updates `model_provider` and does not rewrite conversation entries.
- **Deletion behavior**: the migration path has no step that deletes sessions or index rows. It copies the backup, writes a temporary file, and then replaces the original to reduce the risk of leaving a partial file.

After a successful migration, the existing official sessions appear in the unified list. The backup directory contains the pre-write copies, and the active files use the new classification tag.

> **Note**: enabling and migration themselves **do not pop a success toast**. Migration runs as a side task on the backend during save; in the UI you'll only see the switch turn on. So "I didn't see a migration-success popup" is normal and does not mean failure.

---

## What happens when you disable it: step by step

### Step 1: Flip the switch from on to off -> probe for backups -> a confirmation dialog pops up

When disabling, FyAgent **first spends a moment probing whether there's a migration backup**, then pops up a confirmation dialog (so the disable dialog has a slight delay, which is normal). The text reads as follows (verbatim):

- **Title**: Turn off unified session history
- **Body**:

  > After turning this off, the official subscription and third-party providers return to separate history lists. Sessions created while it was on cannot be attributed to a provider, so they stay in the third-party history and the official subscription will not see them.

- **Checkbox** (shown conditionally): Restore the official sessions migrated at enable time back to the official history (exact restore from backup)
- **Confirm button**: Turn off
- **Cancel button**: Cancel

> **Key point**: the body says the official subscription **will not see them**—**won't see**, not **delete**. The new sessions you chatted during the unified period are still fully present in the `custom` drawer; after disabling, the official side simply won't see them.

**This restore checkbox is checked by default.** In other words, the default behavior is "restore the official sessions you migrated in back to the official history at the same time you disable." You only need to keep it checked and click "Turn off."

If the checkbox **doesn't appear**, the system has determined there's no backup that needs restoring (either you never checked migration, or no backup was found)—in that case your existing official sessions were never touched, and turning off the switch returns them to the `openai` drawer on their own.

### Step 2: How restore runs (precise flip-back per the backup ledger)

If you keep the box checked and click "Turn off," FyAgent's restore flow goes like this:

```text
① First copy the current state once more into a separate restore-backup directory
   ~/.fyagent/backups/codex-official-history-unify-restore-v1/<timestamp>/
   (restore itself backs up first, so restore won't lose data either)
② Comb through all migration backup generations, find the session ids "whose tag was originally openai," and assemble a "ledger"
③ Only for sessions that are [both in the ledger AND currently still custom], change the tag back to "openai"
```

Note the **dual condition** in step ③—it must be in the ledger (proving it really was migrated from the official side) AND currently still `custom` (showing you haven't manually changed it). Only when both conditions hold does it get flipped back. This guarantees the restore is both precise and free of collateral damage.

**At this moment your data is**: the migrated-back official sessions have their tag changed back to `openai` and reappear in the official list; meanwhile both the migration backup and the restore backup copies are still on disk.

### Step 3: Read the toast, confirm the result

Only the "disable + check restore" path pops a result toast. The toasts you may see (verbatim):

| Toast you see | Meaning |
|---|---|
| **Official session history restored from backup ({{files}} session files, {{rows}} index rows)** | Restore succeeded. `{{files}}` / `{{rows}}` show the actual numbers |
| **No restorable migration backup for the current Codex directory** | Nothing to restore (**does not mean data is lost**, see scenario E in the reference table) |
| **Unified session history was re-enabled; restore skipped** | You turned the switch back on while restore was queued, so the system deliberately abandoned the restore (see scenario F) |
| **Failed to restore official session history, please try again** | The restore process errored; just retry, the data is not corrupted |
| **Save failed, please try again** | The disabled state was not saved, so restore does not run and the switch returns to its previous position |

> **If saving the disabled state fails:** FyAgent does not run restore and returns the switch to its previous state, avoiding a mismatch between configuration and session classification.

---

## Sessions are missing from the list: symptom table

The table below maps common symptoms to likely causes and actions. Check the active provider and classification first; use the read-only checks when you need to inspect files.

| Scenario | What you see | Likely cause | Action |
|---|---|---|---|
| **A** Didn't check migration | Old official sessions not in the unified list | Existing sessions still carry the `openai` tag | Re-enable and check migration, or turn off the switch |
| **B** Cross-provider resume fails | Can't resume / errors out | Files remain in place; the current backend cannot decrypt the ciphertext | Resume on the original provider; to only read content, read the jsonl directly |
| **C** Proxy takeover / injection refused | No migration and no restore | Migration was skipped; session files were not rewritten | Exit takeover -> restart and retry; or just turn off the switch |
| **D** New sessions didn't return to official after restore | New sessions from the unified period aren't on the official side | They're in the `custom` drawer, untouched by design | Switch to a third-party provider to see them |
| **E** Toast "no restorable backup" | Restore "failed" | Usually nothing was ever migrated, sessions are in the original drawer | Turn off the switch and the official sessions reappear automatically |
| **F** Toast "switch was re-enabled, restore skipped" | Restore refused | This restore did not run, avoiding a classification conflict | Fully turn off the switch first, then restore |

### Scenario A: You enabled the switch but didn't check migration -> old official sessions "disappear"

**Symptom**: you turned on the unified switch, but didn't check "Also migrate existing official session history" in the enable dialog (it's unchecked by default). After enabling, your earlier official sessions seem to be gone from the list.

**Cause**: The existing sessions were not migrated. The switch only takes effect on official sessions "created after enabling"; your official sessions from **before** enabling still carry the `openai` tag and sit untouched in `~/.codex/sessions/`. You're now on the `custom` drawer, so naturally you can't see the old sessions left in the `openai` drawer—that's the entire reason for the "apparent disappearance."

**What to do** (pick either):
1. **Re-enable the switch and check "Also migrate existing official session history,"** which moves the old sessions to the `custom` drawer and they immediately appear in the unified list (automatic backup before the rewrite).
2. **Or simply turn off the unified switch**, the official side runs on the `openai` drawer again, and the old sessions reappear right where they were.

### Scenario B: Cross-provider resume of an old session fails -> you think "this session is broken / gone"

**Symptom**: after unification, the list shows an old session chatted with "another provider." You switch to your current provider and click "Resume," but it errors out or can't connect.

**Cause**: the session file remains in place; the failure occurs during cross-backend decryption. A Codex session stores an encrypted block of reasoning content `encrypted_content`, and **this ciphertext can only be decrypted by the backend that originally generated it**. Using provider B to resume a session generated by provider A means B can't decrypt A's ciphertext -> resume fails. This is **a design limitation of upstream Codex (by design)** and has nothing to do with whether FyAgent touched the file. You can inspect the readable content in the `.jsonl` file.

> This error affects generation of a new turn. Retry with the provider that created the session and confirm that the original `.jsonl` file is still present.

**What to do**:
- **Resume with "the provider that originally created this session,"** so it can decrypt normally and connect.
- Just want to read the history without continuing? Read that session's `.jsonl` file directly (commands at the end).
- Rule of thumb: **cross-provider is better suited to "starting a new session"; resume old sessions on their original provider whenever possible.**

### Scenario C: You enabled the switch and checked migration, but migration was silently skipped -> you think "migration lost the sessions"

**Symptom**: you enabled the switch and checked migration, but the old official sessions neither entered the unified list nor could be restored when you turned the switch off (or the restore checkbox didn't even appear in the disable dialog, see scenario E). You suspect migration lost the sessions during the process.

**Cause**: migration did not run, so the session files were not rewritten. Before migration, FyAgent checks whether Codex's live config (`~/.codex/config.toml`) is routed to the shared `custom` classification, and migrates only after that condition is met. In the following two situations FyAgent treats unification as incomplete, skips migration, and keeps the migration request for a later retry:

- **During proxy takeover**: FyAgent's proxy has taken over the live config, and the live config during takeover doesn't carry the unified routing marker.
- **Injection refused**: your `config.toml` already has a manually specified `model_provider`, or there's already a differently-shaped `[model_providers.custom]` table (possibly with a third-party address). To avoid incorrectly routing official traffic to a third-party backend, FyAgent would rather not inject and not migrate.

Skipping migration does not rewrite session files. Resolve the conflict and restart FyAgent, or disable the feature.

**What to do**:
- Exit proxy takeover -> **restart FyAgent**: on startup it automatically retries migration (your migration intent is preserved the whole time).
- Check `~/.codex/config.toml`: if there's a conflicting route you wrote by hand, clean up the conflict before enabling the switch.
- If you no longer need this feature, turn off the switch; official sessions that were not migrated still display under the `openai` classification.

### Scenario D: You turned off the switch and restored, but "the new sessions chatted during the unified period" didn't return to official -> you think "the new sessions are gone"

**Symptom**: during the unified period, you chatted a few more new sessions with the official account. Later you turned off the switch, checked restore, and after restoring you find those new sessions didn't return to the official drawer.

**Cause**: sessions created while the switch was enabled remain in the `custom` classification. Restore is based on "the backup ledger from migration time"—**only sessions that were originally migrated in from the `openai` drawer** are recorded in the backup and get precisely flipped back to `openai`. The sessions you **created during the unified period** are in no backup ledger; and after unification both official and third-party use the `custom` tag, so **FyAgent can't tell whether a new session was chatted with the official account or a third-party**. To avoid wrongly stuffing third-party sessions into the official history, the product decision is: these new sessions all stay in the `custom` (third-party) history and are not moved automatically. The disable dialog's text says this explicitly too—"Sessions created while it was on cannot be attributed to a provider, so they stay in the third-party history."

**What to do**:
- Switch to any third-party provider (the `custom` drawer) to see these sessions in the history list.
- To read content, read the `.jsonl` directly; to resume, follow scenario B's rule (go back to the backend that originally generated it).
- If you really want to manually return **one specific** session to official: there's currently no automatic button (deliberately omitted, to avoid misjudging the direction). Advanced users can, **after backing up** that file first, manually change `model_provider` in the `session_meta` of the first line of its `.jsonl` from `custom` back to `openai` (an advanced operation; always make a copy before editing).

### Scenario E: Restore toast "No restorable migration backup for the current Codex directory" -> you think "restore failed = data is gone"

**Symptom**: you checked restore when turning off the switch, and got the toast "No restorable migration backup for the current Codex directory." This means the current directory has no migration record that can be used for this restore.

**Cause**: Usually, the current directory has no migration that needs to be restored. Common reasons:

- **You never checked "migrate existing official sessions" in the first place**: with no migration, there's naturally no migration backup and no sessions to flip back. Your old official sessions have been in the `openai` drawer all along and reappear after you turn off the switch (same as scenario A). (In this case, the disable dialog may **not even show the restore checkbox**—because the system can't find any backup.)
- **You've already restored once**: the session tags have all been flipped back to `openai`, so clicking again naturally finds "no targets still in custom to restore"—this is **idempotent protection, not failure**.
- **You switched Codex directories**: restore only recognizes the backup ledger belonging to the **current** directory; switch directories and it can't find the old directory's ledger. Just switch the directory back.

None of these three paths runs the session-deletion operation.

**What to do**: use the commands at the end of the guide to record the total number and recent modification times of session files in `~/.codex/sessions/`, then check whether `~/.fyagent/backups/` contains a `codex-official-history-unify-v1` directory. If it is absent, the current Codex directory usually has no generated migration backup.

### Scenario F: Restore refused, toast "Unified session history was re-enabled; restore skipped"

**Symptom**: you turned off the switch -> checked restore -> but you were quick and immediately turned the switch back on, then saw the toast "Unified session history was re-enabled; restore skipped."

**Cause**: restore changes session classification from `custom` to `openai`. If the switch has already been re-enabled, live routes to `custom` again, so continuing restore would make the configuration and classification disagree. FyAgent therefore skips this restore and leaves the session classifications unchanged.

**What to do**: to restore, turn the switch off and wait for the setting to save before selecting restore. To keep the unified list, do not run restore.

**Troubleshooting order: check the active provider and session classification, then inspect session files, the index, and backups. Make a separate backup before manual edits.**

---

## Check session files and backups

The following checks read session files, the index, and backup directories without modifying them. Record the total file count and recent modification times before checking classifications.

### The simplest way: open it directly in a file manager (no command line at all)

- **macOS (Finder)**: press `Cmd + Shift + G`, paste `~/.codex/sessions` and hit Enter to see a pile of `.jsonl` session files and their modification times; for the backup directory paste `~/.fyagent/backups`.
- **Windows (File Explorer)**: paste `%USERPROFILE%\.codex\sessions` into the address bar and hit Enter to see the session folders and the `.jsonl` files inside; for the backup directory paste `%USERPROFILE%\.fyagent\backups`.

**As long as you can see a batch of `.jsonl` files here, that proves your session data is intact on disk.** The file count and modification times are more intuitive than any amount of text.

### Where exactly your session / history files live

| Content | Real path | Notes |
|---|---|---|
| **Session body (the core)** | `~/.codex/sessions/` (includes date-based subdirectories, recursive) | One `.jsonl` text file per session—**this is your conversation content** |
| **Archived sessions** | `~/.codex/archived_sessions/` | Also `.jsonl` |
| **Session index database** | `~/.codex/state_5.sqlite` | The `model_provider` column of the `threads` table is the "drawer tag"—**this is the actual classification source the resume list reads** |
| **Migration backup** (auto-created when migration is enabled) | `~/.fyagent/backups/codex-official-history-unify-v1/<timestamp>/` | Contains `jsonl/`, `state/`, `meta.json` |
| **Restore backup** (auto-created when you restore) | `~/.fyagent/backups/codex-official-history-unify-restore-v1/<timestamp>/` | A safety copy taken before restore |

> **Note**: if you've changed the Codex directory in FyAgent, or set `sqlite_home` in `config.toml`, replace `~/.codex` above with your actual directory. Below, `~` = your user home directory.

### macOS commands

**1. Record the total number and recent modification times of session files**

```bash
# Count session files and compare the result with a pre-change record or backup
find ~/.codex/sessions ~/.codex/archived_sessions -name '*.jsonl' 2>/dev/null | wc -l

# Show the 10 most recently modified session files
find ~/.codex/sessions -name '*.jsonl' 2>/dev/null -print0 \
  | xargs -0 ls -lt 2>/dev/null | head -10
```

**2. (Auxiliary) See how many sessions are in each "drawer"**

```bash
# Number of session files in the official drawer (openai)
grep -rlE '"model_provider"[[:space:]]*:[[:space:]]*"openai"' ~/.codex/sessions 2>/dev/null | wc -l

# Number of session files in the unified drawer (custom)
grep -rlE '"model_provider"[[:space:]]*:[[:space:]]*"custom"' ~/.codex/sessions 2>/dev/null | wc -l

# See the tag distribution at a glance
grep -rhoE '"model_provider"[[:space:]]*:[[:space:]]*"[^"]*"' ~/.codex/sessions 2>/dev/null | sort | uniq -c
```

> **Counting note:** Early Codex sessions may not include `model_provider` in their `.jsonl` file, so a tag-based grep can return fewer entries than the total file count. Record the total in step 1, then use `state_5.sqlite` to inspect the classification of older sessions.

**3. (Advanced) Query the index database `state_5.sqlite`—the classification the resume list actually reads**

```bash
# Requires sqlite3 to be installed; skip if you don't have it
sqlite3 ~/.codex/state_5.sqlite \
  "SELECT COALESCE(model_provider,'<empty>'), COUNT(*) FROM threads GROUP BY 1;"
```

> This `threads` table is the actual classification source Codex's resume list reads; the `openai` row count ≈ the number of sessions you can see in your official drawer. It may not match step 2's jsonl grep—the reason is exactly what's described above: "old sessions don't write the jsonl field, but they're still openai in the index database." A mismatch between the two is not an anomaly.

**4. Read the content of a specific session directly (confirm the conversation text is still there)**

```bash
# Replace <filename> with one of the .jsonl paths listed by ls above
python3 -m json.tool < "<filename>.jsonl" 2>/dev/null | head -50

# Or just open it in an editor (plain text)
open -e "<filename>.jsonl"      # macOS
```

**5. Look at FyAgent's backup directory (proof that a copy was kept before migration / restore)**

```bash
ls -la ~/.fyagent/backups/codex-official-history-unify-v1/ 2>/dev/null
ls -la ~/.fyagent/backups/codex-official-history-unify-restore-v1/ 2>/dev/null
```

### Windows commands (PowerShell)

The session directory is usually at `C:\Users\<your username>\.codex\`, and backups at `C:\Users\<your username>\.fyagent\backups\`.

```powershell
# 1. Total session files, for comparison with a pre-change record or backup
(Get-ChildItem "$env:USERPROFILE\.codex\sessions","$env:USERPROFILE\.codex\archived_sessions" -Recurse -Filter *.jsonl -ErrorAction SilentlyContinue).Count

# 2. The 10 most recently modified sessions
Get-ChildItem "$env:USERPROFILE\.codex\sessions" -Recurse -Filter *.jsonl |
  Sort-Object LastWriteTime -Descending | Select-Object -First 10 FullName,LastWriteTime

# 3. (Auxiliary) How many session files in the official (openai) / unified (custom) drawers
(Get-ChildItem "$env:USERPROFILE\.codex\sessions" -Recurse -Filter *.jsonl |
  Select-String -Pattern 'model_provider"\s*:\s*"openai"' -List).Count
(Get-ChildItem "$env:USERPROFILE\.codex\sessions" -Recurse -Filter *.jsonl |
  Select-String -Pattern 'model_provider"\s*:\s*"custom"' -List).Count

# 4. Look at the backup directories
Get-ChildItem "$env:USERPROFILE\.fyagent\backups\codex-official-history-unify-v1" -ErrorAction SilentlyContinue
Get-ChildItem "$env:USERPROFILE\.fyagent\backups\codex-official-history-unify-restore-v1" -ErrorAction SilentlyContinue
```

> Same reminder: the step-3 grep counting **fewer** than the total file count is normal (old sessions don't write that field); judge "nothing lost" by the **total file count** from step 1.

---

## Technical appendix

### 1. The bucketing mechanism (the essence of the drawers)

Codex's resume / history list filters by the currently active `model_provider` id with **exact string matching**. The **first line** of a session's `.jsonl` file is a `type:"session_meta"` record whose `payload.model_provider` is the drawer that session belongs to (`grep -rl` counts a file as long as the tag appears once anywhere in it, so no line-by-line parsing is needed; sessions from old versions that didn't write the field can't be counted). What actually drives the resume list is the `threads.model_provider` column of the index database `state_5.sqlite`. When `config.toml` has no explicit `model_provider`, the official subscription falls into the built-in default id `openai`; all of FyAgent's third-party providers uniformly use `custom`.

### 2. What the switch does (injection, lives only in live)

When enabled, FyAgent injects the following into the official live `config.toml`:

```toml
model_provider = "custom"

[model_providers.custom]
name = "OpenAI"
requires_openai_auth = true
supports_websockets = true
wire_api = "responses"
```

Every field has a purpose: `requires_openai_auth = true` keeps authentication going through the ChatGPT login in `auth.json`, with the base_url defaulting back to the official Codex backend; `name = "OpenAI"` lets Codex's official feature gates (web search, remote compaction, etc.) keep matching; `supports_websockets = true` restores the capability that custom entries lose by default; `wire_api = "responses"` uses the official responses protocol. **The net effect is: authentication is unchanged, only the bucket name changed.**

**Storage boundary: this injection is written to the live `config.toml`, not the provider configuration stored in the database.** When switching away from the official provider and writing live back, FyAgent removes the section only when its shape matches the injected artifact; a third-party-customized `custom` table remains. After disabling the switch and completing one provider switch, live is regenerated from the stored configuration.

### 3. The two refusal gates for injection (corresponding to scenario C)

- `config.toml` already has an explicit `model_provider` -> don't override the user's route;
- A differently-shaped `[model_providers.custom]` table already exists (possibly with a third-party `base_url`) -> refuse injection, otherwise ChatGPT OAuth traffic would be routed to the wrong backend.

When injection is refused, live is not unified, and the migration gate (checking whether live's `model_provider` equals `custom` after trim) judges `live_not_unified` -> skip migration, preserve intent, and do it later on the next startup retry. This is "safe deferral," not "failure with data loss."

### 4. The three session classes (which determine the migration / restore boundary)

- **Class A**: existing official sessions migrated in at enable time—the backup is the ledger, and they can be precisely restored back to `openai`;
- **Class B**: created during the unified period—not recorded in a migration backup and not distinguishable as official or third-party, so FyAgent does not move them automatically (they stay `custom`);
- **Class C**: third-party history from before enabling—outside the migration and restore scope.

### 5. Migration and restore protections

The implementation uses the following protections to reduce the risk of failed writes, concurrent changes, and incorrect restore targets:

- **Change the classification field, not conversation entries**: migration and restore switch `model_provider` between `openai` and `custom`; their implementation does not rewrite `response_item` entries or conversation text.
- **Copy a backup before writing**: jsonl uses file copy and the state DB uses a SQLite copy, both stored in a timestamped generation directory. Migration backups live in `codex-official-history-unify-v1/`, restore backups in the separate `codex-official-history-unify-restore-v1/`—the two are kept apart to keep the ledger clean.
- **No session deletion in migration/restore + whole-file replacement**: jsonl rewrites use a temporary file followed by whole-file replacement, and the state DB uses a transactional `UPDATE`; these paths do not call session or index deletion.
- **Pessimistic skip + idempotent and retryable**: when buckets are inconsistent (`live_not_unified`), it would rather not migrate; a single process lock serializes migration and restore to avoid "startup retry / post-save background task / disable-time restore" concurrently rewriting the same batch of files in both directions; the completion marker is bound to the Codex directory and written conditionally to prevent missed migrations; restore uses the "in the ledger + currently still custom" dual condition to prevent wrong changes. Restore scans the union of all backup generations, so even after many switch cycles it can still restore early-migrated sessions; a repeated restore returns `nothing_to_restore`, which is idempotent protection rather than failure.

### 6. Cross-backend encrypted_content (corresponding to scenario B)

When resuming across providers, the current backend may be unable to decrypt the session's `encrypted_content`, causing resume to fail. Switch back to the provider that created the session or start a new session, and use the read-only checks above to inspect the original `.jsonl` file.

---

## References

- [Keep Codex Remote Control and Official Plugins While Using Third-Party APIs: FyAgent Setup Guide](./codex-official-auth-preservation-guide-en.md)
- [Using DeepSeek-Style Chat APIs in Codex: FyAgent Local Routing Guide](./codex-deepseek-routing-guide-en.md)
- The "Codex App Enhancements" section in the FyAgent user manual

---

**Troubleshooting summary:** If sessions are missing from the list, switch back to the provider that created them, then inspect `~/.codex/sessions/`, `state_5.sqlite`, and `~/.fyagent/backups/`. To undo migration of existing official sessions, disable the switch and restore from the recorded backup.
