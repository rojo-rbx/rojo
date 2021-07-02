if not plugin then
	return
end

local Log = require(script.Parent.Log)

local DevSettings = require(script.DevSettings)

Log.setLogLevelThunk(function()
	return DevSettings:getLogLevel()
end)

local Roact = require(script.Parent.Roact)

local Config = require(script.Config)
local App = require(script.App)

local app = Roact.createElement(App, {
	plugin = plugin
})
local tree = Roact.mount(app, nil, "Rojo UI")

game.Close:Connect(function()
	-- Unmount on game close to cleanup active connection if it exists
	Roact.unmount(tree)
end)

plugin.Unloading:Connect(function()
	Roact.unmount(tree)
end)

if Config.isDevBuild then
	local TestEZ = require(script.Parent.TestEZ)

	require(script.runTests)(TestEZ)
end