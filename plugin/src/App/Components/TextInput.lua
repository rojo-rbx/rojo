local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Flipper = require(Packages.Flipper)

local Theme = require(Plugin.App.Theme)
local Assets = require(Plugin.Assets)
local bindingUtil = require(Plugin.App.bindingUtil)

local SlicedImage = require(script.Parent.SlicedImage)

local SPRING_PROPS = {
	frequency = 5,
	dampingRatio = 1,
}

local e = Roact.createElement

local TextInput = Roact.Component:extend("TextInput")

function TextInput:init()
	self.motor = Flipper.GroupMotor.new({
		hover = 0,
		enabled = self.props.enabled and 1 or 0,
	})
	self.binding = bindingUtil.fromMotor(self.motor)
end

function TextInput:didUpdate(lastProps)
	if lastProps.enabled ~= self.props.enabled then
		self.motor:setGoal({
			enabled = Flipper.Spring.new(self.props.enabled and 1 or 0),
		})
	end
end

function TextInput:render()
	return Theme.with(function(theme)
		theme = theme.TextInput

		local bindingHover = bindingUtil.deriveProperty(self.binding, "hover")
		local bindingEnabled = bindingUtil.deriveProperty(self.binding, "enabled")

		return e(SlicedImage, {
			slice = Assets.Slices.RoundedBorder,
			color = bindingUtil.mapLerp(bindingEnabled, theme.Enabled.BorderColor, theme.Disabled.BorderColor),
			transparency = self.props.transparency,

			size = self.props.size or UDim2.new(1, 0, 1, 0),
			position = self.props.position,
			layoutOrder = self.props.layoutOrder,
			anchorPoint = self.props.anchorPoint,
		}, {
			HoverOverlay = e(SlicedImage, {
				slice = Assets.Slices.RoundedBackground,
				color = theme.ActionFillColor,
				transparency = Roact.joinBindings({
					hover = bindingHover:map(function(value)
						return 1 - value
					end),
					transparency = self.props.transparency,
				}):map(function(values)
					return bindingUtil.blendAlpha({ theme.ActionFillTransparency, values.hover, values.transparency })
				end),
				size = UDim2.new(1, 0, 1, 0),
				zIndex = -1,
			}),
			Input = e("TextBox", {
				BackgroundTransparency = 1,
				Size = UDim2.fromScale(1, 1),
				Text = self.props.text,
				PlaceholderText = self.props.placeholder,
				Font = Enum.Font.GothamMedium,
				TextColor3 = bindingUtil.mapLerp(bindingEnabled, theme.Disabled.TextColor, theme.Enabled.TextColor),
				PlaceholderColor3 = bindingUtil.mapLerp(
					bindingEnabled,
					theme.Disabled.PlaceholderColor,
					theme.Enabled.PlaceholderColor
				),
				TextSize = 18,
				TextEditable = self.props.enabled,
				ClearTextOnFocus = self.props.clearTextOnFocus,

				[Roact.Event.MouseEnter] = function()
					self.motor:setGoal({
						hover = Flipper.Spring.new(1, SPRING_PROPS),
					})
				end,

				[Roact.Event.MouseLeave] = function()
					self.motor:setGoal({
						hover = Flipper.Spring.new(0, SPRING_PROPS),
					})
				end,

				[Roact.Event.FocusLost] = function(rbx)
					self.props.onEntered(rbx.Text)
				end,
			}),
			Children = Roact.createFragment(self.props[Roact.Children]),
		})
	end)
end

return TextInput
