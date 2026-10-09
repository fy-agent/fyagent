# Memory

Open 「记忆模块」 and choose 「长期记忆」 (Long-term memory) or 「每日记忆」 (Daily memory).

## Long-term memory

Select OpenClaw or Hermes `MEMORY.md` / `USER.md`, edit the body and click 「保存」. A file marked 「尚未创建」 is created when saved.

OpenClaw files are in its workspace; 「打开 OpenClaw 工作区」 opens that directory. Hermes long-term files are in `memories/`; the page shows enable switches and character limits. Content over the limit can be saved, but Hermes may not use all of it, so shorten it when possible.

## Daily memory

Daily records are OpenClaw workspace files at `memory/YYYY-MM-DD.md`.

- 「创建或打开今天」 opens today's record; edit and save to create the file.
- Search daily records, select a result and edit its content.
- 「打开记忆目录」 opens the directory in your file manager.
- Check the date before confirming deletion; the local file is deleted.

Switching files, memory types or pages with unsaved edits requires confirmation. Save first to retain your content. A refresh failure can leave previous content visible; retry reading before continuing to edit.

## Manual backfill and recovery limits

For an older Daily date, use 「打开记忆目录」 to find and manually edit the matching `YYYY-MM-DD.md`, then reread in FyAgent. There is no automatic history reconstruction or one-click backfill. Retain any history you need before editing.

When FyAgent actually changes an existing file, the adjacent `<filename>.fyagent.backup` holds the most recent pre-write content. The next different write replaces this generation; saving identical content does not refresh it. A newly created file has no previous body to recover. Daily has no one-click restore button: copy the current file and backup, compare them, then manually backfill and reread. This is not a database or full-history backup.

[Back to manual](README.md)
