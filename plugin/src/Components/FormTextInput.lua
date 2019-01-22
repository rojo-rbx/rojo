local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local Assets = require(Plugin.Assets)
local Theme = require(Plugin.Theme)

local e = Roact.createElement

local RoundBox = Assets.Slices.RoundBox

local TEXT_SIZE = 22
local PADDING = 8

local function FormTextInput(props)
	local value = props.value
	local placeholderValue = props.placeholderValue
	local onValueChange = props.onValueChange
	local layoutOrder = props.layoutOrder
	local width = props.width

	return e("ImageLabel", {
		LayoutOrder = layoutOrder,
		Image = RoundBox.asset,
		ImageRectOffset = RoundBox.offset,
		ImageRectSize = RoundBox.size,
		ScaleType = Enum.ScaleType.Slice,
		SliceCenter = RoundBox.center,
		ImageColor3 = Theme.SecondaryColor,
		Size = UDim2.new(width.Scale, width.Offset, 0, TEXT_SIZE + PADDING * 2),
		BackgroundTransparency = 1,
	}, {
		InputInner = e("TextBox", {
			BackgroundTransparency = 1,
			Size = UDim2.new(1, -PADDING * 2, 1, -PADDING * 2),
			Position = UDim2.new(0.5, 0, 0.5, 0),
			AnchorPoint = Vector2.new(0.5, 0.5),
			Font = Theme.InputFont,
			ClearTextOnFocus = false,
			TextXAlignment = Enum.TextXAlignment.Center,
			TextSize = TEXT_SIZE,
			Text = value,
			PlaceholderText = placeholderValue,
			PlaceholderColor3 = Theme.AccentLightColor,
			TextColor3 = Theme.AccentColor,

			[Roact.Change.Text] = function(rbx)
				onValueChange(rbx.Text)
			end,
		}),
	})
end

return FormTextInput