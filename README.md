A Minecraft launcher for the command line inspired by [Prism Launcher](https://prismlauncher.org/)

Work in progress, this list will change

## Features

- [x] Microsoft account login
- [x] Create instances for any vanilla version
- [x] Add Minecraft Forge to new instances
- [x] Curseforge modpack zip install
- [x] Curseforge modpack search/install
- [x] FTB modpack search/install

## Chores

- [ ] Add license
- [ ] Show progress of modpack search when fetching pack details
- [ ] Consider using curse API for modpack search to improve perf
- [ ] Skip interactive select when modpack search returns one result
- [ ] API keys as built-time options with env var override

## Refactoring

- [ ] Separate lib and cli app so they have their own dependencies
- [ ] Move `ToString` impl from lib to cli for formatting modpack select lists
- [ ] Move code out of `main.rs` and into submodule for commands
- [ ] Consider renaming `json` submodules, "manifest" in file names seems unnecessary
- [ ] Figure out how to do error handling correctly in `json.rs/int_to_string`
