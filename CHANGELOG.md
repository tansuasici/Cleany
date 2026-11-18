# Changelog

All notable changes to Cleany are listed here. Each release on the [Releases](https://github.com/tansuasici/Cleany/releases) page uses the notes for its version from this file.

## [Unreleased]

### Added
- Cleany checks for a newer version when it opens. When one is available, a bar offers to install it and restart; nothing is downloaded until you choose to. The download is checked against Cleany's signing key before it is installed.
- On Linux, the AppImage updates itself. When Cleany cannot update an install in place, the bar says where to get the new version.
- Copies of 0.1.0 do not include this yet, so update to the next version by hand once.

### Changed
- The icons in the app now come from one consistent set, so they share the same size and line weight.

### Other
- The macOS disk image is notarized as well, so macOS checks it without a warning before you open it.

## [0.1.0]

The first public release of Cleany for macOS, Windows and Linux.

### See where your space went
- Scan a drive or a folder and see how much space every folder takes, largest first.
- Explore the results in a table sorted by size, in a map drawn as nested boxes, or in rings.
- Open a folder with a click and move back and forward through your history.
- See the largest files and folders under a folder in a single list, however deep they sit.
- On macOS, the view updates when a file is added or removed, so you don't need to scan again.

### Clean up safely
- Collect what you want to remove in the selection basket and see how much space it will free.
- Items go to the Trash first, so you can restore them.
- Folders that belong to the operating system are protected and cannot be removed.
- Every removal is written to a log file on your computer.
- Permanent deletion is optional. It is turned on in Settings and asks for confirmation each time.

### Accurate numbers
- Hard links are counted once.
- Cloud files that are only placeholders are not added to the total.
- On macOS, system folders that appear in two places are counted once.

### Other
- Export a scan or a single folder as CSV or JSON, with progress and the option to stop.
- Reveal an item in Finder or your file manager, open it, copy its path, or open a terminal in that folder.
- Cleany can keep running in the menu bar when you close the window.
- Cleany can open at login. On macOS 13 or later, System Settings lists it under the name Cleany.
- On macOS, Settings also opens from the app menu or with Cmd+,.
- On macOS, Cleany guides you to grant Full Disk Access on first launch.
- The interface is available in English and Turkish.

### Downloads
- macOS 11 or later, Apple Silicon and Intel: `.dmg`
- Windows 10 and 11: `.exe` installer or `.msi`. The installer is not signed yet.
- Linux: `.AppImage`, `.deb` or `.rpm`

[0.1.0]: https://github.com/tansuasici/Cleany/releases/tag/v0.1.0
