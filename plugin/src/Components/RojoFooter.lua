local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local Config = require(Plugin.Config)
local Version = require(Plugin.Version)
local Assets = require(Plugin.Assets)
local Theme = require(Plugin.Theme)

local e = Roact.createElement

local RojoFooter = Roact.Component:extend("RojoFooter")

function RojoFooter:init()
	self.footerSize, self.setFooterSize = Roact.createBinding(Vector2.new())
	self.footerVersionSize, self.setFooterVersionSize = Roact.createBinding(Vector2.new())
end

function RojoFooter:render()
	return e("Frame", {
		LayoutOrder = 3,
		Size = UDim2.new(1, 0, 0, 32),
		BackgroundColor3 = Theme.SecondaryColor,
		BorderSizePixel = 0,
	}, {
		Padding = e("UIPadding", {
			PaddingTop = UDim.new(0, 4),
			PaddingBottom = UDim.new(0, 4),
			PaddingLeft = UDim.new(0, 8),
			PaddingRight = UDim.new(0, 8),
		}),

		LogoContainer = e("Frame", {
			BackgroundTransparency = 1,

			Size = UDim2.new(0, 0, 0, 32),
		}, {
			Logo = e("ImageLabel", {
				Image = Assets.Images.Logo,
				Size = UDim2.new(0, 80, 0, 40),
				ScaleType = Enum.ScaleType.Fit,
				BackgroundTransparency = 1,
				Position = UDim2.new(0, 0, 1, -10),
				AnchorPoint = Vector2.new(0, 1),
			}),
		}),

		Version = e("TextLabel", {
			Position = UDim2.new(1, 0, 0, 0),
			Size = UDim2.new(0, 0, 1, 0),
			AnchorPoint = Vector2.new(1, 0),
			Font = Theme.TitleFont,
			TextSize = 18,
			Text = Version.display(Config.version),
			TextXAlignment = Enum.TextXAlignment.Right,
			TextColor3 = Theme.LightTextColor,
			BackgroundTransparency = 1,

			[Roact.Change.AbsoluteSize] = function(rbx)
				self.setFooterVersionSize(rbx.AbsoluteSize)
			end,
		}),
	})
end

return RojoFooter