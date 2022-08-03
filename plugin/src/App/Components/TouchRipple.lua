local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Flipper = require(Packages.Flipper)

local Assets = require(Plugin.Assets)
local bindingUtil = require(Plugin.App.bindingUtil)

local EXPAND_SPRING = {
	frequency = 7,
	dampingRatio = 2,
}

local TouchRipple = Roact.Component:extend("TouchRipple")

function TouchRipple:init()
	self.ref = Roact.createRef()

	self.motor = Flipper.GroupMotor.new({
		scale = 0,
		opacity = 0,
	})
	self.binding = bindingUtil.fromMotor(self.motor)

	self.position, self.setPosition = Roact.createBinding(
		Vector2.new(0, 0)
	)
end

function TouchRipple:reset()
	self.motor:setGoal({
		scale = Flipper.Instant.new(0),
		opacity = Flipper.Instant.new(0),
	})

	-- Forces motor to update
	self.motor:step(0)
end

function TouchRipple:calculateRadius(position)
	local container = self.ref.current

	if container then
		local corner = Vector2.new(
			math.floor((1 - position.X) + 0.5),
			math.floor((1 - position.Y) + 0.5)
		)

		local size = container.AbsoluteSize
		local ratio = size / math.min(size.X, size.Y)

		return ((corner * ratio) - (position * ratio)).Magnitude
	else
		return 0
	end
end

function TouchRipple:render()
	local scale = bindingUtil.deriveProperty(self.binding, "scale")
	local transparency = bindingUtil.deriveProperty(self.binding, "opacity"):map(function(value)
		return 1 - value
	end)

	transparency = Roact.joinBindings({
		transparency,
		self.props.transparency,
	}):map(bindingUtil.blendAlpha)

	return Roact.createElement("Frame", {
		ClipsDescendants = true,
		Size = UDim2.new(1, 0, 1, 0),
		ZIndex = self.props.zIndex,
		BackgroundTransparency = 1,

		[Roact.Ref] = self.ref,

		[Roact.Event.InputBegan] = function(object, input)
			if input.UserInputType == Enum.UserInputType.MouseButton1 then
				self:reset()

				local position = Vector2.new(input.Position.X, input.Position.Y)
				local relativePosition = (position - object.AbsolutePosition) / object.AbsoluteSize

				self.setPosition(relativePosition)

				self.motor:setGoal({
					scale = Flipper.Spring.new(1, EXPAND_SPRING),
					opacity = Flipper.Spring.new(1, EXPAND_SPRING),
				})

				input:GetPropertyChangedSignal("UserInputState"):Connect(function()
					local userInputState = input.UserInputState

					if
						userInputState == Enum.UserInputState.Cancel
						or userInputState == Enum.UserInputState.End
					then
						self.motor:setGoal({
							opacity = Flipper.Spring.new(0, {
								frequency = 5,
								dampingRatio = 1,
							}),
						})
					end
				end)
			end
		end,
	}, {
		Circle = Roact.createElement("ImageLabel", {
			Image = Assets.Images.Circles[500],
			ImageColor3 = self.props.color,
			ImageTransparency = transparency,

			Size = Roact.joinBindings({
				scale = scale,
				position = self.position,
			}):map(function(values)
				local targetSize = self:calculateRadius(values.position) * 2
				local currentSize = targetSize * values.scale

				local container = self.ref.current

				if container then
					local containerSize = container.AbsoluteSize
					local containerAspect = containerSize.X / containerSize.Y

					return UDim2.new(
						currentSize / math.max(containerAspect, 1), 0,
						currentSize * math.min(containerAspect, 1), 0
					)
				end
			end),

			Position = self.position:map(function(value)
				return UDim2.new(value.X, 0, value.Y, 0)
			end),
			AnchorPoint = Vector2.new(0.5, 0.5),

			BackgroundTransparency = 1,
		}),
	})
end

return TouchRipple
