#!/bin/sh

set -ev

rojo build plugin/place.project.json -o PluginPlace.rbxlx
run-in-roblox -s plugin/testBootstrap.server.lua PluginPlace.rbxlx