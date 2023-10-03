A Minecraft launcher for the command line inspired by [Prism Launcher](https://prismlauncher.org/)

# Usage

Authenticate with your Microsoft account. This only needs to be run once.
The `auth` command will prompt you to open a URL, enter a code, and sign-in
to your account. When sign-in is complete, the command will save your token
in the system keyring.

    steve auth

Use the auth status command check credentials are save, and show some basic
information about the stored tokens.

    steve auth status

Delete the stored credentials from system keyring.

    steve auth clear

Create a new instance. If you omit the Minecraft version argument, `steve` will
prompt you to select the version from a list.

    steve create vanilla 1.20.1

To add forge to the instance, pass the `--forge` option. Pass the specific forge
version, or don't and `steve` will prompt to select a version that matches the
Minecraft version.

    steve create my_modpack 1.20.1 --forge

Launch the new instance.

    steve launch vanilla

Search for modpacks with "atm9" in the name and install to the path "Minecraft/ATM9".
Modpack search supports FTB and CurseForge.

    steve modpack Minecraft/ATM9 atm9

Do you download a modpack ZIP file from CurseForge?

    steve import Minecraft/ATM9 "~/Downloads/All+the+Mods+9-0.1.4.zip"

# About Modpack Updating

Specifying an existing instance directory when installing a modpack will replace
any files in the pack distribution that match existing files, but leave all other
instances files alone. This means it's possible to "update" an existing instance
to the latest modpack version.

If the update adds new versions of mods, resource packs, or shader packs, `steve`
will prompt you to remove the old ones. For mods in particular this is important
as duplicate versions will cause an error at launch.

# About Shared Data

All of the game assets and libraries `steve` downloads are stored in a directory
shared with all instances. This directory is resolved in the order as follows:

* The `-d` command line option
* `$STEVE_DATA_HOME`
* `${XDG_DATA_HOME}/steve`
* `${HOME}/.local/share/steve`
