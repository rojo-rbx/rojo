local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Flipper = require(Packages.Flipper)

local Assets = require(Plugin.Assets)
local Theme = require(Plugin.App.Theme)
local bindingUtil = require(Plugin.App.bindingUtil)

local SlicedImage = require(script.Parent.SlicedImage)
local Tooltip = require(script.Parent.Tooltip)

local e = Roact.createElement

local Checkbox = Roact.Component:extend("Checkbox")

function Checkbox:init()
	self.motor = Flipper.SingleMotor.new(self.props.active and 1 or 0)
	self.binding = bindingUtil.fromMotor(self.motor)
end

function Checkbox:didUpdate(lastProps)
	if lastProps.active ~= self.props.active then
		self.motor:setGoal(
			Flipper.Spring.new(self.props.active and 1 or 0, {
				frequency = 6,
				dampingRatio = 1.1,
			})
		)
	end
end

function Checkbox:render()
	return Theme.with(function(theme)
		theme = theme.Checkbox

		local activeTransparency = Roact.joinBindings({
			self.binding:map(function(value)
				return 1 - value
			end),
			self.props.transparency,
		}):map(bindingUtil.blendAlpha)

		return e("ImageButton", {
			Size = UDim2.new(0, 28, 0, 28),
			Position = self.props.position,
			AnchorPoint = self.props.anchorPoint,
			LayoutOrder = self.props.layoutOrder,
			ZIndex = self.props.zIndex,
			BackgroundTransparency = 1,

			[Roact.Event.Activated] = self.props.onClick,
		}, {
			StateTip = e(Tooltip.Trigger, {
				text = if self.props.active then "Enabled" else "Disabled",
			}),

			Active = e(SlicedImage, {
				slice = Assets.Slices.RoundedBackground,
				color = theme.Active.BackgroundColor,
				transparency = activeTransparency,
				size = UDim2.new(1, 0, 1, 0),
				zIndex = 2,
			}, {
				Icon = e("ImageLabel", {
					Image = Assets.Images.Checkbox.Active,
					ImageColor3 = theme.Active.IconColor,
					ImageTransparency = activeTransparency,

					Size = UDim2.new(0, 16, 0, 16),
					Position = UDim2.new(0.5, 0, 0.5, 0),
					AnchorPoint = Vector2.new(0.5, 0.5),

					BackgroundTransparency = 1,
				}),
			}),

			Inactive = e(SlicedImage, {
				slice = Assets.Slices.RoundedBorder,
				color = theme.Inactive.BorderColor,
				transparency = self.props.transparency,
				size = UDim2.new(1, 0, 1, 0),
			}, {
				Icon = e("ImageLabel", {
					Image = Assets.Images.Checkbox.Inactive,
					ImageColor3 = theme.Inactive.IconColor,
					ImageTransparency = self.props.transparency,

					Size = UDim2.new(0, 16, 0, 16),
					Position = UDim2.new(0.5, 0, 0.5, 0),
					AnchorPoint = Vector2.new(0.5, 0.5),

					BackgroundTransparency = 1,
				}),
			}),
		})
	end)
end

return Checkbox
