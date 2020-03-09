local Roact = require(script:FindFirstAncestor("Rojo").Roact)

local Plugin = script:FindFirstAncestor("Plugin")

local Theme = require(Plugin.Components.Theme)
local Panel = require(Plugin.Components.Panel)
local FitText = require(Plugin.Components.FitText)
local FormButton = require(Plugin.Components.FormButton)

local e = Roact.createElement

local ConnectionActivePanel = Roact.Component:extend("ConnectionActivePanel")

function ConnectionActivePanel:render()
	local stopSession = self.props.stopSession

	return Theme.with(function(theme)
		return e(Panel, nil, {
			Layout = Roact.createElement("UIListLayout", {
				HorizontalAlignment = Enum.HorizontalAlignment.Center,
				VerticalAlignment = Enum.VerticalAlignment.Center,
				SortOrder = Enum.SortOrder.LayoutOrder,
				Padding = UDim.new(0, 8),
			}),

			Text = e(FitText, {
				Padding = Vector2.new(12, 6),
				Font = theme.ButtonFont,
				TextSize = 18,
				Text = "Connected to Live-Sync Server",
				TextColor3 = theme.Text1,
				BackgroundTransparency = 1,
			}),

			DisconnectButton = e(FormButton, {
				layoutOrder = 2,
				text = "Disconnect",
				secondary = true,
				onClick = function()
					stopSession()
				end,
			}),
		})
	end)
end

return ConnectionActivePanel