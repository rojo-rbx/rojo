local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Flipper = require(Packages.Flipper)

local Assets = require(Plugin.Assets)
local Theme = require(Plugin.App.Theme)
local bindingUtil = require(Plugin.App.bindingUtil)
local getTextBoundsAsync = require(Plugin.App.getTextBoundsAsync)

local SlicedImage = require(script.Parent.SlicedImage)
local ScrollingFrame = require(script.Parent.ScrollingFrame)
local Tooltip = require(script.Parent.Tooltip)

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

function Dropdown:didUpdate(prevProps)
	if self.props.locked and not prevProps.locked then
		self:setState({
			open = false,
		})
	end

	self.openMotor:setGoal(Flipper.Spring.new(self.state.open and 1 or 0, {
		frequency = 6,
		dampingRatio = 1.1,
	}))
end

function Dropdown:render()
	return Theme.with(function(theme)
		local dropdownTheme = theme.Dropdown

		local optionButtons = {}
		local width = -1
		for i, option in self.props.options do
			local text = tostring(option or "")
			local textBounds = getTextBoundsAsync(text, theme.Font.Main, theme.TextSize.Body, math.huge)
			if textBounds.X > width then
				width = textBounds.X
			end

			optionButtons[text] = e("TextButton", {
				Text = text,
				LayoutOrder = i,
				Size = UDim2.new(1, 0, 0, 24),
				BackgroundColor3 = dropdownTheme.BackgroundColor,
				TextTransparency = self.props.transparency,
				BackgroundTransparency = self.props.transparency,
				BorderSizePixel = 0,
				TextColor3 = dropdownTheme.TextColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextSize = theme.TextSize.Body,
				FontFace = theme.Font.Main,

				[Roact.Event.Activated] = function()
					if self.props.locked then
						return
					end
					self:setState({
						open = false,
					})
					self.props.onClick(option)
				end,
			}, {
				Padding = e("UIPadding", {
					PaddingLeft = UDim.new(0, 6),
				}),
			})
		end

		return e("ImageButton", {
			Size = UDim2.new(0, width + 50, 0, 28),
			Position = self.props.position,
			AnchorPoint = self.props.anchorPoint,
			LayoutOrder = self.props.layoutOrder,
			ZIndex = self.props.zIndex,
			BackgroundTransparency = 1,

			[Roact.Event.Activated] = function()
				if self.props.locked then
					return
				end
				self:setState({
					open = not self.state.open,
				})
			end,
		}, {
			Border = e(SlicedImage, {
				slice = Assets.Slices.RoundedBorder,
				color = dropdownTheme.BorderColor,
				transparency = self.props.transparency,
				size = UDim2.new(1, 0, 1, 0),
			}, {
				DropArrow = e("ImageLabel", {
					Image = if self.props.locked then Assets.Images.Dropdown.Locked else Assets.Images.Dropdown.Arrow,
					ImageColor3 = dropdownTheme.IconColor,
					ImageTransparency = self.props.transparency,

					Size = UDim2.new(0, 18, 0, 18),
					Position = UDim2.new(1, -6, 0.5, 0),
					AnchorPoint = Vector2.new(1, 0.5),
					Rotation = self.openBinding:map(function(a)
						return a * 180
					end),

					BackgroundTransparency = 1,
				}, {
					StateTip = if self.props.locked
						then e(Tooltip.Trigger, {
							text = self.props.lockedTooltip or "(Cannot be changed right now)",
						})
						else nil,
				}),
				Active = e("TextLabel", {
					Size = UDim2.new(1, -30, 1, 0),
					Position = UDim2.new(0, 6, 0, 0),
					BackgroundTransparency = 1,
					Text = self.props.active,
					FontFace = theme.Font.Main,
					TextSize = theme.TextSize.Body,
					TextColor3 = dropdownTheme.TextColor,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextTransparency = self.props.transparency,
				}),
			}),
			Options = if self.state.open
				then e(SlicedImage, {
					slice = Assets.Slices.RoundedBackground,
					color = dropdownTheme.BackgroundColor,
					position = UDim2.new(1, 0, 1, 3),
					size = self.openBinding:map(function(a)
						return UDim2.new(1, 0, a * math.min(3, #self.props.options), 0)
					end),
					anchorPoint = Vector2.new(1, 0),
				}, {
					Border = e(SlicedImage, {
						slice = Assets.Slices.RoundedBorder,
						color = dropdownTheme.BorderColor,
						transparency = self.props.transparency,
						size = UDim2.new(1, 0, 1, 0),
					}),
					ScrollingFrame = e(ScrollingFrame, {
						size = UDim2.new(1, -4, 1, -4),
						position = UDim2.new(0, 2, 0, 2),
						transparency = self.props.transparency,
						contentSize = self.contentSize,
					}, {
						Layout = e("UIListLayout", {
							VerticalAlignment = Enum.VerticalAlignment.Top,
							FillDirection = Enum.FillDirection.Vertical,
							SortOrder = Enum.SortOrder.LayoutOrder,
							Padding = UDim.new(0, 0),

							[Roact.Change.AbsoluteContentSize] = function(object)
								self.setContentSize(object.AbsoluteContentSize)
							end,
						}),
						Options = Roact.createFragment(optionButtons),
					}),
				})
				else nil,
		})
	end)
end

return Dropdown
