local Roact = require(script:FindFirstAncestor("Rojo").Roact)

local Plugin = script:FindFirstAncestor("Plugin")

local Theme = require(Plugin.Theme)

local Panel = require(Plugin.Components.Panel)
local FitText = require(Plugin.Components.FitText)

local e = Roact.createElement

local ConnectingPanel = Roact.Component:extend("ConnectingPanel")

function ConnectingPanel:render()
	return e(Panel, nil, {
		Layout = Roact.createElement("UIListLayout", {
			HorizontalAlignment = Enum.HorizontalAlignment.Center,
			VerticalAlignment = Enum.VerticalAlignment.Center,
			SortOrder = Enum.SortOrder.LayoutOrder,
			Padding = UDim.new(0, 8),
		}),

		Text = e(FitText, {
			Padding = Vector2.new(12, 6),
			Font = Theme.ButtonFont,
			TextSize = 18,
			Text = "Connecting...",
			TextColor3 = Theme.PrimaryColor,
			BackgroundTransparency = 1,
		}),
	})
end

return ConnectingPanel