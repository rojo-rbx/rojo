local RunService = game:GetService("RunService")

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
	plugin = plugin,
})
local tree = Roact.mount(app, nil, "Rojo UI")

local unmounted = false
function unmount()
	if not unmounted then
		Roact.unmount(tree)
		unmounted = true
	end
end

plugin.Unloading:Connect(unmount)
if RunService:IsServer() then
	game:BindToClose(unmount)
end

if Config.isDevBuild then
	local TestEZ = require(script.Parent.TestEZ)

	require(script.runTests)(TestEZ)
end
