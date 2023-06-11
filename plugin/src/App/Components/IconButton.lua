local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Flipper = require(Packages.Flipper)

local Assets = require(Plugin.Assets)
local bindingUtil = require(Plugin.App.bindingUtil)

local HOVER_SPRING_PROPS = {
	frequency = 5,
	dampingRatio = 1.1,
}

local e = Roact.createElement

local IconButton = Roact.Component:extend("IconButton")

function IconButton:init()
	self.motor = Flipper.SingleMotor.new(0)
	self.binding = bindingUtil.fromMotor(self.motor)
end

function IconButton:render()
	local iconSize = self.props.iconSize

	return e("ImageButton", {
		Size = UDim2.new(0, iconSize * 1.5, 0, iconSize * 1.5),
		Position = self.props.position,
		AnchorPoint = self.props.anchorPoint,

		Visible = self.props.visible,
		LayoutOrder = self.props.layoutOrder,
		ZIndex = self.props.zIndex,
		BackgroundTransparency = 1,

		[Roact.Event.Activated] = self.props.onClick,

		[Roact.Event.MouseEnter] = function()
			self.motor:setGoal(
				Flipper.Spring.new(1, HOVER_SPRING_PROPS)
			)
		end,

		[Roact.Event.MouseLeave] = function()
			self.motor:setGoal(
				Flipper.Spring.new(0, HOVER_SPRING_PROPS)
			)
		end,
	}, {
		Icon = e("ImageLabel", {
			Image = self.props.icon,
			ImageColor3 = self.props.color,
			ImageTransparency = self.props.transparency,

			Size = UDim2.new(0, iconSize, 0, iconSize),
			Position = UDim2.new(0.5, 0, 0.5, 0),
			AnchorPoint = Vector2.new(0.5, 0.5),

			BackgroundTransparency = 1,
		}),

		HoverCircle = e("ImageLabel", {
			Image = Assets.Images.Circles[128],
			ImageColor3 = self.props.color,
			ImageTransparency = Roact.joinBindings({
				hover = self.binding,
				transparency = self.props.transparency,
			}):map(function(values)
				return bindingUtil.blendAlpha({ 0.85, 1 - values.hover, values.transparency })
			end),

			Size = UDim2.new(1, 0, 1, 0),

			BackgroundTransparency = 1,
		}),

		Children = Roact.createFragment(self.props[Roact.Children]),
	})
end

return IconButton
