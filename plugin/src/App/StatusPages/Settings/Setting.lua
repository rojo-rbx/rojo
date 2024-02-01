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

local e = Roact.createElement

local DIVIDER_FADE_SIZE = 0.1

local function getTextBounds(text, textSize, font, lineHeight, bounds)
	local textBounds = TextService:GetTextSize(text, textSize, font, bounds)

	local lineCount = textBounds.Y / textSize
	local lineHeightAbsolute = textSize * lineHeight

	return Vector2.new(textBounds.X, lineHeightAbsolute * lineCount - (lineHeightAbsolute - textSize))
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
		theme = theme.Settings

		return e("Frame", {
			Size = self.contentSize:map(function(value)
				return UDim2.new(1, 0, 0, 20 + value.Y + 20)
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
						color = theme.BackButtonColor,
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
				Name = e("TextLabel", {
					Text = (
						if self.props.experimental
							then '<font color="#FF8E3C">⚠ </font>'
							elseif
								self.props.developerDebug
							then '<font family="rbxasset://fonts/families/Guru.json" color="#35B5FF">⚑ </font>' -- Guru is the only font with the flag emoji
							else ""
					) .. self.props.name,
					Font = Enum.Font.GothamBold,
					TextSize = 17,
					TextColor3 = theme.Setting.NameColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextTransparency = self.props.transparency,
					RichText = true,

					Size = UDim2.new(1, 0, 0, 17),

					LayoutOrder = 1,
					BackgroundTransparency = 1,
				}),

				Description = e("TextLabel", {
					Text = (if self.props.experimental
						then '<font color="#FF8E3C">[Experimental] </font>'
						elseif self.props.developerDebug then '<font color="#35B5FF">[Dev Debug] </font>'
						else "") .. self.props.description,
					Font = Enum.Font.Gotham,
					LineHeight = 1.2,
					TextSize = 14,
					TextColor3 = theme.Setting.DescriptionColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextTransparency = self.props.transparency,
					TextWrapped = true,
					RichText = true,

					Size = Roact.joinBindings({
						containerSize = self.containerSize,
						inputSize = self.inputSize,
					}):map(function(values)
						local desc = (if self.props.experimental then "[Experimental] " else "")
							.. self.props.description
						local offset = values.inputSize.X + 5
						local textBounds = getTextBounds(
							desc,
							14,
							Enum.Font.Gotham,
							1.2,
							Vector2.new(values.containerSize.X - offset, math.huge)
						)
						return UDim2.new(1, -offset, 0, textBounds.Y)
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
end

return Setting
