local RunService = game:GetService("RunService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local Theme = require(Plugin.App.Theme)

local ROTATIONS_PER_SECOND = 1.75

local e = Roact.createElement

local Throbber = Roact.PureComponent:extend("Throbber")

function Throbber:init()
	self.rotation, self.setRotation = Roact.createBinding(0)
end

function Throbber:render()
	return Theme.with(function(theme)
		return e("ImageLabel", {
			Image = "rbxassetid://3222730627",
			ImageColor3 = theme.Throbber.Background,
			ImageTransparency = self.props.transparency,
			Size = UDim2.new(0, 24, 0, 24),
			Position = self.props.Position,
			AnchorPoint = self.props.AnchorPoint,
			BackgroundTransparency = 1,
		}, {
			Foreground = e("ImageLabel", {
				Image = "rbxassetid://3222731032",
				ImageColor3 = theme.Throbber.Foreground,
				ImageTransparency = self.props.transparency,
				Size = UDim2.new(1, 0, 1, 0),
				Rotation = self.rotation:map(function(value)
					return value * 360
				end),
				BackgroundTransparency = 1,
			}),
		})
	end)
end

function Throbber:didMount()
	self.stepper = RunService.RenderStepped:Connect(function(deltaTime)
		local rotation = self.rotation:getValue()

		rotation = rotation + deltaTime * ROTATIONS_PER_SECOND
		rotation = rotation % 1

		self.setRotation(rotation)
	end)
end

function Throbber:willUnmount()
	self.stepper:Disconnect()
end

return Throbber