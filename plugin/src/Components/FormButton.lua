local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local Assets = require(Plugin.Assets)
local FitList = require(Plugin.Components.FitList)
local FitText = require(Plugin.Components.FitText)

local e = Roact.createElement

local GrayButton07 = Assets.Slices.GrayButton07

local function FormButton(props)
	local text = props.text
	local layoutOrder = props.layoutOrder
	local onClick = props.onClick

	return e(FitList, {
		containerKind = "ImageButton",
		containerProps = {
			LayoutOrder = layoutOrder,
			BackgroundTransparency = 1,
			Image = GrayButton07.asset,
			ImageRectOffset = GrayButton07.offset,
			ImageRectSize = GrayButton07.size,
			ScaleType = Enum.ScaleType.Slice,
			SliceCenter = GrayButton07.center,

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