#!/bin/sh

set -e

DIR="$( mktemp -d )"
PLUGIN_FILE="$DIR/Rojo.rbxm"
TESTEZ_FILE="$DIR/TestEZ.rbxm"

rojo build plugin -o "$PLUGIN_FILE"
rojo build plugin/testez.project.json -o "$TESTEZ_FILE"
remodel bin/mark-plugin-as-dev.lua "$PLUGIN_FILE" "$TESTEZ_FILE" 2>/dev/null

cp "$PLUGIN_FILE" "$LOCALAPPDATA/Roblox/Plugins/Rojo.rbxm"