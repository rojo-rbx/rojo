local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local Assets = require(Plugin.Assets)
local Theme = require(Plugin.Components.Theme)
local FitList = require(Plugin.Components.FitList)
local FitText = require(Plugin.Components.FitText)

local e = Roact.createElement

local RoundBox = Assets.Slices.RoundBox

local function FormButton(props)
	local text = props.text
	local layoutOrder = props.layoutOrder
	local onClick = props.onClick

	local textColor
	local backgroundColor

	return Theme.with(function(theme)
		if props.secondary then
			textColor = theme.Brand1
			backgroundColor = theme.Background2
		else
			textColor = theme.TextOnAccent
			backgroundColor = theme.Brand1
		end

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
				ImageColor3 = backgroundColor,

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
				TextSize = 18,
				TextColor3 = textColor,
				Font = theme.ButtonFont,
				Padding = Vector2.new(16, 8),
				BackgroundTransparency = 1,
			}),
		})
	end)
end

return FormButton