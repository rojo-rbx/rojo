local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Theme = require(Plugin.App.Theme)

local e = Roact.createElement

local function DisplayValue(props)
	return Theme.with(function(theme)
		local t = typeof(props.value)
		if t == "Color3" then
			-- Colors get a blot that shows the color
			return Roact.createFragment({
				Blot = e("Frame", {
					BackgroundColor3 = props.value,
					BorderColor3 = theme.BorderedContainer.BorderColor,
					Size = UDim2.new(0, 20, 0, 20),
					Position = UDim2.new(0, 0, 0.5, 0),
					AnchorPoint = Vector2.new(0, 0.5),
				}, {
					Corner = e("UICorner", {
						CornerRadius = UDim.new(0, 4),
					}),
				}),
				Label = e("TextLabel", {
					Text = string.format("%d,%d,%d", props.value.R*255,props.value.G*255,props.value.B*255),
					BackgroundTransparency = 1,
					Font = Enum.Font.GothamMedium,
					TextSize = 14,
					TextColor3 = theme.Settings.Setting.DescriptionColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextTransparency = props.transparency,
					TextTruncate = Enum.TextTruncate.AtEnd,
					Size = UDim2.new(1, -25, 1, 0),
					Position = UDim2.new(0, 25, 0, 0),
				}),
			})
		end

		-- TODO: Maybe add visualizations to other datatypes?
		-- Or special text handling tostring for some?
		-- Will add as needed, let's see what cases arise.

		return e("TextLabel", {
			Text = string.gsub(tostring(props.value), "%s", " "),
			BackgroundTransparency = 1,
			Font = Enum.Font.GothamMedium,
			TextSize = 14,
			TextColor3 = theme.Settings.Setting.DescriptionColor,
			TextXAlignment = Enum.TextXAlignment.Left,
			TextTransparency = props.transparency,
			TextTruncate = Enum.TextTruncate.AtEnd,
			Size = UDim2.new(1, 0, 1, 0),
		})
	end)
end

return DisplayValue
