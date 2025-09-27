# Package Manager Implementation Plan for PMUX

## Current Status Analysis
Based on your Bedrock Linux setup, we have:
- **Fedora RPM**: 3221 packages (stratum: fedora) 
- **Arch Pacman**: 574 installed, 14802 available (stratum: tut-arch)
- **Gentoo Portage**: 286 installed, 19125 available (stratum: gentoo)  
- **Nix**: 24839 store paths (global)
- **AUR (paru)**: Available but not detected

## Critical Issues to Fix

### 1. UI Rendering Bug
**Problem**: Scrolling down only refreshes package name field, scrolling up refreshes full line
**Solution**: Fix the list rendering to always refresh complete lines consistently

### 2. Missing Package Manager Detection
**Problem**: Only detecting 3 managers instead of 5 (missing paru/AUR and dnf)
**Solution**: Add proper Bedrock-aware detection for all stratum-specific locations

### 3. Incomplete Package Data
**Problem**: Missing descriptions, versions, and proper metadata parsing
**Solution**: Implement proper database parsing for each package manager

## Implementation Plan

### Phase 1: Fix Bedrock Linux Detection

#### Update `detect_package_managers_with_config()` in `src/core/local.rs`:

```rust
// Add these detections:
// 1. DNF/RPM (Fedora stratum)
if Path::new("/bedrock/strata/fedora/var/lib/rpm/Packages").exists() {
    managers.push(LocalPackageManager::new("dnf".to_string(), Some("fedora".to_string())));
}

// 2. Paru/AUR cache (tut-arch stratum) 
if Path::new("/bedrock/strata/tut-arch/home/chris/.cache/paru/packages.aur").exists() {
    managers.push(LocalPackageManager::new("paru".to_string(), Some("tut-arch".to_string())));
}
```

### Phase 2: Implement Proper Database Parsing

#### A. Pacman Database Parser
**Location**: `/bedrock/strata/tut-arch/var/lib/pacman/sync/*.db`
**Format**: Compressed tar archives containing package metadata

```rust
fn parse_pacman_db(db_path: &str) -> Result<Vec<Package>, Error> {
    // Extract tar.xz archive
    // Parse each package's desc file for:
    // - %NAME%
    // - %VERSION% 
    // - %DESC%
    // - %REPO%
}
```

#### B. AUR/Paru Parser  
**Location**: `/bedrock/strata/tut-arch/home/chris/.cache/paru/packages.aur`
**Format**: Custom paru cache format

```rust
fn parse_paru_cache(cache_path: &str) -> Result<Vec<Package>, Error> {
    // Parse paru's binary cache format
    // Extract AUR package metadata
}
```

#### C. DNF/RPM Database Parser
**Location**: `/bedrock/strata/fedora/var/cache/dnf/*.solv`
**Format**: Libsolv binary format + RPM database

```rust
fn parse_dnf_solv(cache_dir: &str) -> Result<Vec<Package>, Error> {
    // Parse .solv files in DNF cache
    // Extract package names, versions, descriptions
    // Cross-reference with RPM database
}
```

#### D. Portage Parser Enhancement
**Location**: `/bedrock/strata/gentoo/var/db/repos/gentoo/`
**Format**: Directory structure with ebuild files

```rust
fn parse_portage_ebuilds(repo_path: &str) -> Result<Vec<Package>, Error> {
    // Parse category/package/package-version.ebuild files
    // Extract DESCRIPTION, HOMEPAGE, etc from ebuild headers
}
```

#### E. Nix Store Parser
**Location**: `/nix/var/nix/db/db.sqlite`
**Format**: SQLite database

```rust
fn parse_nix_sqlite(db_path: &str) -> Result<Vec<Package>, Error> {
    // Query ValidPaths table
    // Parse derivation files for metadata
    // Extract package names from store paths
}
```

### Phase 3: Fix UI Rendering

#### Update `draw_results_list()` in `src/ui/tui.rs`:

```rust
// Ensure consistent line rendering regardless of scroll direction
// Problem: Different rendering paths for up vs down scrolling
// Solution: Use single rendering function for all list items
```

### Phase 4: Data Structure Enhancements

#### Enhance `Package` struct in `src/core/package_managers.rs`:

```rust
pub struct Package {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,        // NEW
    pub repository: Option<String>,      // NEW (core, extra, AUR, etc)
    pub architecture: Option<String>,    // NEW
    pub installed_size: Option<u64>,     // NEW
    pub download_size: Option<u64>,      // NEW
    pub installed: bool,
    pub source: String,
}
```

## Specific Database Formats to Parse

### 1. Pacman Sync Databases
```bash
# Location: /bedrock/strata/tut-arch/var/lib/pacman/sync/
# Files: core.db, extra.db, community.db, multilib.db
# Format: tar.xz archives containing:
#   package-name-version/
#   ├── desc      # Package metadata
#   ├── depends   # Dependencies  
#   └── files     # File list
```

### 2. AUR/Paru Cache
```bash
# Location: /bedrock/strata/tut-arch/home/chris/.cache/paru/
# Files: packages.aur (binary cache)
# Alternative: Use AUR RPC API for real-time data
```

### 3. DNF Cache
```bash
# Location: /bedrock/strata/fedora/var/cache/dnf/
# Files: *.solv (libsolv binary format)
# Contains: Package names, versions, descriptions, dependencies
```

### 4. Portage Tree
```bash
# Location: /bedrock/strata/gentoo/var/db/repos/gentoo/
# Structure: category/package/package-version.ebuild
# Parse: DESCRIPTION="..." lines from ebuild files
```

### 5. Nix Store Database
```bash
# Location: /nix/var/nix/db/db.sqlite
# Tables: ValidPaths, Refs, DerivationOutputs
# Query: SELECT path FROM ValidPaths WHERE path LIKE '/nix/store/%'
```

## Expected Results After Implementation

### Package Counts (should match your neofetch):
- **DNF (Fedora)**: ~3221 packages
- **Pacman (Arch)**: ~574 installed + ~14802 available  
- **Paru (AUR)**: ~60000+ packages
- **Emerge (Gentoo)**: ~286 installed + ~19125 available
- **Nix**: ~24839 store paths

### UI Improvements:
- ✅ Consistent line rendering (no partial refreshes)
- ✅ All 5 package managers detected
- ✅ Complete package metadata (name, version, description)
- ✅ Proper Bedrock stratum awareness

## Implementation Priority

1. **CRITICAL**: Fix UI rendering bug (affects usability)
2. **HIGH**: Add missing package manager detection (dnf, paru)
3. **HIGH**: Implement proper database parsing for complete metadata
4. **MEDIUM**: Add enhanced package information (homepage, size, etc)

## Testing Strategy

After implementation, verify:
```bash
# Should detect all 5 package managers
cargo run
# Search for "firefox" - should show results from all repos
# Scroll up/down - should render consistently
# Check package counts match neofetch output
```

This implementation will give you a complete, fast, database-driven package manager multiplexer that works perfectly with Bedrock Linux!