# SkipUAC

A minimal Windows utility that creates **(elevated)** desktop shortcuts for *trusted* apps to reduce repeated UAC prompts.

中文说明：见 `README.md`.

## How it works (high level)

- SkipUAC does **not** disable UAC and does not change Windows security policies.
- After you drop a `.lnk` or `.exe`, SkipUAC records the target path (and arguments if present).
- When you click **Create**, SkipUAC creates a Windows **Task Scheduler** task configured to **Run with highest privileges**, and then generates a desktop **(elevated)** shortcut.
- Double-clicking that **(elevated)** shortcut triggers the scheduled task to start the target app, reducing how often you need to confirm UAC prompts.

## Quick start

1. Drag and drop a shortcut (`.lnk`) or executable (`.exe`) into the app window, or click **Add** to pick a file.
2. Select one or more items in the list, then click **Create** to generate desktop **(elevated)** shortcuts.
3. Use the generated desktop **(elevated)** shortcuts to launch your apps.
4. Remove: right-click an item and choose **Delete** (or use the current delete action). This removes both the desktop shortcut and the corresponding scheduled task.

## Security notes (please read)

- Do **not** set UAC to “Never notify” (effectively disabling UAC). It significantly weakens Windows security boundaries.
- Only use SkipUAC for apps you **fully trust**. Do not add unknown or unverified executables.
- Any “reduced-friction elevation” mechanism can be abused. If a target executable (or its directory) is replaced/injected, it may run malicious code with elevated privileges.

## Development

### Prerequisites

- Windows 10/11
- Node.js (recent LTS recommended)
- Rust toolchain (required by Tauri)

### Dev

```bash
npm install
npm run tauri dev
```

### Build

```bash
npm run tauri build
```

## License

MIT License. See `LICENSE`.

## Disclaimer

- This project is provided “as is”, without warranty of any kind. Use at your own risk.
- It may create/remove scheduled tasks and desktop shortcuts. Please use it only if you understand the mechanism and risks.

## Contact

- Email: `snoe8090@gmail.com`
