local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local Theme = require(Plugin.App.Theme)
local TextButton = require(Plugin.App.Components.TextButton)
local Header = require(Plugin.App.Components.Header)

local e = Roact.createElement

local PatchDiff = require(script.PatchDiff)

local ConfirmingPage = Roact.Component:extend("ConfirmingPage")

function ConfirmingPage:render()
	return Theme.with(function(theme)
		return Roact.createFragment({
			Header = e(Header, {
				transparency = self.props.transparency,
				layoutOrder = 1,
			}),

			Title = e("TextLabel", {
				Text = string.format(
					"Sync changes for project '%s':",
					self.props.confirmData.serverInfo.projectName or "UNKNOWN"
				),
				LayoutOrder = 2,
				Font = Enum.Font.Gotham,
				LineHeight = 1.2,
				TextSize = 14,
				TextColor3 = theme.Settings.Setting.DescriptionColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = self.props.transparency,
				Size = UDim2.new(1, 0, 0, 20),
				BackgroundTransparency = 1,
			}),

			PatchDiff = e(PatchDiff, {
				confirmData = self.props.confirmData,
				transparency = self.props.transparency,
				layoutOrder = 3,
			}),

			Buttons = e("Frame", {
				Size = UDim2.new(1, 0, 0, 34),
				LayoutOrder = 4,
				BackgroundTransparency = 1,
			}, {
				Abort = e(TextButton, {
					text = "Abort",
					style = "Bordered",
					transparency = self.props.transparency,
					layoutOrder = 1,
					onClick = self.props.onAbort,
				}),

				Reject = e(TextButton, {
					text = "Reject",
					style = "Bordered",
					transparency = self.props.transparency,
					layoutOrder = 2,
					onClick = self.props.onReject,
				}),

				Accept = e(TextButton, {
					text = "Accept",
					style = "Solid",
					transparency = self.props.transparency,
					layoutOrder = 3,
					onClick = self.props.onAccept,
				}),

				Layout = e("UIListLayout", {
					HorizontalAlignment = Enum.HorizontalAlignment.Right,
					FillDirection = Enum.FillDirection.Horizontal,
					SortOrder = Enum.SortOrder.LayoutOrder,
					Padding = UDim.new(0, 10),
				}),
			}),

			Layout = e("UIListLayout", {
				HorizontalAlignment = Enum.HorizontalAlignment.Center,
				VerticalAlignment = Enum.VerticalAlignment.Center,
				FillDirection = Enum.FillDirection.Vertical,
				SortOrder = Enum.SortOrder.LayoutOrder,
				Padding = UDim.new(0, 10),
			}),

			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, 20),
				PaddingRight = UDim.new(0, 20),
			}),
		})
	end)
end

return ConfirmingPage
