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
local SlicedImage = require(Plugin.App.Components.SlicedImage)

local e = Roact.createElement

local DIVIDER_FADE_SIZE = 0.1

local function getTextBounds(text, textSize, font, lineHeight, bounds)
	local textBounds = TextService:GetTextSize(text, textSize, font, bounds)

	local lineCount = textBounds.Y / textSize
	local lineHeightAbsolute = textSize * lineHeight

	return Vector2.new(textBounds.X, lineHeightAbsolute * lineCount - (lineHeightAbsolute - textSize))
end

local Listing = Roact.Component:extend("Listing")

function Listing:init()
	self.contentSize, self.setContentSize = Roact.createBinding(Vector2.new(0, 0))
	self.containerSize, self.setContainerSize = Roact.createBinding(Vector2.new(0, 0))
end

function Listing:render()
	return Theme.with(function(theme)
		theme = theme.Settings

		return e("Frame", {
			Size = self.contentSize:map(function(value)
				return UDim2.new(1, 0, 0, 20 + value.Y + 20)
			end),
			LayoutOrder = self.props.layoutOrder,
			ZIndex = -self.props.layoutOrder,
			BackgroundTransparency = 1,

			[Roact.Change.AbsoluteSize] = function(object)
				self.setContainerSize(object.AbsoluteSize)
			end,
		}, {
			Settings = e("TextButton", {
				Text = "",
				BackgroundTransparency = 1,
				Size = UDim2.fromOffset(28, 28),
				Position = UDim2.fromScale(1, 0.5),
				AnchorPoint = Vector2.new(1, 0.5),

				[Roact.Event.Activated] = function()
					self.props.onClick()
				end,
			}, {
				Button = Theme.with(function(theme)
					theme = theme.Checkbox
					return e(SlicedImage, {
						slice = Assets.Slices.RoundedBorder,
						color = theme.Inactive.BorderColor,
						transparency = self.props.transparency,
						size = UDim2.new(1, 0, 1, 0),
					}, {
						Icon = e("ImageLabel", {
							Image = Assets.Images.Icons.Settings,
							ImageColor3 = theme.Active.IconColor,
							ImageTransparency = self.props.transparency,

							Size = UDim2.new(0, 16, 0, 16),
							Position = UDim2.new(0.5, 0, 0.5, 0),
							AnchorPoint = Vector2.new(0.5, 0.5),

							BackgroundTransparency = 1,
						}),
					})
				end),
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
							Vector2.new(value.X - 40, math.huge)
						)
						return UDim2.new(1, - 40, 0, textBounds.Y)
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

return Listing
