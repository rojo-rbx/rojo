local Roact = require(script:FindFirstAncestor("Rojo").Roact)

local Plugin = script:FindFirstAncestor("Plugin")

local Theme = require(Plugin.Components.Theme)
local Panel = require(Plugin.Components.Panel)
local FitText = require(Plugin.Components.FitText)
local FitScrollingFrame = require(Plugin.Components.FitScrollingFrame)
local FormButton = require(Plugin.Components.FormButton)

local e = Roact.createElement

local BUTTON_HEIGHT = 60
local HOR_PADDING = 8

local ErrorPanel = Roact.Component:extend("ErrorPanel")

function ErrorPanel:render()
	local errorMessage = self.props.errorMessage
	local onDismiss = self.props.onDismiss

	return Theme.with(function(theme)
		return e(Panel, nil, {
			Layout = Roact.createElement("UIListLayout", {
				HorizontalAlignment = Enum.HorizontalAlignment.Center,
				VerticalAlignment = Enum.VerticalAlignment.Center,
				SortOrder = Enum.SortOrder.LayoutOrder,
				Padding = UDim.new(0, 8),
			}),

			ErrorContainer = e(FitScrollingFrame, {
				containerProps = {
					BackgroundTransparency = 1,
					BorderSizePixel = 0,
					Size = UDim2.new(1, -HOR_PADDING * 2, 1, -BUTTON_HEIGHT),
					Position = UDim2.new(0, HOR_PADDING, 0, 0),
					ScrollBarImageColor3 = theme.Text1,
					VerticalScrollBarInset = Enum.ScrollBarInset.ScrollBar,
					ScrollingDirection = Enum.ScrollingDirection.Y,
				},
			}, {
				Text = e(FitText, {
					Size = UDim2.new(1, 0, 0, 0),

					LayoutOrder = 1,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextYAlignment = Enum.TextYAlignment.Top,
					FitAxis = "Y",
					Font = theme.ButtonFont,
					TextSize = 18,
					Text = errorMessage,
					TextWrap = true,
					TextColor3 = theme.Text1,
					BackgroundTransparency = 1,
				}),
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
	end)
end

return ErrorPanel