local TextService = game:GetService("TextService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local Theme = require(Plugin.App.Theme)

local TextButton = require(Plugin.App.components.TextButton)
local BorderedContainer = require(Plugin.App.components.BorderedContainer)
local ScrollingFrame = require(Plugin.App.components.ScrollingFrame)

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
		-- It's difficult to pass down PropMarkers to custom components
		Dummy = e("Frame", {
			Size = UDim2.new(1, 0, 1, 0),
			BackgroundTransparency = 1,

			[Roact.Change.AbsoluteSize] = function(object)
				local containerSize = object.AbsoluteSize - ERROR_PADDING * 2

				local textBounds = TextService:GetTextSize(
					self.props.errorMessage, 16, Enum.Font.Code,
					Vector2.new(containerSize.X, math.huge)
				)

				self.setContentSize(Vector2.new(containerSize.X, textBounds.Y))
			end,
		}),

		ScrollingFrame = e(ScrollingFrame, {
			size = UDim2.new(1, 0, 1, 0),
			contentSize = self.contentSize:map(function(value)
				return value + ERROR_PADDING * 2
			end),
			transparency = self.props.transparency,
		}, {
			ErrorMessage = Theme.with(function(theme)
				return e("TextLabel", {
					Text = self.props.errorMessage,
					Font = Enum.Font.Code,
					TextSize = 16,
					TextColor3 = theme.ErrorColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextYAlignment = Enum.TextYAlignment.Top,
					TextTransparency = self.props.transparency,
					TextWrapped = true,

					Size = UDim2.new(1, 0, 1, 0),

					BackgroundTransparency = 1,
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
				text = "Got it!",
				style = "Bordered",
				transparency = self.props.transparency,
				layoutOrder = 1,
				onClick = self.props.onClose,
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
	return {
		errorMessage = props.errorMessage,
	}
end

return ErrorPage