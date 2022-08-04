local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Flipper = require(Packages.Flipper)

local Dictionary = require(Plugin.Dictionary)

local bindingUtil = require(script.Parent.bindingUtil)

local e = Roact.createElement

local Page = Roact.Component:extend("Page")

function Page:init()
	self:setState({
		rendered = self.props.active
	})

	self.motor = Flipper.SingleMotor.new(self.props.active and 1 or 0)
	self.binding = bindingUtil.fromMotor(self.motor)

	self.motor:onStep(function(value)
		local rendered = value > 0

		self:setState(function(state)
			if state.rendered ~= rendered then
				return {
					rendered = rendered,
				}
			end
		end)
	end)
end

function Page:render()
	if not self.state.rendered then
		return nil
	end

	local transparency = self.binding:map(function(value)
		return 1 - value
	end)

	return e("Frame", {
		Position = transparency:map(function(value)
			value = self.props.active and value or -value
			return UDim2.new(0, value * 30, 0, 0)
		end),
		Size = UDim2.new(1, 0, 1, 0),
		BackgroundTransparency = 1,
	}, {
		Component = e(self.props.component, Dictionary.merge(self.props, {
			transparency = transparency,
		}))
	})
end

function Page:didUpdate(lastProps)
	if self.props.active ~= lastProps.active then
		self.motor:setGoal(
			Flipper.Spring.new(self.props.active and 1 or 0, {
				frequency = 6,
				dampingRatio = 1,
			})
		)
	end
end

return Page
