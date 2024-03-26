local TextService = game:GetService("TextService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Settings = require(Plugin.Settings)
local Assets = require(Plugin.Assets)
local Theme = require(Plugin.App.Theme)

local Checkbox = require(Plugin.App.Components.Checkbox)
local Dropdown = require(Plugin.App.Components.Dropdown)
local IconButton = require(Plugin.App.Components.IconButton)
local Tag = require(Plugin.App.Components.Tag)

local e = Roact.createElement

local DIVIDER_FADE_SIZE = 0.1
local TAG_TYPES = {
	unstable = {
		text = "UNSTABLE",
		icon = Assets.Images.Icons.Warning,
		color = { "Settings", "Setting", "UnstableColor" },
	},
	debug = {
		text = "DEBUG",
		icon = Assets.Images.Icons.Debug,
		color = { "Settings", "Setting", "DebugColor" },
	},
}

local function getTextBounds(text, textSize, font, lineHeight, bounds)
	local textBounds = TextService:GetTextSize(text, textSize, font, bounds)

	local lineCount = textBounds.Y / textSize
	local lineHeightAbsolute = textSize * lineHeight

	return Vector2.new(textBounds.X, lineHeightAbsolute * lineCount - (lineHeightAbsolute - textSize))
end

local function getThemeColorFromPath(theme, path)
	local color = theme
	for _, key in path do
		if color[key] == nil then
			return theme.BrandColor
		end
		color = color[key]
	end
	return color
end

local Setting = Roact.Component:extend("Setting")

function Setting:init()
	self.contentSize, self.setContentSize = Roact.createBinding(Vector2.new(0, 0))
	self.containerSize, self.setContainerSize = Roact.createBinding(Vector2.new(0, 0))
	self.inputSize, self.setInputSize = Roact.createBinding(Vector2.new(0, 0))

	self:setState({
		setting = Settings:get(self.props.id),
	})

	self.changedCleanup = Settings:onChanged(self.props.id, function(value)
		self:setState({
			setting = value,
		})
	end)
end

function Setting:willUnmount()
	self.changedCleanup()
end

function Setting:render()
	return Theme.with(function(theme)
		local settingsTheme = theme.Settings

		return e("Frame", {
			Size = self.contentSize:map(function(value)
				return UDim2.new(1, 0, 0, value.Y + 20)
			end),
			LayoutOrder = self.props.layoutOrder,
			ZIndex = -self.props.layoutOrder,
			BackgroundTransparency = 1,
			Visible = self.props.visible,

			[Roact.Change.AbsoluteSize] = function(object)
				self.setContainerSize(object.AbsoluteSize)
			end,
		}, {
			RightAligned = Roact.createElement("Frame", {
				BackgroundTransparency = 1,
				Size = UDim2.new(1, 0, 1, 0),
			}, {
				Layout = e("UIListLayout", {
					VerticalAlignment = Enum.VerticalAlignment.Center,
					HorizontalAlignment = Enum.HorizontalAlignment.Right,
					FillDirection = Enum.FillDirection.Horizontal,
					SortOrder = Enum.SortOrder.LayoutOrder,
					Padding = UDim.new(0, 2),
					[Roact.Change.AbsoluteContentSize] = function(rbx)
						self.setInputSize(rbx.AbsoluteContentSize)
					end,
				}),

				Input = if self.props.input ~= nil
					then self.props.input
					elseif self.props.options ~= nil then e(Dropdown, {
						locked = self.props.locked,
						options = self.props.options,
						active = self.state.setting,
						transparency = self.props.transparency,
						onClick = function(option)
							Settings:set(self.props.id, option)
						end,
					})
					else e(Checkbox, {
						locked = self.props.locked,
						active = self.state.setting,
						transparency = self.props.transparency,
						onClick = function()
							local currentValue = Settings:get(self.props.id)
							Settings:set(self.props.id, not currentValue)
						end,
					}),

				Reset = if self.props.onReset
					then e(IconButton, {
						icon = Assets.Images.Icons.Reset,
						iconSize = 24,
						color = settingsTheme.BackButtonColor,
						transparency = self.props.transparency,
						visible = self.props.showReset,
						layoutOrder = -1,

						onClick = self.props.onReset,
					})
					else nil,
			}),

			Text = e("Frame", {
				Size = UDim2.new(1, 0, 1, 0),
				BackgroundTransparency = 1,
			}, {
				Heading = e("Frame", {
					Size = UDim2.new(1, 0, 0, 16),
					BackgroundTransparency = 1,
				}, {
					Layout = e("UIListLayout", {
						VerticalAlignment = Enum.VerticalAlignment.Center,
						FillDirection = Enum.FillDirection.Horizontal,
						SortOrder = Enum.SortOrder.LayoutOrder,
						Padding = UDim.new(0, 5),
					}),
					Tag = if self.props.tag and TAG_TYPES[self.props.tag]
						then e(Tag, {
							layoutOrder = 1,
							transparency = self.props.transparency,
							text = TAG_TYPES[self.props.tag].text,
							icon = TAG_TYPES[self.props.tag].icon,
							color = getThemeColorFromPath(theme, TAG_TYPES[self.props.tag].color),
						})
						else nil,
					Name = e("TextLabel", {
						Text = self.props.name,
						Font = Enum.Font.GothamBold,
						TextSize = 16,
						TextColor3 = if self.props.tag and TAG_TYPES[self.props.tag]
							then getThemeColorFromPath(theme, TAG_TYPES[self.props.tag].color)
							else settingsTheme.Setting.NameColor,
						TextXAlignment = Enum.TextXAlignment.Left,
						TextTransparency = self.props.transparency,
						RichText = true,

						Size = UDim2.new(1, 0, 0, 16),

						LayoutOrder = 2,
						BackgroundTransparency = 1,
					}),
				}),

				Description = e("TextLabel", {
					Text = self.props.description,
					Font = Enum.Font.Gotham,
					LineHeight = 1.2,
					TextSize = 14,
					TextColor3 = settingsTheme.Setting.DescriptionColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextTransparency = self.props.transparency,
					TextWrapped = true,
					RichText = true,

					Size = Roact.joinBindings({
						containerSize = self.containerSize,
						inputSize = self.inputSize,
					}):map(function(values)
						local offset = values.inputSize.X + 5
						local textBounds = getTextBounds(
							self.props.description,
							14,
							Enum.Font.Gotham,
							1.2,
							Vector2.new(values.containerSize.X - offset, math.huge)
						)
						return UDim2.new(1, -offset, 0, textBounds.Y)
					end),

					LayoutOrder = 3,
					BackgroundTransparency = 1,
				}),

				Layout = e("UIListLayout", {
					VerticalAlignment = Enum.VerticalAlignment.Center,
					FillDirection = Enum.FillDirection.Vertical,
					SortOrder = Enum.SortOrder.LayoutOrder,
					Padding = UDim.new(0, 5),

					[Roact.Change.AbsoluteContentSize] = function(object)
						self.setContentSize(object.AbsoluteContentSize)
					end,
				}),
			}),

			Divider = e("Frame", {
				BackgroundColor3 = settingsTheme.DividerColor,
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
end

return Setting
