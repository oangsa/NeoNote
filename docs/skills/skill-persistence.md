# skill-persistence

NeoNote stores user-facing configuration exclusively under `%APPDATA%\NeoNote\`.

Use `AppDataPaths` as the single source of truth for:

- `config.json`
- `session.json`
- `recent_files.json`
- `themes/built-in/`
- `themes/user/`

Do not write user configuration next to the executable, inside the repo, or in ad hoc temp locations. If `%APPDATA%` is unavailable during development, fall back only to a local development path so the app can still start.
