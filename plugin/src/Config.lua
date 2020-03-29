local strict = require(script.Parent.strict)

local isDevBuild = script.Parent.Parent:FindFirstChild("ROJO_DEV_BUILD") ~= nil

return strict("Config", {
	isDevBuild = isDevBuild,
	codename = "Epiphany",
	version = {0, 6, 0, "-alpha.3"},
	expectedServerVersionString = "0.6.0 or newer",
	protocolVersion = 3,
	defaultHost = "localhost",
	defaultPort = 34872,
})