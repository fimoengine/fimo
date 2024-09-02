#!/bin/bash

embed_gdb_script()
{
  PATH=$1
  echo "// $PATH"
  printf "asm(\".pushsection \\\\\".debug_gdb_scripts\\\\\", \\\\\"MS\\\\\",@progbits,1\\\\n\"\n"
  printf "\t\".byte 4\\\\n\"\n"
  printf "\t\".ascii \\\\\"gdb.inlined-script\\\\\\\\n\\\\\"\\\\n\"\n"
  while IFS= read -r line
  do
    line=${line//\"/\\\\\\\"}
    printf "\t\".ascii \\\\\"%s\\\\\\\\n\\\\\"\\\\n\"\n" "$line"
  done < "$PATH"
  printf "\t\".byte 0\\\\n\"\n"
  printf "\t\".popsection\\\\n\");\n"
}

tabs 4 > /dev/null 2>&1
for var in "$@"
do
  embed_gdb_script "$var"
done