local Roact = require(script:FindFirstAncestor("Rojo").Roact)

local Plugin = script:FindFirstAncestor("Plugin")

local Checkbox = require(Plugin.Components.Checkbox)
local FitList = require(Plugin.Components.FitList)
local FitText = require(Plugin.Components.FitText)
local FormButton = require(Plugin.Components.FormButton)
local Panel = require(Plugin.Components.Panel)
local PluginSettings = require(Plugin.Components.PluginSettings)
local Theme = require(Plugin.Components.Theme)

local e = Roact.createElement

local SettingsPanel = Roact.Component:extend("SettingsPanel")

function SettingsPanel:render()
	local back = self.props.back

	return Theme.with(function(theme)
		return PluginSettings.with(function(settings)
			return e(Panel, nil, {
				Layout = Roact.createElement("UIListLayout", {
					HorizontalAlignment = Enum.HorizontalAlignment.Center,
					VerticalAlignment = Enum.VerticalAlignment.Center,
					SortOrder = Enum.SortOrder.LayoutOrder,
					Padding = UDim.new(0, 16),
				}),

				OpenScriptsExternally = e(FitList, {
					containerProps = {
						LayoutOrder = 1,
						BackgroundTransparency = 1,
					},
					layoutProps = {
						Padding = UDim.new(0, 4),
						FillDirection = Enum.FillDirection.Horizontal,
						HorizontalAlignment = Enum.HorizontalAlignment.Left,
						VerticalAlignment = Enum.VerticalAlignment.Center,
					},
				}, {
					Label = e(FitText, {
						Kind = "TextLabel",
						LayoutOrder = 1,
						BackgroundTransparency = 1,
						TextXAlignment = Enum.TextXAlignment.Left,
						Font = theme.MainFont,
						TextSize = 16,
						Text = "Open Scripts Externally",
						TextColor3 = theme.Text1,
					}),

					Padding = e("Frame", {
						Size = UDim2.new(0, 8, 0, 0),
						BackgroundTransparency = 1,
						LayoutOrder = 2,
					}),

					Input = e(Checkbox, {
						layoutOrder = 3,
						checked = settings:get("openScriptsExternally"),
						onChange = function(newValue)
							settings:set("openScriptsExternally", not settings:get("openScriptsExternally"))
						end,
					}),
				}),

				TwoWaySync = e(FitList, {
					containerProps = {
						LayoutOrder = 2,
						BackgroundTransparency = 1,
					},
					layoutProps = {
						Padding = UDim.new(0, 4),
						FillDirection = Enum.FillDirection.Horizontal,
						HorizontalAlignment = Enum.HorizontalAlignment.Left,
						VerticalAlignment = Enum.VerticalAlignment.Center,
					},
				}, {
					Label = e(FitText, {
						Kind = "TextLabel",
						LayoutOrder = 1,
						BackgroundTransparency = 1,
						TextXAlignment = Enum.TextXAlignment.Left,
						Font = theme.MainFont,
						TextSize = 16,
						Text = "Two-Way Sync (Experimental!)",
						TextColor3 = theme.Text1,
					}),

					Padding = e("Frame", {
						Size = UDim2.new(0, 8, 0, 0),
						BackgroundTransparency = 1,
						LayoutOrder = 2,
					}),

					Input = e(Checkbox, {
						layoutOrder = 3,
						checked = settings:get("twoWaySync"),
						onChange = function(newValue)
							settings:set("twoWaySync", not settings:get("twoWaySync"))
						end,
					}),
				}),

				BackButton = e(FormButton, {
					layoutOrder = 4,
					text = "Okay",
					secondary = true,
					onClick = function()
						back()
					end,
				}),
			})
		end)
	end)
end

return SettingsPanel