# iso2raw

ISO to RAW (MODE1/2352) converter written in Rust.

## Overview

`iso2raw` converts ISO 9660 files (2048 bytes per sector) to RAW CD-ROM format (MODE1/2352 bytes per sector). This is useful for creating CD images that can be written to physical media or used with emulators that require the full sector format including EDC/ECC data.

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Basic usage - converts input.iso to input.bin
iso2raw input.iso

# Specify output file
iso2raw input.iso -o output.bin

# Use specific number of threads
iso2raw input.iso -j 4

# Quiet mode (no progress bar)
iso2raw input.iso -q
```

## Building from Source

```bash
git clone https://github.com/yourusername/iso2raw.git
cd iso2raw
cargo build --release
```

## Testing

```bash
cargo test
```

## License

This project is open source. See LICENSE file for details.