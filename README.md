A Minecraft launcher for the command line inspired by [Prism Launcher](https://prismlauncher.org/)

# Key Features

* Create, updates, and launches minecraft "instances"
* An "instance" can be a directory anywhere you like
* Search/install modpacks from CurseForge
* Easily update modpack instances to the latest pack
* Docker image for setting up and running a Minecraft server

# Building / Installing

Currently, steve requires a few environment variables to build:

* `MSA_CLIENT_ID` - Microsoft MSA Client ID
   * Requires registering an App in Azure portal
* `CURSE_API_KEY` - CurseForge API Key
   * Requires filling out a request form
   * https://support.curseforge.com/en/support/solutions/articles/9000208346-about-the-curseforge-api-and-how-to-apply-for-a-key

> I'm aware it's a bit of a bumber having these API keys be a barrier to building.
> I'm a bit stuck on what to do about that. API keys are typically not something
> you just give out to others, as they are tied to an account for these services.
>
> It "feels" crazy to give them out.
>
> BUT... if I wanted to distribute builds to users, that's exactly what I need
> to do (since the keys are strings in the binary).
> So if I'm giving out the keys in a binary, what's the difference with handing
> out keys for others to use (appart from feeling like the wrong thing to do)?

You can put them in `.cargo/config.toml` for convenient discovery by cargo,
or provide as variables in the `cargo build` command:

    MSA_CLIENT_ID=... CURSE_API_KEY=... cargo build

Assuming you have `~/.local/bin` in your `PATH`, you can install the single
`steve` binary with:

    # Install to ~/.local/bin/steve
    cargo install --path steve-cli --root ~/.local

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

> FTB seems to be distributing modpacks on CurseForge these days. But some of
> older packs are still only available from FTB.

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

When steve installs a modpack, it tracks the version of the pack and list of
installed files. Using the list of previously installed files, steve can accurately
remove files no longer included in the pack when installing a new version.

You can use the convenient `steve update` command to update to the latest version,
or use `steve modpack` to search for the latest version to install.

# Shared Data

All of the game assets and libraries `steve` downloads are stored in a directory
shared with all instances. This directory is resolved in the order as follows:

* The `-d` command line option
* `$STEVE_DATA_HOME`
* `${XDG_DATA_HOME}/steve`
* `${HOME}/.local/share/steve`

# Docker usage

The docker image is intended to be used both interactively to create the server
instance, as well as running the server (non-interactively).

The `-v` parameter mounts a volume for the instance directory to a path on the
host system so that instances outlive the container. The format of this paremter
is "[LOCAL PATH]:[CONTAINER PATH]". The container path must be `/instance`,
but can be changed with the `-i` parameter in steve.

Running the container interactively will leave behind stopped containers, but
we can remove them with the `--rm` flag.

    # Create new server instance with mod loader in "./my_mc_server"
    # Copy your desired mods to "./my_mc_server/server/mods/"
    docker run --rm -it -v my_mc_server:/instance steve server create --loader neoforge-21.1.206

    # Search and install modpack to "./my_mc_server"
    docker run --rm -it -v my_mc_server:/instance steve modpack --server /instance "skies"

    # Launch your instance
    docker run -d -v my_mc_server:/instance
