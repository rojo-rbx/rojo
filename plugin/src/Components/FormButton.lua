local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local Assets = require(Plugin.Assets)
local FitList = require(Plugin.Components.FitList)
local FitText = require(Plugin.Components.FitText)

local e = Roact.createElement

local RoundBox = Assets.Slices.RoundBox

local function FormButton(props)
	local text = props.text
	local layoutOrder = props.layoutOrder
	local onClick = props.onClick

	local imageColor = props.secondary and Color3.new(0.95, 0.95, 0.95) or nil

	return e(FitList, {
		containerKind = "ImageButton",
		containerProps = {
			LayoutOrder = layoutOrder,
			BackgroundTransparency = 1,
			Image = RoundBox.asset,
			ImageRectOffset = RoundBox.offset,
			ImageRectSize = RoundBox.size,
			SliceCenter = RoundBox.center,
			ScaleType = Enum.ScaleType.Slice,
			ImageColor3 = imageColor,

			[Roact.Event.Activated] = function()
				if onClick ~= nil then
					onClick()
				end
			end,
		},
	}, {
		Text = e(FitText, {
			Kind = "TextLabel",
			Text = text,
			TextSize = 22,
			Font = Enum.Font.SourceSansBold,
			Padding = Vector2.new(14, 6),
			TextColor3 = Color3.new(0.05, 0.05, 0.05),
			BackgroundTransparency = 1,
		}),
	})
end

return FormButton