local RunService = game:GetService("RunService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Theme = require(Plugin.App.Theme)
local Assets = require(Plugin.Assets)

local ROTATIONS_PER_SECOND = 1.75

local e = Roact.createElement

local Spinner = Roact.PureComponent:extend("Spinner")

function Spinner:init()
	self.rotation, self.setRotation = Roact.createBinding(0)
end

function Spinner:render()
	return Theme.with(function(theme)
		return e("ImageLabel", {
			Image = Assets.Images.Spinner.Background,
			ImageColor3 = theme.Spinner.BackgroundColor,
			ImageTransparency = self.props.transparency,

			Size = UDim2.new(0, 24, 0, 24),
			Position = self.props.position,
			AnchorPoint = self.props.anchorPoint,

			LayoutOrder = self.props.layoutOrder,
			BackgroundTransparency = 1,
		}, {
			Foreground = e("ImageLabel", {
				Image = Assets.Images.Spinner.Foreground,
				ImageColor3 = theme.Spinner.ForegroundColor,
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

function Spinner:didMount()
	self.stepper = RunService.RenderStepped:Connect(function(deltaTime)
		local rotation = self.rotation:getValue()

		rotation = rotation + deltaTime * ROTATIONS_PER_SECOND
		rotation = rotation % 1

		self.setRotation(rotation)
	end)
end

function Spinner:willUnmount()
	self.stepper:Disconnect()
end

return Spinner
