local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)
local Theme = require(Plugin.App.Theme)

local ScrollingFrame = require(Plugin.App.Components.ScrollingFrame)

local e = Roact.createElement

local function displayValue(value)
	local t = type(value)
	if t == "string" then
		return string.gsub(value, "%s", " ")
	elseif t == "table" then
		local out = {"{"}
		for k,v in value do
			table.insert(out, string.format(
				"[%s] = %s",
				tostring(k), tostring(v)
			))
		end
		table.insert(out, "}")
		return table.concat(out, " ")
	elseif t == "userdata" then
		t = typeof(value)
		if t == "Color3" then
			return string.format("%d, %d, %d", 255*value.R, 255*value.G, 255*value.B)
		else
			return tostring(value)
		end
	else
		return tostring(value)
	end
end

local DiffTable = Roact.Component:extend("DiffTable")

function DiffTable:init()
	self.contentSize, self.setContentSize = Roact.createBinding(Vector2.new(0, 0))
end

function DiffTable:render()
	return Theme.with(function(theme)
		local props = self.props
		local csv = props.csv

		local rows = {}
		local pad = {
			PaddingLeft = UDim.new(0, 5),
			PaddingRight = UDim.new(0, 5),
		}

		local headers = e("Frame", {
			Size = UDim2.new(1,0,0,30),
			BackgroundTransparency = 1,
			LayoutOrder = 0,
		}, {
			Padding = e("UIPadding", pad),
			A = e("TextLabel", {
				Text = tostring(csv[1][1]),
				BackgroundTransparency = 1,
				Font = Enum.Font.GothamBold,
				TextSize = 14,
				TextColor3 = theme.Settings.Setting.DescriptionColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = props.transparency,
				TextTruncate = Enum.TextTruncate.AtEnd,
				Size = UDim2.new(0.333, 0, 1, 0),
				Position = UDim2.new(0, 0, 0, 0),
			}),
			B = e("TextLabel", {
				Text = tostring(csv[1][2]),
				BackgroundTransparency = 1,
				Font = Enum.Font.GothamBold,
				TextSize = 14,
				TextColor3 = theme.Settings.Setting.DescriptionColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = props.transparency,
				TextTruncate = Enum.TextTruncate.AtEnd,
				Size = UDim2.new(0.333, 0, 1, 0),
				Position = UDim2.new(0.333, 0, 0, 0),
			}),
			C = e("TextLabel", {
				Text = tostring(csv[1][3]),
				BackgroundTransparency = 1,
				Font = Enum.Font.GothamBold,
				TextSize = 14,
				TextColor3 = theme.Settings.Setting.DescriptionColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = props.transparency,
				TextTruncate = Enum.TextTruncate.AtEnd,
				Size = UDim2.new(0.333, 0, 1, 0),
				Position = UDim2.new(0.666, 0, 0, 0),
			}),
		})

		for row, values in csv do
			if row == 1 then continue end -- Skip headers

			rows[row] = e("Frame", {
				Size = UDim2.new(1,0,0,30),
				BackgroundTransparency = row % 2 == 0 and 0.9 or 1,
				BorderSizePixel = 0,
				LayoutOrder = row,
			}, {
				Padding = e("UIPadding", pad),
				A = e("TextLabel", {
					Text = displayValue(values[1]),
					BackgroundTransparency = 1,
					Font = Enum.Font.GothamMedium,
					TextSize = 14,
					TextColor3 = theme.Settings.Setting.DescriptionColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextTransparency = props.transparency,
					TextTruncate = Enum.TextTruncate.AtEnd,
					Size = UDim2.new(0.333, 0, 1, 0),
					Position = UDim2.new(0, 0, 0, 0),
				}),
				B = e("TextLabel", {
					Text = displayValue(values[2]),
					BackgroundTransparency = 1,
					Font = Enum.Font.GothamMedium,
					TextSize = 14,
					TextColor3 = theme.Settings.Setting.DescriptionColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextTransparency = props.transparency,
					TextTruncate = Enum.TextTruncate.AtEnd,
					Size = UDim2.new(0.333, 0, 1, 0),
					Position = UDim2.new(0.333, 0, 0, 0),
				}),
				C = e("TextLabel", {
					Text = displayValue(values[3]),
					BackgroundTransparency = 1,
					Font = Enum.Font.GothamMedium,
					TextSize = 14,
					TextColor3 = theme.Settings.Setting.DescriptionColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextTransparency = props.transparency,
					TextTruncate = Enum.TextTruncate.AtEnd,
					Size = UDim2.new(0.333, 0, 1, 0),
					Position = UDim2.new(0.666, 0, 0, 0),
				}),
			})
		end

		table.insert(rows, e("UIListLayout", {
			FillDirection = Enum.FillDirection.Vertical,
			SortOrder = Enum.SortOrder.LayoutOrder,
			HorizontalAlignment = Enum.HorizontalAlignment.Right,
			VerticalAlignment = Enum.VerticalAlignment.Top,

			[Roact.Change.AbsoluteContentSize] = function(object)
				self.setContentSize(object.AbsoluteContentSize)
			end,
		}))

		return e("Frame", {
			Size = UDim2.new(1, 0, 1, 0),
			BackgroundTransparency = 1,
		}, {
			Headers = headers,
			Values = e(ScrollingFrame, {
				size = UDim2.new(1, 0, 1, -30),
				position = UDim2.new(0, 0, 0, 30),
				contentSize = self.contentSize,
				transparency = self.props.transparency,
			}, rows),
		})
	end)
end

return DiffTable
