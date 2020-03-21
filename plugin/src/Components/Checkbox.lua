local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local Theme = require(Plugin.Components.Theme)

local e = Roact.createElement

local function Checkbox(props)
	local checked = props.checked
	local layoutOrder = props.layoutOrder
	local onChange = props.onChange

	return Theme.with(function(theme)
		return e("ImageButton", {
			LayoutOrder = layoutOrder,
			Size = UDim2.new(0, 20, 0, 20),
			BorderSizePixel = 2,
			BorderColor3 = theme.Text2,
			BackgroundColor3 = theme.Background2,

			[Roact.Event.Activated] = function()
				onChange(not checked)
			end,
		}, {
			Indicator = e("Frame", {
				Size = UDim2.new(0, 18, 0, 18),
				Position = UDim2.new(0.5, 0, 0.5, 0),
				AnchorPoint = Vector2.new(0.5, 0.5),
				BorderSizePixel = 0,
				BackgroundColor3 = theme.Brand1,
				BackgroundTransparency = checked and 0 or 1,
			})
		})
	end)
end

return Checkbox