local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Theme = require(Plugin.App.Theme)
local Assets = require(Plugin.Assets)
local Config = require(Plugin.Config)
local Version = require(Plugin.Version)

local Tooltip = require(Plugin.App.Components.Tooltip)
local SlicedImage = require(script.Parent.SlicedImage)

local e = Roact.createElement

local function VersionIndicator(props)
	local updateMessage = Version.getUpdateMessage()

	return Theme.with(function(theme)
		return e("Frame", {
			LayoutOrder = props.layoutOrder,
			Size = UDim2.new(0, 0, 0, 25),
			BackgroundTransparency = 1,
			AutomaticSize = Enum.AutomaticSize.X,
		}, {
			Border = if updateMessage
				then e(SlicedImage, {
					slice = Assets.Slices.RoundedBorder,
					color = theme.Button.Bordered.Enabled.BorderColor,
					transparency = props.transparency,
					size = UDim2.fromScale(1, 1),
					zIndex = 0,
				}, {
					Indicator = e("ImageLabel", {
						Size = UDim2.new(0, 10, 0, 10),
						ScaleType = Enum.ScaleType.Fit,
						Image = Assets.Images.Circles[16],
						ImageColor3 = theme.Header.LogoColor,
						ImageTransparency = props.transparency,
						BackgroundTransparency = 1,
						Position = UDim2.new(1, 0, 0, 0),
						AnchorPoint = Vector2.new(0.5, 0.5),
					}),
				})
				else nil,

			Tip = if updateMessage
				then e(Tooltip.Trigger, {
					text = updateMessage,
					delay = 0.1,
				})
				else nil,

			VersionText = e("TextLabel", {
				Text = Version.display(Config.version),
				FontFace = theme.Font.Thin,
				TextSize = theme.TextSize.Body,
				TextColor3 = theme.Header.VersionColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = props.transparency,
				BackgroundTransparency = 1,

				Size = UDim2.new(0, 0, 1, 0),
				AutomaticSize = Enum.AutomaticSize.X,
			}, {
				Padding = e("UIPadding", {
					PaddingLeft = UDim.new(0, 6),
					PaddingRight = UDim.new(0, 6),
				}),
			}),
		})
	end)
end

local function Header(props)
	return Theme.with(function(theme)
		return e("Frame", {
			Size = UDim2.new(1, 0, 0, 32),
			LayoutOrder = props.layoutOrder,
			BackgroundTransparency = 1,
		}, {
			Logo = e("ImageLabel", {
				Image = Assets.Images.Logo,
				ImageColor3 = theme.Header.LogoColor,
				ImageTransparency = props.transparency,

				Size = UDim2.new(0, 60, 0, 27),

				LayoutOrder = 1,
				BackgroundTransparency = 1,
			}),

			VersionIndicator = e(VersionIndicator, {
				transparency = props.transparency,
				layoutOrder = 2,
			}),

			Layout = e("UIListLayout", {
				VerticalAlignment = Enum.VerticalAlignment.Center,
				FillDirection = Enum.FillDirection.Horizontal,
				SortOrder = Enum.SortOrder.LayoutOrder,
				Padding = UDim.new(0, 15),
			}),
		})
	end)
end

return Header
