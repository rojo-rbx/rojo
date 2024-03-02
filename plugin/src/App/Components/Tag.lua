local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Assets = require(Plugin.Assets)

local SlicedImage = require(Plugin.App.Components.SlicedImage)

local e = Roact.createElement

return function(props)
	return e(SlicedImage, {
		slice = Assets.Slices.RoundedBackground,
		color = props.color,
		transparency = props.transparency,

		size = UDim2.new(0, 0, 0, 16),
		automaticSize = Enum.AutomaticSize.X,

		layoutOrder = props.layoutOrder,
	}, {
		Padding = e("UIPadding", {
			PaddingLeft = UDim.new(0, 4),
			PaddingRight = UDim.new(0, 4),
			PaddingTop = UDim.new(0, 2),
			PaddingBottom = UDim.new(0, 2),
		}),
		Icon = if props.icon
			then e("ImageLabel", {
				Size = UDim2.new(0, 12, 0, 12),
				Position = UDim2.new(0, 0, 0.5, 0),
				AnchorPoint = Vector2.new(0, 0.5),
				Image = props.icon,
				BackgroundTransparency = 1,
				ImageColor3 = props.textColor,
			})
			else nil,
		Text = e("TextLabel", {
			Text = props.text,
			Font = Enum.Font.GothamMedium,
			TextSize = 12,
			TextColor3 = props.textColor,
			TextXAlignment = Enum.TextXAlignment.Center,
			TextTransparency = props.transparency,
			Size = UDim2.new(0, 0, 1, 0),
			Position = UDim2.new(0, if props.icon then 15 else 0, 0, 0),
			AutomaticSize = Enum.AutomaticSize.X,
			BackgroundTransparency = 1,
		}),
	})
end
