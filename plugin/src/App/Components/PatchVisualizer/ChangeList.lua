local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Theme = require(Plugin.App.Theme)
local ScrollingFrame = require(Plugin.App.Components.ScrollingFrame)
local DisplayValue = require(script.Parent.DisplayValue)

local e = Roact.createElement

local ChangeList = Roact.Component:extend("ChangeList")

function ChangeList:init()
	self.contentSize, self.setContentSize = Roact.createBinding(Vector2.new(0, 0))
end

function ChangeList:render()
	return Theme.with(function(theme)
		local props = self.props
		local changes = props.changes
		local columnVisibility = props.columnVisibility

		-- Color alternating rows for readability
		local rowTransparency = props.transparency:map(function(t)
			return 0.93 + (0.07 * t)
		end)

		local rows = {}
		local pad = {
			PaddingLeft = UDim.new(0, 5),
			PaddingRight = UDim.new(0, 5),
		}

		local headers = e("Frame", {
			Size = UDim2.new(1, 0, 0, 30),
			BackgroundTransparency = rowTransparency,
			BackgroundColor3 = theme.Diff.Row,
			LayoutOrder = 0,
		}, {
			Padding = e("UIPadding", pad),
			Layout = e("UIListLayout", {
				FillDirection = Enum.FillDirection.Horizontal,
				SortOrder = Enum.SortOrder.LayoutOrder,
				HorizontalAlignment = Enum.HorizontalAlignment.Left,
				VerticalAlignment = Enum.VerticalAlignment.Center,
			}),
			A = e("TextLabel", {
				Visible = columnVisibility[1],
				Text = tostring(changes[1][1]),
				BackgroundTransparency = 1,
				Font = Enum.Font.GothamBold,
				TextSize = 14,
				TextColor3 = theme.Settings.Setting.DescriptionColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = props.transparency,
				TextTruncate = Enum.TextTruncate.AtEnd,
				Size = UDim2.new(0.3, 0, 1, 0),
				LayoutOrder = 1,
			}),
			B = e("TextLabel", {
				Visible = columnVisibility[2],
				Text = tostring(changes[1][2]),
				BackgroundTransparency = 1,
				Font = Enum.Font.GothamBold,
				TextSize = 14,
				TextColor3 = theme.Settings.Setting.DescriptionColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = props.transparency,
				TextTruncate = Enum.TextTruncate.AtEnd,
				Size = UDim2.new(0.35, 0, 1, 0),
				LayoutOrder = 2,
			}),
			C = e("TextLabel", {
				Visible = columnVisibility[3],
				Text = tostring(changes[1][3]),
				BackgroundTransparency = 1,
				Font = Enum.Font.GothamBold,
				TextSize = 14,
				TextColor3 = theme.Settings.Setting.DescriptionColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = props.transparency,
				TextTruncate = Enum.TextTruncate.AtEnd,
				Size = UDim2.new(0.35, 0, 1, 0),
				LayoutOrder = 3,
			}),
		})

		for row, values in changes do
			if row == 1 then
				continue -- Skip headers, already handled above
			end

			local metadata = values[4]
			local isWarning = metadata.isWarning

			rows[row] = e("Frame", {
				Size = UDim2.new(1, 0, 0, 30),
				BackgroundTransparency = row % 2 ~= 0 and rowTransparency or 1,
				BackgroundColor3 = theme.Diff.Row,
				BorderSizePixel = 0,
				LayoutOrder = row,
			}, {
				Padding = e("UIPadding", pad),
				Layout = e("UIListLayout", {
					FillDirection = Enum.FillDirection.Horizontal,
					SortOrder = Enum.SortOrder.LayoutOrder,
					HorizontalAlignment = Enum.HorizontalAlignment.Left,
					VerticalAlignment = Enum.VerticalAlignment.Center,
				}),
				A = e("TextLabel", {
					Visible = columnVisibility[1],
					Text = (if isWarning then "⚠ " else "") .. tostring(values[1]),
					BackgroundTransparency = 1,
					Font = Enum.Font.GothamMedium,
					TextSize = 14,
					TextColor3 = if isWarning then theme.Diff.Warning else theme.Settings.Setting.DescriptionColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextTransparency = props.transparency,
					TextTruncate = Enum.TextTruncate.AtEnd,
					Size = UDim2.new(0.3, 0, 1, 0),
					LayoutOrder = 1,
				}),
				B = e(
					"Frame",
					{
						Visible = columnVisibility[2],
						BackgroundTransparency = 1,
						Size = UDim2.new(0.35, 0, 1, 0),
						LayoutOrder = 2,
					},
					e(DisplayValue, {
						value = values[2],
						transparency = props.transparency,
						textColor = if isWarning then theme.Diff.Warning else theme.Settings.Setting.DescriptionColor,
					})
				),
				C = e(
					"Frame",
					{
						Visible = columnVisibility[3],
						BackgroundTransparency = 1,
						Size = UDim2.new(0.35, 0, 1, 0),
						LayoutOrder = 3,
					},
					e(DisplayValue, {
						value = values[3],
						transparency = props.transparency,
						textColor = if isWarning then theme.Diff.Warning else theme.Settings.Setting.DescriptionColor,
					})
				),
			})
		end

		table.insert(
			rows,
			e("UIListLayout", {
				FillDirection = Enum.FillDirection.Vertical,
				SortOrder = Enum.SortOrder.LayoutOrder,
				HorizontalAlignment = Enum.HorizontalAlignment.Right,
				VerticalAlignment = Enum.VerticalAlignment.Top,

				[Roact.Change.AbsoluteContentSize] = function(object)
					self.setContentSize(object.AbsoluteContentSize)
				end,
			})
		)

		return e("Frame", {
			Size = UDim2.new(1, 0, 1, 0),
			BackgroundTransparency = 1,
		}, {
			Headers = headers,
			Values = e(ScrollingFrame, {
				size = UDim2.new(1, 0, 1, -30),
				position = UDim2.new(0, 0, 0, 30),
				contentSize = self.contentSize,
				transparency = props.transparency,
			}, rows),
		})
	end)
end

return ChangeList
