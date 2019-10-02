#!/bin/sh

set -e

DIR="$( mktemp -d )"
PLUGIN_FILE="$DIR/Rojo.rbxmx"
PLACE_FILE="$DIR/RojoTestPlace.rbxlx"

rojo build plugin -o "$PLUGIN_FILE"
rojo build plugin/place.project.json -o "$PLACE_FILE"

remodel bin/put-plugin-in-test-place.lua "$PLUGIN_FILE" "$PLACE_FILE"

run-in-roblox -s plugin/testBootstrap.server.lua "$PLACE_FILE"

pushd plugin
luacheck src
popd

pushd rojo-plugin-http
luacheck src
popd

pushd rojo-plugin-log
luacheck src
popd