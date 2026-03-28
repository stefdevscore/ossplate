#!/bin/sh

if [ "$1" = "version" ]; then
  printf '{"tool":"stub-tool","version":"9.9.9"}\n'
  exit 0
fi

if [ "$1" = "validate" ] && [ "$2" = "--json" ]; then
  printf '{"ok":true,"issues":[]}\n'
  exit 0
fi

if [ "$1" = "sync" ] && [ "$2" = "--check" ]; then
  printf 'sync check ok\n'
  exit 0
fi

printf 'stub received: %s\n' "$*" >&2
exit 7
