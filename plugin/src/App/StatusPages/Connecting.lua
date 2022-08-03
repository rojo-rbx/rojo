local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Spinner = require(Plugin.App.Components.Spinner)

local e = Roact.createElement

local ConnectingPage = Roact.Component:extend("ConnectingPage")

function ConnectingPage:render()
	return e(Spinner, {
		position = UDim2.new(0.5, 0, 0.5, 0),
		anchorPoint = Vector2.new(0.5, 0.5),
		transparency = self.props.transparency,
	})
end

return ConnectingPage
