# pmux - Package Manager Multiplexer

A fast, lean, and modular terminal-based universal package manager interface written in Rust. Browse, search, and install packages from multiple package managers through a unified TUI interface.

## Features

- **Universal Interface**: Support for multiple package managers (Nix, AUR/paru, APT, DNF, Pacman, Emerge)
- **Enhanced 5-Unit TUI Layout**: 
  - Results unit with package listing and multi-selection (● indicators)
  - Centered search field with selection counter `[3] (2/15) >> query`
  - Package details unit with async AUR integration
  - Installed packages list (dynamically updated)
  - Terminal unit for installation queue and commands
- **Multi-Selection System**: Ctrl+Space to select/deselect packages, Ctrl+C to clear all
- **Improved Navigation**: Direct arrow key navigation in results, optimized tab switching
- **AUR Integration**: Full Arch User Repository support with async search and details
- **Bedrock Linux Support**: Automatic detection of all strata package managers
- **Smart Caching**: Fast package database caching with configurable refresh intervals
- **Async Architecture**: Non-blocking operations with tokio runtime
- **Modular Architecture**: Easy to add new package managers
- **Full Color Support**: Customizable themes and color schemes
- **Root/Sudo Handling**: Automatic privilege escalation when needed
- **Cross-Platform**: Works on all major Linux distributions

## Usage

```bash
# Launch TUI with no initial query
pmux

# Launch TUI with initial search query
pmux firefox

# Single-shot mode (exit after installation)
pmux -S package-name
```

## Configuration

Configuration files are stored in `~/.config/pmux/`:

```
~/.config/pmux/
├── config.toml          # Main configuration
├── pkgmanagers/         # Package manager configurations
│   ├── nix.toml
│   ├── paru.toml
│   ├── apt.toml
│   └── ...
└── themes/
    └── colors.toml      # Color schemes
```

## TUI Interface

The interface consists of 5 main units in an optimized layout:

1. **Results Unit** (Top): Scrollable list of packages with multi-selection indicators (●)
2. **Search Field** (Center): Dynamic search with selection counter `[3] (2/15) >> query`
3. **Details Unit** (Bottom): Package information with async AUR details
4. **Installed List** (Right Top): Dynamic list of installed packages
5. **Terminal Unit** (Right Bottom): Installation queue and live command output

### Navigation & Controls

- **Tab**: Switch between panes (Results → Search → Details → Installed → Terminal)
- **Arrow Keys**: Navigate within focused pane (Results or Installed lists)
- **'/' or 'i'**: Enter search mode (focus search field)
- **Ctrl+Space**: Toggle package selection (multi-select) - only in Results pane
- **Enter**: Install selected packages
- **Ctrl+C**: Clear all selections
- **Esc**: Exit search mode or quit application
- **q**: Quit application

## Package Manager Support

- ✅ **Pacman** (Arch Linux) - Full support with package details
- ✅ **AUR** (paru/yay) - Complete integration with async search and details
- ✅ **DNF** (Fedora/RHEL) - Package listing and installation
- ✅ **Emerge** (Gentoo/Portage) - Portage tree support
- ✅ **Nix** (NixOS/nix-env) - Nix package manager support
- ✅ **APT** (Debian/Ubuntu) - Full APT integration
- ✅ **Bedrock Linux** - Automatic detection of all strata package managers
- 🚧 Zypper (openSUSE)
- 🚧 Flatpak
- 🚧 Snap

### AUR Integration Features
- Real-time search with AUR RPC v5 API
- Package details with dependencies, votes, and popularity
- Async operations for smooth UI experience
- Automatic detection on Arch Linux systems

## Building

```bash
cargo build --release
```

## Design Philosophy

- **Lean**: Minimal dependencies, maximum performance
- **Clean**: No unnecessary code, every line serves a purpose
- **Modular**: Easy to extend and customize
- **Universal**: Works across all Linux distributions
- **Fast**: Efficient caching and minimal overhead

## License

MIT
