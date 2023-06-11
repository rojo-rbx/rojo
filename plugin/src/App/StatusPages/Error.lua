local TextService = game:GetService("TextService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Theme = require(Plugin.App.Theme)

local TextButton = require(Plugin.App.Components.TextButton)
local BorderedContainer = require(Plugin.App.Components.BorderedContainer)
local ScrollingFrame = require(Plugin.App.Components.ScrollingFrame)
local Tooltip = require(Plugin.App.Components.Tooltip)

local e = Roact.createElement

local ERROR_PADDING = Vector2.new(13, 10)

local Error = Roact.Component:extend("Error")

function Error:init()
	self.contentSize, self.setContentSize = Roact.createBinding(Vector2.new(0, 0))
end

function Error:render()
	return e(BorderedContainer, {
		size = Roact.joinBindings({
			containerSize = self.props.containerSize,
			contentSize = self.contentSize,
		}):map(function(values)
			local maximumSize = values.containerSize
			maximumSize -= Vector2.new(14, 14) * 2 -- Page padding
			maximumSize -= Vector2.new(0, 34 + 10) -- Buttons and spacing

			local outerSize = values.contentSize + ERROR_PADDING * 2

			return UDim2.new(1, 0, 0, math.min(outerSize.Y, maximumSize.Y))
		end),
		transparency = self.props.transparency,
	}, {
		ScrollingFrame = e(ScrollingFrame, {
			size = UDim2.new(1, 0, 1, 0),
			contentSize = self.contentSize:map(function(value)
				return value + ERROR_PADDING * 2
			end),
			transparency = self.props.transparency,

			[Roact.Change.AbsoluteSize] = function(object)
				local containerSize = object.AbsoluteSize - ERROR_PADDING * 2

				local textBounds = TextService:GetTextSize(
					self.props.errorMessage, 16, Enum.Font.Code,
					Vector2.new(containerSize.X, math.huge)
				)

				self.setContentSize(Vector2.new(containerSize.X, textBounds.Y))
			end,
		}, {
			ErrorMessage = Theme.with(function(theme)
				return e("TextBox", {
					[Roact.Event.InputBegan] = function(rbx, input)
						if input.UserInputType ~= Enum.UserInputType.MouseButton1 then return end
						rbx.SelectionStart = 0
						rbx.CursorPosition = #rbx.Text+1
					end,


					Text = self.props.errorMessage,
					TextEditable = false,
					Font = Enum.Font.Code,
					TextSize = 16,
					TextColor3 = theme.ErrorColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextYAlignment = Enum.TextYAlignment.Top,
					TextTransparency = self.props.transparency,
					TextWrapped = true,
					ClearTextOnFocus = false,
					BackgroundTransparency = 1,
					Size = UDim2.new(1, 0, 1, 0),
				})
			end),

			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, ERROR_PADDING.X),
				PaddingRight = UDim.new(0, ERROR_PADDING.X),
				PaddingTop = UDim.new(0, ERROR_PADDING.Y),
				PaddingBottom = UDim.new(0, ERROR_PADDING.Y),
			}),
		}),
	})
end

local ErrorPage = Roact.Component:extend("ErrorPage")

function ErrorPage:init()
	self.containerSize, self.setContainerSize = Roact.createBinding(Vector2.new(0, 0))
end

function ErrorPage:render()
	return Roact.createElement("Frame", {
		Size = UDim2.new(1, 0, 1, 0),
		BackgroundTransparency = 1,

		[Roact.Change.AbsoluteSize] = function(object)
			self.setContainerSize(object.AbsoluteSize)
		end,
	}, {
		Error = e(Error, {
			errorMessage = self.state.errorMessage,
			containerSize = self.containerSize,
			transparency = self.props.transparency,
			layoutOrder = 1,
		}),

		Buttons = e("Frame", {
			Size = UDim2.new(1, 0, 0, 35),
			LayoutOrder = 2,
			BackgroundTransparency = 1,
		}, {
			Close = e(TextButton, {
				text = "Okay",
				style = "Bordered",
				transparency = self.props.transparency,
				layoutOrder = 1,
				onClick = self.props.onClose,
			}, {
				Tip = e(Tooltip.Trigger, {
					text = "Dismiss message"
				}),
			}),

			Layout = e("UIListLayout", {
				HorizontalAlignment = Enum.HorizontalAlignment.Right,
				FillDirection = Enum.FillDirection.Horizontal,
				SortOrder = Enum.SortOrder.LayoutOrder,
			}),
		}),

		Layout = e("UIListLayout", {
			VerticalAlignment = Enum.VerticalAlignment.Center,
			FillDirection = Enum.FillDirection.Vertical,
			SortOrder = Enum.SortOrder.LayoutOrder,
			Padding = UDim.new(0, 10),
		}),

		Padding = e("UIPadding", {
			PaddingLeft = UDim.new(0, 14),
			PaddingRight = UDim.new(0, 14),
			PaddingTop = UDim.new(0, 14),
			PaddingBottom = UDim.new(0, 14),
		}),
	})
end

function ErrorPage.getDerivedStateFromProps(props)
	-- If errorMessage ever gets removed from props, make sure we still have the
	-- property! The component still needs to have its data for it to be properly
	-- animated out without the labels changing.

	return {
		errorMessage = props.errorMessage,
	}
end

return ErrorPage
