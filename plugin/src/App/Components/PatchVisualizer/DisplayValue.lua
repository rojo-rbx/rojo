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
					BackgroundTransparency = props.transparency,
					BackgroundColor3 = props.value,
					Size = UDim2.new(0, 20, 0, 20),
					Position = UDim2.new(0, 0, 0.5, 0),
					AnchorPoint = Vector2.new(0, 0.5),
				}, {
					Corner = e("UICorner", {
						CornerRadius = UDim.new(0, 4),
					}),
					Stroke = e("UIStroke", {
						Color = theme.BorderedContainer.BorderColor,
						Transparency = props.transparency,
					}),
				}),
				Label = e("TextLabel", {
					Text = string.format("%d,%d,%d", props.value.R * 255, props.value.G * 255, props.value.B * 255),
					BackgroundTransparency = 1,
					Font = Enum.Font.GothamMedium,
					TextSize = 14,
					TextColor3 = props.textColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextTransparency = props.transparency,
					TextTruncate = Enum.TextTruncate.AtEnd,
					Size = UDim2.new(1, -25, 1, 0),
					Position = UDim2.new(0, 25, 0, 0),
				}),
			})

		elseif t == "table" then
			-- Showing a memory address for tables is useless, so we want to show the best we can
			local textRepresentation = nil

			local meta = getmetatable(props.value)
			if meta and meta.__tostring then
				-- If the table has a tostring metamethod, use that
				textRepresentation = tostring(props.value)
			elseif next(props.value) == nil then
				-- If it's empty, show empty braces
				textRepresentation = "{}"
			else
				-- If it has children, list them out
				local out, i = {}, 0
				for k, v in pairs(props.value) do
					i += 1

					-- Wrap strings in quotes
					if type(k) == "string" then
						k = "\"" .. k .. "\""
					end
					if type(v) == "string" then
						v = "\"" .. v .. "\""
					end

					out[i] = string.format("[%s] = %s", tostring(k), tostring(v))
				end
				textRepresentation = "{ " .. table.concat(out, ", ") .. " }"
			end

			return e("TextLabel", {
				Text = textRepresentation,
				BackgroundTransparency = 1,
				Font = Enum.Font.GothamMedium,
				TextSize = 14,
				TextColor3 = props.textColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = props.transparency,
				TextTruncate = Enum.TextTruncate.AtEnd,
				Size = UDim2.new(1, 0, 1, 0),
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
			TextColor3 = props.textColor,
			TextXAlignment = Enum.TextXAlignment.Left,
			TextTransparency = props.transparency,
			TextTruncate = Enum.TextTruncate.AtEnd,
			Size = UDim2.new(1, 0, 1, 0),
		})
	end)
end

return DisplayValue
