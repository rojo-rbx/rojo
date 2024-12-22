local TextService = game:GetService("TextService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Assets = require(Plugin.Assets)
local Theme = require(Plugin.App.Theme)
local getTextBoundsAsync = require(Plugin.App.getTextBoundsAsync)

local SlicedImage = require(Plugin.App.Components.SlicedImage)

local e = Roact.createElement

local DIVIDER_FADE_SIZE = 0.1

local Listing = Roact.Component:extend("Listing")

function Listing:init()
	self.contentSize, self.setContentSize = Roact.createBinding(Vector2.new(0, 0))
	self.containerSize, self.setContainerSize = Roact.createBinding(Vector2.new(0, 0))
end

function Listing:render()
	return Theme.with(function(theme)
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
				Button = e(SlicedImage, {
					slice = Assets.Slices.RoundedBorder,
					color = theme.Checkbox.Inactive.BorderColor,
					transparency = self.props.transparency,
					size = UDim2.new(1, 0, 1, 0),
				}, {
					Icon = e("ImageLabel", {
						Image = Assets.Images.Icons.Settings,
						ImageColor3 = theme.Notification.InfoColor,
						ImageTransparency = self.props.transparency,

						Size = UDim2.new(0, 16, 0, 16),
						Position = UDim2.new(0.5, 0, 0.5, 0),
						AnchorPoint = Vector2.new(0.5, 0.5),

						BackgroundTransparency = 1,
					}),
				}),
			}),

			Text = e("Frame", {
				Size = UDim2.new(1, 0, 1, 0),
				BackgroundTransparency = 1,
			}, {
				Name = e("TextLabel", {
					Text = self.props.name,
					FontFace = theme.Font.Bold,
					TextSize = theme.TextSize.Medium,
					TextColor3 = theme.Settings.Setting.NameColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextTransparency = self.props.transparency,

					Size = UDim2.new(1, 0, 0, 17),

					LayoutOrder = 1,
					BackgroundTransparency = 1,
				}),

				Description = e("TextLabel", {
					Text = self.props.description,
					FontFace = theme.Font.Thin,
					LineHeight = 1.2,
					TextSize = theme.TextSize.Body,
					TextColor3 = theme.Settings.Setting.DescriptionColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextTransparency = self.props.transparency,
					TextWrapped = true,

					Size = self.containerSize:map(function(value)
						local textBounds = getTextBoundsAsync(
							self.props.description,
							theme.Font.Thin,
							theme.TextSize.Body,
							value.X - 40,
							false,
							1.2
						)
						return UDim2.new(1, -40, 0, textBounds.Y)
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
				BackgroundColor3 = theme.Settings.DividerColor,
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
