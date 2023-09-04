A Minecraft launcher for the command line inspired by [Prism Launcher](https://prismlauncher.org/)

Work in progress, this list will change

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
- [ ] Use keyring for credential storage
- [ ] Consider using curse API for modpack search to improve perf
- [x] Show progress of modpack search when fetching pack details
- [ ] Skip interactive select when modpack search returns one result

## Refactoring

- [x] Separate lib and cli app so they have their own dependencies
- [x] Move `ToString` impl from lib to cli for formatting modpack select lists
- [x] Move code out of `main.rs` and into submodule for commands
