local TextService = game:GetService("TextService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)
local Flipper = require(Rojo.Flipper)

local Assets = require(Plugin.Assets)
local Theme = require(Plugin.App.Theme)
local bindingUtil = require(Plugin.App.bindingUtil)

local SlicedImage = require(script.Parent.SlicedImage)
local ScrollingFrame = require(script.Parent.ScrollingFrame)

local e = Roact.createElement

local Dropdown = Roact.Component:extend("Dropdown")

function Dropdown:init()
	self.openMotor = Flipper.SingleMotor.new(0)
	self.openBinding = bindingUtil.fromMotor(self.openMotor)

	self.contentSize, self.setContentSize = Roact.createBinding(Vector2.new(0, 0))

	self:setState({
		open = false,
	})
end

function Dropdown:didUpdate()
	self.openMotor:setGoal(
		Flipper.Spring.new(self.state.open and 1 or 0, {
			frequency = 6,
			dampingRatio = 1.1,
		})
	)
end

function Dropdown:render()
	return Theme.with(function(theme)
		theme = theme.Dropdown

		local optionButtons = {}
		local width = -1
		for i, option in self.props.options do
			local textSize = TextService:GetTextSize(
				tostring(option or ""), 15, Enum.Font.GothamMedium,
				Vector2.new(math.huge, 20)
			)
			if textSize.X > width then
				width = textSize.X
			end

			table.insert(optionButtons, e("TextButton", {
				Text = tostring(option),
				LayoutOrder = i,
				Size = UDim2.new(1, 0, 0, 20),
				BackgroundColor3 = theme.BackgroundColor,
				TextTransparency = self.props.transparency,
				BackgroundTransparency = self.props.transparency,
				BorderSizePixel = 0,
				TextColor3 = theme.TextColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextSize = 15,
				Font = Enum.Font.GothamMedium,

				[Roact.Event.Activated] = function()
					self:setState({
						open = false,
					})
					self.props.onClick(option)
				end,
			}, {
				Padding = e("UIPadding", {
					PaddingLeft = UDim.new(0, 6),
				}),
			}))
		end

		return e("ImageButton", {
			Size = UDim2.new(0, width+50, 0, 28),
			Position = self.props.position,
			AnchorPoint = self.props.anchorPoint,
			LayoutOrder = self.props.layoutOrder,
			ZIndex = self.props.zIndex,
			BackgroundTransparency = 1,

			[Roact.Event.Activated] = function()
				self:setState({
					open = not self.state.open,
				})
			end,
		}, {
			Border = e(SlicedImage, {
				slice = Assets.Slices.RoundedBorder,
				color = theme.BorderColor,
				transparency = self.props.transparency,
				size = UDim2.new(1, 0, 1, 0),
			}, {
				DropArrow = e("ImageLabel", {
					Image = Assets.Images.Dropdown.Arrow,
					ImageColor3 = self.openBinding:map(function(a)
						return theme.Closed.IconColor:Lerp(theme.Open.IconColor, a)
					end),
					ImageTransparency = self.props.transparency,

					Size = UDim2.new(0, 20, 0, 20),
					Position = UDim2.new(1, -6, 0.5, 0),
					AnchorPoint = Vector2.new(1, 0.5),
					Rotation = self.openBinding:map(function(a)
						return a * 180
					end),

					BackgroundTransparency = 1,
				}),
				Active = e("TextLabel", {
					Size = UDim2.new(1, -30, 1, 0),
					Position = UDim2.new(0, 6, 0, 0),
					BackgroundTransparency = 1,
					Text = self.props.active,
					Font = Enum.Font.GothamMedium,
					TextSize = 15,
					TextColor3 = theme.TextColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextTransparency = self.props.transparency,
				}),
			}),
			Options = self.state.open and e(ScrollingFrame, {
				position = UDim2.new(1, 0, 1, 0),
				size = UDim2.new(1, 0, 3, 0),
				anchorPoint = Vector2.new(1, 0),
				transparency = self.props.transparency,
				contentSize = self.contentSize,
			}, {
				Corner = e("UICorner", {
					CornerRadius = UDim.new(0, 3),
				}),
				Layout = e("UIListLayout", {
					VerticalAlignment = Enum.VerticalAlignment.Top,
					FillDirection = Enum.FillDirection.Vertical,
					SortOrder = Enum.SortOrder.LayoutOrder,
					Padding = UDim.new(0, 0),

					[Roact.Change.AbsoluteContentSize] = function(object)
						self.setContentSize(object.AbsoluteContentSize)
					end,
				}),
				table.unpack(optionButtons),
			}) or nil,
		})
	end)
end

return Dropdown
