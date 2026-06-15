# skill-windows-packaging

NeoNote's Windows packaging conventions:

- The crate version in `Cargo.toml` is the release version source of truth.
- `build.rs` should derive Windows resource metadata and manifest version from `CARGO_PKG_VERSION` instead of hardcoding a separate value.
- The installed executable should be named `NeoNote.exe` to stay aligned with the file-association configuration in `AppController`.
- Built-in themes are a runtime asset and must be shipped alongside the executable under `assets\themes\built-in\`.
- Installer output should go under `target\installer\` so packaging artifacts stay out of source control via the existing `.gitignore`.
- The current NSIS entrypoint is `installer/neonote.nsi` and the local wrapper for building release installers is `scripts/build-installer.ps1`.
- The installer is per-user and installs into `$LOCALAPPDATA\Programs\NeoNote` to match the user-level configuration and file-association model already used by the app.
- Uninstall should remove installed binaries and shortcuts, but leave `%APPDATA%\NeoNote\` user data intact unless there is an explicit future requirement to purge user data.
