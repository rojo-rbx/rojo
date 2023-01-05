local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Config = require(Plugin.Config)
local Version = require(Plugin.Version)
local Theme = require(Plugin.App.Theme)

local TextButton = require(Plugin.App.Components.TextButton)

local e = Roact.createElement

local ConflictAPIPopup = Roact.Component:extend("ConflictAPIPopup")

function ConflictAPIPopup:render()
	return Theme.with(function(theme)
		theme = theme.Settings

		return e("Frame", {
			BackgroundTransparency = 1,
			Size = UDim2.new(1, 0, 1, 0),
		}, {
			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, 20),
				PaddingRight = UDim.new(0, 20),
				PaddingTop = UDim.new(0, 15),
				PaddingBottom = UDim.new(0, 15),
			}),

			Details = e("Frame", {
				BackgroundTransparency = 1,
				Size = UDim2.new(1, 0, 1, 0),
			}, {
				Layout = e("UIListLayout", {
					FillDirection = Enum.FillDirection.Vertical,
					SortOrder = Enum.SortOrder.LayoutOrder,
					Padding = UDim.new(0, 15),
					HorizontalAlignment = Enum.HorizontalAlignment.Center,
				}),

				Info = e("TextLabel", {
					BackgroundTransparency = 1,
					Size = UDim2.new(1, 0, 0, 0),
					AutomaticSize = Enum.AutomaticSize.Y,
					Text = "There is already a Rojo API exposed by a Rojo plugin. Do you want to overwrite it with this one?",
					Font = Enum.Font.GothamMedium,
					TextSize = 17,
					TextColor3 = theme.Setting.NameColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextWrapped = true,
					TextTransparency = self.props.transparency,
					LayoutOrder = 1,
				}),

				Existing = e("TextLabel", {
					BackgroundTransparency = 1,
					Size = UDim2.new(1, 0, 0, 0),
					AutomaticSize = Enum.AutomaticSize.Y,
					Text = string.format("Existing: Version %s, Protocol %d", Version.display(self.props.existingAPI.Version), self.props.existingAPI.ProtocolVersion),
					Font = Enum.Font.Gotham,
					TextSize = 15,
					TextColor3 = theme.Setting.DescriptionColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextTransparency = self.props.transparency,
					LayoutOrder = 2,
				}),

				Incoming = e("TextLabel", {
					BackgroundTransparency = 1,
					Size = UDim2.new(1, 0, 0, 0),
					AutomaticSize = Enum.AutomaticSize.Y,
					Text = string.format("Incoming: Version %s, Protocol %d", Version.display(Config.version), Config.protocolVersion),
					Font = Enum.Font.Gotham,
					TextSize = 15,
					TextColor3 = theme.Setting.DescriptionColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextTransparency = self.props.transparency,
					LayoutOrder = 3,
				}),
			}),

			Actions = e("Frame", {
				BackgroundTransparency = 1,
				Size = UDim2.new(1, 0, 0, 34),
				Position = UDim2.fromScale(0, 1),
				AnchorPoint = Vector2.new(0, 1),
			}, {
				Layout = e("UIListLayout", {
					FillDirection = Enum.FillDirection.Horizontal,
					SortOrder = Enum.SortOrder.LayoutOrder,
					Padding = UDim.new(0, 10),
					HorizontalAlignment = Enum.HorizontalAlignment.Right,
				}),

				Keep = e(TextButton, {
					text = "Keep",
					style = "Bordered",
					transparency = self.props.transparency,
					layoutOrder = 1,
					onClick = function()
						self.props.onDeny()
					end,
				}),

				Overwrite = e(TextButton, {
					text = "Overwrite",
					style = "Solid",
					transparency = self.props.transparency,
					layoutOrder = 2,
					onClick = function()
						self.props.onAccept()
					end,
				}),
			}),
		})
	end)
end

return ConflictAPIPopup
