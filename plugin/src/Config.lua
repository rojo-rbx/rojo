local strict = require(script.Parent.strict)

local isDevBuild = script.Parent.Parent:FindFirstChild("ROJO_DEV_BUILD") ~= nil

return strict("Config", {
	isDevBuild = isDevBuild,
	codename = "Epiphany",
	version = {6, 0, 0, "-rc.4"},
	expectedServerVersionString = "6.0 or newer",
	protocolVersion = 3,
	defaultHost = "localhost",
	defaultPort = 34872,
})