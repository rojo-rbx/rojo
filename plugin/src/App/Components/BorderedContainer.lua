local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Theme = require(Plugin.App.Theme)
local Assets = require(Plugin.Assets)

local SlicedImage = require(script.Parent.SlicedImage)

local e = Roact.createElement

local function BorderedContainer(props)
	return Theme.with(function(theme)
		return e(SlicedImage, {
			slice = Assets.Slices.RoundedBackground,
			color = theme.BorderedContainer.BackgroundColor,
			transparency = props.transparency,

			size = props.size,
			position = props.position,
			anchorPoint = props.anchorPoint,
			layoutOrder = props.layoutOrder,
		}, {
			Content = e("Frame", {
				Size = UDim2.new(1, 0, 1, 0),
				BackgroundTransparency = 1,
				ZIndex = 2,
			}, props[Roact.Children]),

			Border = e(SlicedImage, {
				slice = Assets.Slices.RoundedBorder,
				color = theme.BorderedContainer.BorderColor,
				transparency = props.transparency,

				size = UDim2.new(1, 0, 1, 0),
			}),
		})
	end)
end

return BorderedContainer
