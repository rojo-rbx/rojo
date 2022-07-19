if not plugin then
	return
end

local Roact = require(script.Parent.Roact)
local Log = require(script.Parent.Log)

local Settings = require(script.Settings)
local Config = require(script.Config)
local App = require(script.App)

Log.setLogLevelThunk(function()
	return Log.Level[Settings:get("logLevel")] or Log.Level.Info
end)

local app = Roact.createElement(App, {
	plugin = plugin
})
local tree = Roact.mount(app, game:GetService("CoreGui"), "Rojo UI")

plugin.Unloading:Connect(function()
	Roact.unmount(tree)
end)

if Config.isDevBuild then
	local TestEZ = require(script.Parent.TestEZ)

	require(script.runTests)(TestEZ)
end
