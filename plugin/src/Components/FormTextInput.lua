local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local Assets = require(Plugin.Assets)
local Theme = require(Plugin.Components.Theme)

local e = Roact.createElement

local RoundBox = Assets.Slices.RoundBox

local TEXT_SIZE = 22
local PADDING = 8

local FormTextInput = Roact.Component:extend("FormTextInput")

function FormTextInput:init()
	self:setState({
		focused = false,
	})
end

function FormTextInput:render()
	local value = self.props.value
	local placeholderValue = self.props.placeholderValue
	local onValueChange = self.props.onValueChange
	local layoutOrder = self.props.layoutOrder
	local width = self.props.width

	local shownPlaceholder
	if self.state.focused then
		shownPlaceholder = ""
	else
		shownPlaceholder = placeholderValue
	end

	return Theme.with(function(theme)
		return e("ImageLabel", {
			LayoutOrder = layoutOrder,
			Image = RoundBox.asset,
			ImageRectOffset = RoundBox.offset,
			ImageRectSize = RoundBox.size,
			ScaleType = Enum.ScaleType.Slice,
			SliceCenter = RoundBox.center,
			ImageColor3 = theme.Background2,
			Size = UDim2.new(width.Scale, width.Offset, 0, TEXT_SIZE + PADDING * 2),
			BackgroundTransparency = 1,
		}, {
			InputInner = e("TextBox", {
				BackgroundTransparency = 1,
				Size = UDim2.new(1, -PADDING * 2, 1, -PADDING * 2),
				Position = UDim2.new(0.5, 0, 0.5, 0),
				AnchorPoint = Vector2.new(0.5, 0.5),
				Font = theme.InputFont,
				ClearTextOnFocus = false,
				TextXAlignment = Enum.TextXAlignment.Center,
				TextSize = TEXT_SIZE,
				Text = value,
				PlaceholderText = shownPlaceholder,
				PlaceholderColor3 = theme.Text2,
				TextColor3 = theme.Text1,

				[Roact.Change.Text] = function(rbx)
					onValueChange(rbx.Text)
				end,
				[Roact.Event.Focused] = function()
					self:setState({
						focused = true,
					})
				end,
				[Roact.Event.FocusLost] = function()
					self:setState({
						focused = false,
					})
				end,
			}),
		})
	end)
end

return FormTextInput