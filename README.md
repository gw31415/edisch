# Edisch - Edit Discord Channels

Tool to change Discord channel names in bulk with your $EDITOR

https://github.com/gw31415/edisch/assets/24710985/3c44ab26-0911-4c14-91fe-ed1fcab008dc

## Usage

```
Tool to change Discord channel names in bulk with your $EDITOR

Usage: edisch [OPTIONS]

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
      --completion <SHELL>   Generate shell completion [possible values: bash, elvish, fish, powershell, zsh]
  -h, --help                 Print help
```

## Installation

### From source

```bash
cargo install --git https://github.com/gw31415/edisch
```
