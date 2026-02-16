# bptui

**THIS IS A WORK IN PROGRESS: NOT READY FOR USE**

`bptui` is a lightweight terminal user interface prototype for editing
`boilerplate` R package data (starting with measures).

## Quick start

```bash
cargo run
```

The `cargo run` command opens the interactive TUI. Use your keyboard inside that screen for commands.

## Backup dataset included in this repo

For quick testing, we kept a copy of the measures database here:

- `/Users/joseph/GIT/bptui/boilerplate_unified.json`

## What this scaffold includes

- Ratatui application scaffold
- measure list navigation
- inline field editing via `/edit` (name, description, reference, waves, keywords, standardised, standardised_date, items)
- status bar with dirty state
- margo-inspired theme system (catppuccin palette, panel layout, playful header)

## Keys (scaffold)

- `j` / `k`: move selection up/down
- `n` or `a`: add a measure path entry
- `d`: remove selected measure
- `Enter` or `e`: open the field-edit menu for selected measure
- `/`: start command mode (type a slash command)
- `s` or `/save`: create a checkpoint snapshot
- `r`: reload stub
- `?`: toggle help overlay
- `q` or `Esc`: quit

## Command mode

- `/help` show help overlay
- `/save` create a checkpoint snapshot
- `/add [path]` add placeholder measure or provided path
- `/reload` placeholder reload status
- `/find <query>` select next matching measure
- `/edit <field> <value>` edit selected measure field
- `/edit name <value>` replace measure path/name
- `/edit description <value>`
- `/edit reference <value>`
- `/edit waves <value>`
- `/edit keywords <value>`
- `/edit standardised <true|false>`
- `/edit standardised_date <value>`
- `/edit items <comma or semicolon separated list>`
- `/history` list recent checkpoint history
- `/restore <n>` restore a checkpoint by index shown by `/history`
- `/revert` / `/undo` revert to the previous checkpoint (append-only; creates a new checkpoint)
- `/e` open edit menu for selected measure
- `In menu mode:` `j/k` move, `Enter` edit selected field, `Space` toggle `standardised`.
- `/load` open in-app JSON file picker
- `/load <file>` replace measure list from file (line list, JSON array, or boilerplate map)
- `/clear` clear status message
- `/q` quit
- `/quit` (or `/q`) quit
- `Tab` autocomplete command

Example (inside the running bptui screen):

- Press `/`
- Type `/load` and hit Enter to open the file picker, then start typing `boilerplate_unified` and press Enter

If you already know the full file path, you can still use:

- `/load /Users/joseph/GIT/bptui/boilerplate_unified.json`

### How to set the path to your measures file

By default, start with `/load` to open the in-app file picker.

If you prefer manual entry, absolute paths still work.

In command mode, type:

- `/load <absolute-path>`

Working example using the included backup:

- `/load /Users/joseph/GIT/bptui/boilerplate_unified.json`

To locate the exact path from the terminal:

- `cd /Users/joseph/GIT/templates/boilerplate/data/2005-aug-1-backups`
- `pwd`
- `realpath boilerplate_unified.json`

If you are browsing with yazi, open that file and copy its full path, then paste after `/load`.

Tip: inside the TUI, press `Enter` (or `e`) on a selected measure to open the in-screen edit menu showing all fields.

Notes:

- Each `/save` writes a checkpoint into an in-memory history chain.
- `restore` and `revert` are append-only. They create a new checkpoint entry when used so your
  most recent state is never overwritten.
- Backup examples remain available for quick start; history is for interactive iteration, not file archival.

- If your path contains spaces, quote it:  
  `/load "/path/with spaces/file.json"`
- Success response in the status bar will look like: `Loaded 180 measures from <path>`

## Command autocomplete

While typing a command, press `Tab` to cycle through matching command completions.

Examples:

- `/f` + `Tab` → `/find `
- `/fi` + `Tab` → `/find `
- `/add cl` + `Tab` → `/add clinical.` (next completion if available)

## Notes

- This is an initial scaffold; no R package integration is wired yet.
- Next iteration should replace stubs with:
  - read/write to `boilerplate` measure JSON
  - structured edit form
  - validation + diff preview before save
