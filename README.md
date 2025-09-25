# pmux - Package Manager Multiplexer

A fast, lean, and modular terminal-based universal package manager interface written in Rust. Browse, search, and install packages from multiple package managers through a unified TUI interface.

## Features

- **Universal Interface**: Support for multiple package managers (Nix, AUR/paru/yay, APT, DNF, etc.)
- **5-Unit TUI Layout**: 
  - Results unit with package listing
  - Input field with dynamic query and selection counter
  - Description unit showing package details
  - Installed packages list (dynamically updated)
  - Terminal unit for installation commands
- **Smart Caching**: Fast package database caching with configurable refresh intervals
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

The interface consists of 5 main units:

1. **Results Unit**: Scrollable list of packages with source indicators
2. **Input Field**: Search query with selection counter (e.g., "(1/366) >> █")
3. **Description Unit**: Package information and details
4. **Installed List**: Dynamic list of installed packages
5. **Terminal Unit**: Live installation output and command execution

## Package Manager Support

- ✅ Nix (nix profile)
- ✅ AUR (paru, yay)
- ✅ APT (Debian/Ubuntu)
- 🚧 DNF (Fedora/RHEL)
- 🚧 Pacman (Arch)
- 🚧 Zypper (openSUSE)
- 🚧 Portage (Gentoo)
- 🚧 Flatpak
- 🚧 Snap

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
