# Edisch - Edit Discord Channels

Tool to change Discord channel names in bulk.

https://github.com/gw31415/edisch/assets/24710985/3c44ab26-0911-4c14-91fe-ed1fcab008dc

## Usage

```
Tool to change Discord channel names in bulk

Usage: edisch [OPTIONS]

Options:
  -t, --token <TOKEN>        Bot token
  -g, --guild-id <GUILD_ID>  Guild ID
      --text                 Edit Text Channels
      --voice                Edit Voice Channels
      --forum                Edit Forum Channels
      --stage                Edit Stage Channels
      --news                 Edit News Channels
      --category             Edit Category Channels
      --all                  Edit all channel types
      --completion <SHELL>   Generate shell completion [possible values: bash, elvish, fish, powershell, zsh]
  -h, --help                 Print help
```

## Installation

### From source

```bash
cargo install --git https://github.com/gw31415/edisch
```
