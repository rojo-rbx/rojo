local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Theme = require(Plugin.App.Theme)
local Assets = require(Plugin.Assets)
local bindingUtil = require(Plugin.App.bindingUtil)

local SlicedImage = require(script.Parent.SlicedImage)

local e = Roact.createElement

local function BorderedContainer(props)
	return Theme.with(function(theme)
		local backgroundTransparency = props.transparency:map(function(value)
			return bindingUtil.blendAlpha({ 0.1, value })
		end)
		local borderTransparency = props.transparency:map(function(value)
			return bindingUtil.blendAlpha({ 0.18, value })
		end)
		local shadowTransparency = props.transparency:map(function(value)
			return bindingUtil.blendAlpha({ 0.72, value })
		end)

		return e(SlicedImage, {
			slice = Assets.Slices.RoundedBackground,
			color = props.backgroundColor or theme.BorderedContainer.BackgroundColor,
			transparency = backgroundTransparency,

			size = props.size,
			position = props.position,
			anchorPoint = props.anchorPoint,
			layoutOrder = props.layoutOrder,
		}, {
			Shadow = e(SlicedImage, {
				slice = Assets.Slices.RoundedBackground,
				color = Color3.new(0, 0, 0),
				transparency = shadowTransparency,
				position = UDim2.fromOffset(0, 2),
				size = UDim2.fromScale(1, 1),
				zIndex = -2,
			}),
			Content = e("Frame", {
				Size = UDim2.new(1, -2, 1, -2),
				Position = UDim2.new(0, 1, 0, 1),
				BackgroundTransparency = 1,
				ZIndex = 2,
			}, props[Roact.Children]),

			Border = e(SlicedImage, {
				slice = Assets.Slices.RoundedBorder,
				color = props.borderColor or theme.BorderedContainer.BorderColor,
				transparency = borderTransparency,

				size = UDim2.new(1, 0, 1, 0),
			}),
		})
	end)
end

return BorderedContainer
