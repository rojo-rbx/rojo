local Roact = require(script:FindFirstAncestor("Rojo").Roact)

local Plugin = script:FindFirstAncestor("Plugin")

local Theme = require(Plugin.Theme)

local Panel = require(Plugin.Components.Panel)
local FitText = require(Plugin.Components.FitText)
local FormButton = require(Plugin.Components.FormButton)

local e = Roact.createElement

local ErrorPanel = Roact.Component:extend("ErrorPanel")

function ErrorPanel:render()
	local errorMessage = self.props.errorMessage
	local onDismiss = self.props.onDismiss

	return e(Panel, nil, {
		Layout = Roact.createElement("UIListLayout", {
			HorizontalAlignment = Enum.HorizontalAlignment.Center,
			VerticalAlignment = Enum.VerticalAlignment.Center,
			SortOrder = Enum.SortOrder.LayoutOrder,
			Padding = UDim.new(0, 8),
		}),

		Text = e(FitText, {
			LayoutOrder = 1,
			FitAxis = "Y",
			Size = UDim2.new(1, 0, 0, 0),
			Padding = Vector2.new(12, 6),
			Font = Theme.ButtonFont,
			TextSize = 18,
			Text = errorMessage,
			TextWrap = true,
			TextColor3 = Theme.PrimaryColor,
			BackgroundTransparency = 1,
		}),

		DismissButton = e(FormButton, {
			layoutOrder = 2,
			text = "Dismiss",
			secondary = true,
			onClick = function()
				onDismiss()
			end,
		}),
	})
end

return ErrorPanel