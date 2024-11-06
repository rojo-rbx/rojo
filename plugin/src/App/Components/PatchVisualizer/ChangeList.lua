local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Assets = require(Plugin.Assets)
local Theme = require(Plugin.App.Theme)
local ScrollingFrame = require(Plugin.App.Components.ScrollingFrame)
local BorderedContainer = require(Plugin.App.Components.BorderedContainer)
local DisplayValue = require(script.Parent.DisplayValue)

local EMPTY_TABLE = {}

local e = Roact.createElement

local function ViewDiffButton(props)
	return Theme.with(function(theme)
		return e("TextButton", {
			Text = "",
			Size = UDim2.new(0.7, 0, 1, -4),
			LayoutOrder = 2,
			BackgroundTransparency = 1,
			[Roact.Event.Activated] = props.onClick,
		}, {
			e(BorderedContainer, {
				size = UDim2.new(1, 0, 1, 0),
				transparency = props.transparency:map(function(t)
					return 0.5 + (0.5 * t)
				end),
			}, {
				Layout = e("UIListLayout", {
					FillDirection = Enum.FillDirection.Horizontal,
					SortOrder = Enum.SortOrder.LayoutOrder,
					HorizontalAlignment = Enum.HorizontalAlignment.Center,
					VerticalAlignment = Enum.VerticalAlignment.Center,
					Padding = UDim.new(0, 5),
				}),
				Label = e("TextLabel", {
					Text = "View Diff",
					BackgroundTransparency = 1,
					Font = Enum.Font.GothamMedium,
					TextSize = 14,
					TextColor3 = theme.Settings.Setting.DescriptionColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextTransparency = props.transparency,
					TextTruncate = Enum.TextTruncate.AtEnd,
					Size = UDim2.new(0, 65, 1, 0),
					LayoutOrder = 1,
				}),
				Icon = e("ImageLabel", {
					Image = Assets.Images.Icons.Expand,
					ImageColor3 = theme.Settings.Setting.DescriptionColor,
					ImageTransparency = props.transparency,

					Size = UDim2.new(0, 16, 0, 16),
					Position = UDim2.new(0.5, 0, 0.5, 0),
					AnchorPoint = Vector2.new(0.5, 0.5),

					BackgroundTransparency = 1,
					LayoutOrder = 2,
				}),
			}),
		})
	end)
end

local function RowContent(props)
	local values = props.values
	local metadata = props.metadata

	if props.showStringDiff and values[1] == "Source" then
		-- Special case for .Source updates
		return e(ViewDiffButton, {
			transparency = props.transparency,
			onClick = function()
				if not props.showStringDiff then
					return
				end
				props.showStringDiff(tostring(values[2]), tostring(values[3]))
			end,
		})
	end

	if props.showTableDiff and (type(values[2]) == "table" or type(values[3]) == "table") then
		-- Special case for table properties (like Attributes/Tags)
		return e(ViewDiffButton, {
			transparency = props.transparency,
			onClick = function()
				if not props.showTableDiff then
					return
				end
				props.showTableDiff(values[2], values[3])
			end,
		})
	end

	return Theme.with(function(theme)
		return Roact.createFragment({
			ColumnB = e(
				"Frame",
				{
					BackgroundTransparency = 1,
					Size = UDim2.new(0.35, 0, 1, 0),
					LayoutOrder = 2,
				},
				e(DisplayValue, {
					value = values[2],
					transparency = props.transparency,
					textColor = if metadata.isWarning
						then theme.Diff.Warning
						else theme.Settings.Setting.DescriptionColor,
				})
			),
			ColumnC = e(
				"Frame",
				{
					BackgroundTransparency = 1,
					Size = UDim2.new(0.35, 0, 1, 0),
					LayoutOrder = 3,
				},
				e(DisplayValue, {
					value = values[3],
					transparency = props.transparency,
					textColor = if metadata.isWarning
						then theme.Diff.Warning
						else theme.Settings.Setting.DescriptionColor,
				})
			),
		})
	end)
end

local ChangeList = Roact.Component:extend("ChangeList")

function ChangeList:init()
	self.contentSize, self.setContentSize = Roact.createBinding(Vector2.new(0, 0))
end

function ChangeList:render()
	return Theme.with(function(theme)
		local props = self.props
		local changes = props.changes

		-- Color alternating rows for readability
		local rowTransparency = props.transparency:map(function(t)
			return 0.93 + (0.07 * t)
		end)

		local rows = {}
		local pad = {
			PaddingLeft = UDim.new(0, 5),
			PaddingRight = UDim.new(0, 5),
		}

		local headerRow = changes[1]
		local headers = e("Frame", {
			Size = UDim2.new(1, 0, 0, 24),
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
			ColumnA = e("TextLabel", {
				Text = tostring(headerRow[1]),
				BackgroundTransparency = 1,
				Font = Enum.Font.GothamBold,
				TextSize = 14,
				TextColor3 = theme.TextColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = props.transparency,
				TextTruncate = Enum.TextTruncate.AtEnd,
				Size = UDim2.new(0.3, 0, 1, 0),
				LayoutOrder = 1,
			}),
			ColumnB = e("TextLabel", {
				Text = tostring(headerRow[2]),
				BackgroundTransparency = 1,
				Font = Enum.Font.GothamBold,
				TextSize = 14,
				TextColor3 = theme.TextColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = props.transparency,
				TextTruncate = Enum.TextTruncate.AtEnd,
				Size = UDim2.new(0.35, 0, 1, 0),
				LayoutOrder = 2,
			}),
			ColumnC = e("TextLabel", {
				Text = tostring(headerRow[3]),
				BackgroundTransparency = 1,
				Font = Enum.Font.GothamBold,
				TextSize = 14,
				TextColor3 = theme.TextColor,
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

			local metadata = values[4] or EMPTY_TABLE
			local isWarning = metadata.isWarning

			rows[row] = e("Frame", {
				Size = UDim2.new(1, 0, 0, 24),
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
				ColumnA = e("TextLabel", {
					Text = (if isWarning then "⚠ " else "") .. tostring(values[1]),
					BackgroundTransparency = 1,
					Font = Enum.Font.GothamMedium,
					TextSize = 14,
					TextColor3 = if isWarning then theme.Diff.Warning else theme.TextColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextTransparency = props.transparency,
					TextTruncate = Enum.TextTruncate.AtEnd,
					Size = UDim2.new(0.3, 0, 1, 0),
					LayoutOrder = 1,
				}),
				Content = e(RowContent, {
					values = values,
					metadata = metadata,
					transparency = props.transparency,
					showStringDiff = props.showStringDiff,
					showTableDiff = props.showTableDiff,
				}),
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
				size = UDim2.new(1, 0, 1, -24),
				position = UDim2.new(0, 0, 0, 24),
				contentSize = self.contentSize,
				transparency = props.transparency,
			}, rows),
		})
	end)
end

return ChangeList
