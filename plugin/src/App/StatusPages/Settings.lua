local TextService = game:GetService("TextService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local Assets = require(Plugin.Assets)
local Theme = require(Plugin.App.Theme)
local PluginSettings = require(Plugin.App.PluginSettings)

local Checkbox = require(Plugin.App.Components.Checkbox)
local IconButton = require(Plugin.App.Components.IconButton)
local ScrollingFrame = require(Plugin.App.Components.ScrollingFrame)

local e = Roact.createElement

local DIVIDER_FADE_SIZE = 0.1

local function getTextBounds(text, textSize, font, lineHeight, bounds)
	local textBounds = TextService:GetTextSize(text, textSize, font, bounds)

	local lineCount = textBounds.Y / textSize
	local lineHeightAbsolute = textSize * lineHeight

	return Vector2.new(textBounds.X, lineHeightAbsolute * lineCount - (lineHeightAbsolute - textSize))
end

local function Navbar(props)
	return Theme.with(function(theme)
		theme = theme.Settings.Navbar

		return e("Frame", {
			Size = UDim2.new(1, 0, 0, 46),
			LayoutOrder = props.layoutOrder,
			BackgroundTransparency = 1,
		}, {
			Back = e(IconButton, {
				icon = Assets.Images.Icons.Back,
				iconSize = 24,
				color = theme.BackButtonColor,
				transparency = props.transparency,

				position = UDim2.new(0, 0, 0.5, 0),
				anchorPoint = Vector2.new(0, 0.5),

				onClick = props.onBack,
			}),

			Text = e("TextLabel", {
				Text = "Settings",
				Font = Enum.Font.Gotham,
				TextSize = 18,
				TextColor3 = theme.TextColor,
				TextTransparency = props.transparency,

				Size = UDim2.new(1, 0, 1, 0),

				BackgroundTransparency = 1,
			})
		})
	end)
end

local Setting = Roact.Component:extend("Setting")

function Setting:init()
	self.contentSize, self.setContentSize = Roact.createBinding(Vector2.new(0, 0))
	self.containerSize, self.setContainerSize = Roact.createBinding(Vector2.new(0, 0))
end

function Setting:render()
	return Theme.with(function(theme)
		theme = theme.Settings

		return PluginSettings.with(function(settings)
			return e("Frame", {
				Size = self.contentSize:map(function(value)
					return UDim2.new(1, 0, 0, 20 + value.Y + 20)
				end),
				LayoutOrder = self.props.layoutOrder,
				BackgroundTransparency = 1,

				[Roact.Change.AbsoluteSize] = function(object)
					self.setContainerSize(object.AbsoluteSize)
				end,
			}, {
				Checkbox = e(Checkbox, {
					active = settings:get(self.props.id),
					transparency = self.props.transparency,
					position = UDim2.new(1, 0, 0.5, 0),
					anchorPoint = Vector2.new(1, 0.5),
					onClick = function()
						local currentValue = settings:get(self.props.id)
						settings:set(self.props.id, not currentValue)
					end,
				}),

				Text = e("Frame", {
					Size = UDim2.new(1, 0, 1, 0),
					BackgroundTransparency = 1,
				}, {
					Name = e("TextLabel", {
						Text = self.props.name,
						Font = Enum.Font.GothamBold,
						TextSize = 17,
						TextColor3 = theme.Setting.NameColor,
						TextXAlignment = Enum.TextXAlignment.Left,
						TextTransparency = self.props.transparency,

						Size = UDim2.new(1, 0, 0, 17),

						LayoutOrder = 1,
						BackgroundTransparency = 1,
					}),

					Description = e("TextLabel", {
						Text = self.props.description,
						Font = Enum.Font.Gotham,
						LineHeight = 1.2,
						TextSize = 14,
						TextColor3 = theme.Setting.DescriptionColor,
						TextXAlignment = Enum.TextXAlignment.Left,
						TextTransparency = self.props.transparency,
						TextWrapped = true,

						Size = self.containerSize:map(function(value)
							local textBounds = getTextBounds(
								self.props.description, 14, Enum.Font.Gotham, 1.2,
								Vector2.new(value.X - 50, math.huge)
							)
							return UDim2.new(1, -50, 0, textBounds.Y)
						end),

						LayoutOrder = 2,
						BackgroundTransparency = 1,
					}),

					Layout = e("UIListLayout", {
						VerticalAlignment = Enum.VerticalAlignment.Center,
						FillDirection = Enum.FillDirection.Vertical,
						SortOrder = Enum.SortOrder.LayoutOrder,
						Padding = UDim.new(0, 6),

						[Roact.Change.AbsoluteContentSize] = function(object)
							self.setContentSize(object.AbsoluteContentSize)
						end,
					}),

					Padding = e("UIPadding", {
						PaddingTop = UDim.new(0, 20),
						PaddingBottom = UDim.new(0, 20),
					}),
				}),

				Divider = e("Frame", {
					BackgroundColor3 = theme.DividerColor,
					BackgroundTransparency = self.props.transparency,
					Size = UDim2.new(1, 0, 0, 1),
					BorderSizePixel = 0,
				}, {
					Gradient = e("UIGradient", {
						Transparency = NumberSequence.new({
							NumberSequenceKeypoint.new(0, 1),
							NumberSequenceKeypoint.new(DIVIDER_FADE_SIZE, 0),
							NumberSequenceKeypoint.new(1 - DIVIDER_FADE_SIZE, 0),
							NumberSequenceKeypoint.new(1, 1),
						}),
					}),
				}),
			})
		end)
	end)
end

local SettingsPage = Roact.Component:extend("SettingsPage")

function SettingsPage:init()
	self.contentSize, self.setContentSize = Roact.createBinding(Vector2.new(0, 0))
end

function SettingsPage:render()
	return Theme.with(function(theme)
		theme = theme.Settings

		return e(ScrollingFrame, {
			size = UDim2.new(1, 0, 1, 0),
			contentSize = self.contentSize,
			transparency = self.props.transparency,
		}, {
			Navbar = e(Navbar, {
				onBack = self.props.onBack,
				transparency = self.props.transparency,
				layoutOrder = 0,
			}),

			OpenScriptsExternally = e(Setting, {
				id = "openScriptsExternally",
				name = "Open Scripts Externally",
				description = "Attempt to open scripts in an external editor",
				transparency = self.props.transparency,
				layoutOrder = 1,
			}),

			TwoWaySync = e(Setting, {
				id = "twoWaySync",
				name = "Two-Way Sync",
				description = "EXPERIMENTAL! Editing files in Studio will sync them into the filesystem",
				transparency = self.props.transparency,
				layoutOrder = 2,
			}),

			Layout = e("UIListLayout", {
				FillDirection = Enum.FillDirection.Vertical,
				SortOrder = Enum.SortOrder.LayoutOrder,

				[Roact.Change.AbsoluteContentSize] = function(object)
					self.setContentSize(object.AbsoluteContentSize)
				end,
			}),

			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, 20),
				PaddingRight = UDim.new(0, 20),
			}),
		})
	end)
end

return SettingsPage