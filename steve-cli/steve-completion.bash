#/usr/bin/env bash

_steve() {
   if [ "$3" == "launch" ]; then
      local IFS=$'\n'
      COMPREPLY=( $(compgen -d -- "$2") )
   else
      COMPREPLY=( $(compgen -W "auth create launch import modpack completion" -- "$2") )
   fi
}

complete -F _steve steve
