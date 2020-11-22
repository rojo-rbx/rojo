local TextService = game:GetService("TextService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)
local Flipper = require(Rojo.Flipper)

local Theme = require(Plugin.App.Theme)
local Assets = require(Plugin.Assets)

local SlicedImage = require(script.Parent.SlicedImage)

local SPRING_PROPS = {
	frequency = 5,
	dampingRatio = 1,
}

local e = Roact.createElement

local function mapLerpColor(binding, color1, color2)
	return binding:map(function(value)
		return color1:Lerp(color2, value)
	end)
end

local function bindingDeriveProperty(binding, propertyName)
	return binding:map(function(values)
		return values[propertyName]
	end)
end

local function blendAlpha(alphaValues)
	local alpha

	for _, value in pairs(alphaValues) do
		alpha = alpha and alpha + (1 - alpha) * value or value
	end

	return alpha
end

local Button = Roact.Component:extend("Button")

function Button:init()
	local motor = Flipper.GroupMotor.new({
		hover = 0,
		enabled = self.props.enabled and 1 or 0,
	})
	local motorBinding, setMotorBinding = Roact.createBinding(motor:getValue())
	motor:onStep(setMotorBinding)
	self.motor = motor
	self.binding = motorBinding
end

function Button:render()
	return Theme.with(function(theme)
		local textSize = TextService:GetTextSize(
			self.props.text, 18, Enum.Font.GothamSemibold,
			Vector2.new(math.huge, math.huge)
		)

		local style = self.props.style

		theme = theme.Button[style]

		local bindingHover = bindingDeriveProperty(self.binding, "hover")
		local bindingEnabled = bindingDeriveProperty(self.binding, "enabled")

		return e("ImageButton", {
			Size = UDim2.new(0, 15 + textSize.X + 15, 0, 34),
			Position = self.props.position,
			AnchorPoint = self.props.anchorPoint,

			LayoutOrder = self.props.layoutOrder,
			BackgroundTransparency = 1,

			[Roact.Event.Activated] = self.props.onClick,

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
		}, {
			Text = e("TextLabel", {
				Text = self.props.text,
				Font = Enum.Font.GothamSemibold,
				TextSize = 18,
				TextColor3 = mapLerpColor(bindingEnabled, theme.Enabled.Text, theme.Disabled.Text),
				TextTransparency = self.props.transparency,

				Size = UDim2.new(1, 0, 1, 0),

				BackgroundTransparency = 1,
			}),

			Border = style == "Bordered" and e(SlicedImage, {
				slice = Assets.Slices.RoundedBorder,
				color = mapLerpColor(bindingEnabled, theme.Enabled.Border, theme.Disabled.Border),
				transparency = self.props.transparency,

				size = UDim2.new(1, 0, 1, 0),

				zindex = 0,
			}),

			HoverOverlay = e(SlicedImage, {
				slice = Assets.Slices.RoundedBackground,
				color = theme.HoverOverlay,
				transparency = Roact.joinBindings({
					hover = bindingHover:map(function(value)
						return 1 - value
					end),
					transparency = self.props.transparency,
				}):map(function(values)
					return blendAlpha({ 0.9, values.hover, values.transparency })
				end),

				size = UDim2.new(1, 0, 1, 0),

				zindex = -1,
			}),

			Background = style == "Solid" and e(SlicedImage, {
				slice = Assets.Slices.RoundedBackground,
				color = mapLerpColor(bindingEnabled, theme.Enabled.Background, theme.Disabled.Background),
				transparency = self.props.transparency,

				size = UDim2.new(1, 0, 1, 0),

				zindex = -2,
			}),
		})
	end)
end

return Button