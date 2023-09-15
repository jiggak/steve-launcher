A Minecraft launcher for the command line inspired by [Prism Launcher](https://prismlauncher.org/)

# Basic Usage

Authenticate with your Microsoft account. This only needs to be run once.
The `auth` command will prompt you to open a URL, enter a code, and sign-in
to your account. When sign-in is complete, the command will save your token
in the system keyring.

    steve auth

Create a new instance.

    steve create vanilla 1.20.1

Launch the new instance.

    steve launch vanilla

# About Shared Data

All of the game assets and libraries `steve` downloads are stored in a directory
shared with all instances. This directory is resolved in the order as follows:

* Is the `-d` command line option
* `$STEVE_DATA_HOME`
* `${XDG_DATA_HOME}/steve`
* `${HOME}/.local/share/steve`

## Features

- [x] Microsoft account login
- [x] Create instances for any vanilla version
- [x] Add Minecraft Forge to new instances
- [x] Curseforge modpack zip install
- [x] Curseforge modpack search/install
- [x] FTB modpack search/install

## Chores/Improvements

- [x] Add license
- [x] API keys as built-time options with env var override
- [x] Use keyring for credential storage
- [ ] Consider using curse API for modpack search to improve perf
- [x] Show progress of modpack search when fetching pack details
- [ ] Figure out how to do error handling better, it's difficult to figure out what broke when Result::Err propagates to main
- [ ] Review all `unwrap` and `panic` calls and decide if error should propagate

## Refactoring

- [x] Separate lib and cli app so they have their own dependencies
- [x] Move `ToString` impl from lib to cli for formatting modpack select lists
- [x] Move code out of `main.rs` and into submodule for commands
