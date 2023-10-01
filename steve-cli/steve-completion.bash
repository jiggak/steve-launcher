#/usr/bin/env bash

_steve() {
   if [ "$3" == "launch" ]; then
      local IFS=$'\n'
      COMPREPLY=( $(compgen -d -- "$2") )
   else
      COMPREPLY=( $(compgen -W "create launch auth import modpack help" -- "$2") )
   fi
}

complete -F _steve steve
