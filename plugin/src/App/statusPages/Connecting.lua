local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local Throbber = require(Plugin.App.components.Throbber)

local e = Roact.createElement

local ConnectingPage = Roact.Component:extend("ConnectingPage")

function ConnectingPage:render()
	return e(Throbber, {
		position = UDim2.new(0.5, 0, 0.5, 0),
		anchorPoint = Vector2.new(0.5, 0.5),
		transparency = self.props.transparency,
	})
end

return ConnectingPage