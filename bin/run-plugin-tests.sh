#!/bin/sh

set -e

DIR="$( mktemp -d )"
PLUGIN_FILE="$DIR/Rojo.rbxmx"
PLACE_FILE="$DIR/RojoTestPlace.rbxlx"

rojo build plugin -o "$PLUGIN_FILE"
rojo build plugin/place.project.json -o "$PLACE_FILE"

remodel bin/put-plugin-in-test-place.lua "$PLUGIN_FILE" "$PLACE_FILE"

run-in-roblox -s plugin/testBootstrap.server.lua "$PLACE_FILE"

luacheck plugin/src plugin/log plugin/http