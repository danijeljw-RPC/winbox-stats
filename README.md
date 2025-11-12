# winbox-stats

Capture simple host stats to per-metric monthly SQLite files. Export to JSON and render PNG graphs.

## Build

```
cargo build --release
```

## Use

- No args: capture one sample into `YYYY-MM@HOST@{CPU|RAM|X_Drive}.sqlite` in the current directory.
```
winbox-stats.exe
```

- Graph mode: read all `.sqlite` files under the current directory, export each to `.json`, and plot `.png`.
```
winbox-stats.exe graph
```

- Help and version come from `clap`:
```
winbox-stats.exe --help
winbox-stats.exe --version
```
