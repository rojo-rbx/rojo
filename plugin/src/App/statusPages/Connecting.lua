local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local Throbber = require(Plugin.App.components.Throbber)
local TextButton = require(Plugin.App.components.TextButton)

local e = Roact.createElement

local ConnectingPage = Roact.Component:extend("ConnectingPage")

function ConnectingPage:render()
	return Roact.createFragment({
		Throbber = e(Throbber, {
			transparency = self.props.transparency,
			layoutOrder = 1,
		}),

		Cancel = e(TextButton, {
			text = "Cancel",
			style = "Bordered",
			transparency = self.props.transparency,
			layoutOrder = 2,
			onClick = self.props.onCancel,
		}),

		Layout = e("UIListLayout", {
			HorizontalAlignment = Enum.HorizontalAlignment.Center,
			VerticalAlignment = Enum.VerticalAlignment.Center,
			FillDirection = Enum.FillDirection.Vertical,
			SortOrder = Enum.SortOrder.LayoutOrder,
			Padding = UDim.new(0, 30),
		}),
	})
end

return ConnectingPage