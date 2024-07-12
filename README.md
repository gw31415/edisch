# Edisch - Edit Discord Channels

[![Crates.io](https://img.shields.io/crates/v/edisch?style=flat-square)](https://crates.io/crates/edisch)
[![Crates.io](https://img.shields.io/crates/d/edisch?style=flat-square)](https://crates.io/crates/edisch)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue?style=flat-square)](LICENSE)

**`edisch`** /ˈɛdɪʃ/ is a tool to change Discord channel names in bulk with your $EDITOR

https://github.com/gw31415/edisch/assets/24710985/3c44ab26-0911-4c14-91fe-ed1fcab008dc

## Installation

### Cargo

```bash
cargo install edisch
```

## Usage

```
Tool to change Discord channel names in bulk with your $EDITOR

Usage: edisch [OPTIONS]
       edisch <COMMAND>

Commands:
  export      Export all channel names to a file or stdout
  apply       Apply all channel names from a file or stdin
  completion  Generate shell completion
  help        Print this message or the help of the given subcommand(s)

Options:
  -t, --token <TOKEN>        Bot token. If not provided, it will be read from the $DISCORD_TOKEN environment variable
  -g, --guild-id <GUILD_ID>  Guild ID. If not provided, it will be read from the $GUILD_ID environment variable
      --text                 Edit Text Channels
      --voice                Edit Voice Channels
      --forum                Edit Forum Channels
      --stage                Edit Stage Channels
      --news                 Edit News Channels
      --category             Edit Category Channels
      --all                  Edit All Channels
  -y, --yes                  Automatically confirm all changes
  -h, --help                 Print help
  -V, --version              Print version
```

### Examples

```bash
# Edit all text channels in the guild
edisch --text

# Batch edit all channels in the guild
edisch export | sed 's/old/new/g' | edisch apply -y
```
