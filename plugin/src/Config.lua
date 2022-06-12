local strict = require(script.Parent.strict)

local isDevBuild = script.Parent.Parent:FindFirstChild("ROJO_DEV_BUILD") ~= nil

return strict("Config", {
	isDevBuild = isDevBuild,
	codename = "Epiphany",
	version = {7, 1, 1},
	expectedServerVersionString = "7.0 or newer",
	protocolVersion = 4,
	defaultHost = "localhost",
	defaultPort = 34872,
})