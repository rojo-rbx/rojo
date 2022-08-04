local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Assets = require(Plugin.Assets)
local Theme = require(Plugin.App.Theme)
local bindingUtil = require(Plugin.App.bindingUtil)

local e = Roact.createElement

local function ScrollingFrame(props)
	return Theme.with(function(theme)
		return e("ScrollingFrame", {
			ScrollBarThickness = 9,
			ScrollBarImageColor3 = theme.ScrollBarColor,
			ScrollBarImageTransparency = props.transparency:map(function(value)
				return bindingUtil.blendAlpha({ 0.65, value })
			end),
			TopImage = Assets.Images.ScrollBar.Top,
			MidImage = Assets.Images.ScrollBar.Middle,
			BottomImage = Assets.Images.ScrollBar.Bottom,

			ElasticBehavior = Enum.ElasticBehavior.Always,
			ScrollingDirection = Enum.ScrollingDirection.Y,

			Size = props.size,
			Position = props.position,
			AnchorPoint = props.anchorPoint,
			CanvasSize = props.contentSize:map(function(value)
				return UDim2.new(0, 0, 0, value.Y)
			end),

			BorderSizePixel = 0,
			BackgroundTransparency = 1,

			[Roact.Change.AbsoluteSize] = props[Roact.Change.AbsoluteSize]
		}, props[Roact.Children])
	end)
end

return ScrollingFrame
