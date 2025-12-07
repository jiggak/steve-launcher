A Minecraft launcher for the command line inspired by [Prism Launcher](https://prismlauncher.org/)

# Usage

Authenticate with your Microsoft account. This only needs to be run once.
The `auth` command will prompt you to open a URL, enter a code, and sign-in
to your account. When sign-in is complete, the command will save your token
in the system keyring.

    steve auth

Use the auth status command check credentials are saved, and show some basic
information about the stored tokens.

    steve auth status

Delete the stored credentials from system keyring.

    steve auth clear

Create a new instance. If you omit the Minecraft version argument, `steve` will
prompt you to select the version from a list.

    steve create -i ./Vanilla 1.20.1

To add a mod loader to the instance, pass the `--loader` option with the name
of the mod loader (`forge` or `neoforge`). Pass the specific version, or don't
and `steve` will prompt to select a version that matches the Minecraft version.

    steve create -i ./MyModpack 1.20.1 --loader forge-47.3.7
    # prompt forge version when version not specified
    steve create -i ./MyModpack 1.20.1 --loader forge

Launch the new instance.

    steve launch -i ./Vanilla

Search for modpacks with "atm10" in the name and install to the path "./ATM10".
Modpack search supports FTB and CurseForge.

> FTB seems to be distributing modpacks on CurseForge these days. The

    steve modpack -i ./ATM10 atm10

Did you download a modpack ZIP file from CurseForge?

    steve import -i ~/ATM9 "~/Downloads/All+the+Mods+9-0.1.4.zip"

# Instance Directory

Steve will use the current directory to create or launch an instance.
To specifiy the instance directory, use the `-i` parameter on any command
that operates on an instance.

    steve launch              # Launch instance in current directory
    steve launch -i ./        # Same as previous command
    steve launch -i ~/Vanilla # Launch instance "$HOME/Vanilla" directory

# Modpack Updating

Specifying an existing instance directory when installing a modpack will replace
any files in the pack distribution that match existing files, but leave all other
instances files alone. This means it's possible to "update" an existing instance
to the latest modpack version.

If the update adds new versions of mods, resource packs, or shader packs, `steve`
will prompt you to remove the old ones. For mods in particular this is important
as duplicate versions will cause an error at launch.

# Shared Data

All of the game assets and libraries `steve` downloads are stored in a directory
shared with all instances. This directory is resolved in the order as follows:

* The `-d` command line option
* `$STEVE_DATA_HOME`
* `${XDG_DATA_HOME}/steve`
* `${HOME}/.local/share/steve`
