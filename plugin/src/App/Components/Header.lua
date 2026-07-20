local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Theme = require(Plugin.App.Theme)
local Assets = require(Plugin.Assets)
local Branding = require(Plugin.Branding)
local Config = require(Plugin.Config)
local Version = require(Plugin.Version)

local e = Roact.createElement

local function Header(props)
	return Theme.with(function(theme)
		if props.full then
			return e("Frame", {
				Size = UDim2.new(1, 0, 0, 52),
				LayoutOrder = props.layoutOrder,
				BackgroundTransparency = 1,
			}, {
				Wordmark = e("ImageLabel", {
					Image = Assets.Images.FullLogo,
					ImageColor3 = Color3.new(1, 1, 1),
					ImageTransparency = props.transparency,
					BackgroundTransparency = 1,
					ScaleType = Enum.ScaleType.Fit,
					Size = UDim2.new(1, -56, 0, 31),
					Position = UDim2.new(0.5, 0, 0, 0),
					AnchorPoint = Vector2.new(0.5, 0),
				}),
				Tagline = e("TextLabel", {
					Text = Branding.Tagline,
					FontFace = theme.Font.Thin,
					TextSize = theme.TextSize.Small,
					TextColor3 = theme.Tokens.SecondaryText,
					TextTransparency = props.transparency,
					BackgroundTransparency = 1,
					TextTruncate = Enum.TextTruncate.AtEnd,
					Size = UDim2.new(1, 0, 0, 16),
					Position = UDim2.new(0, 0, 1, -16),
				}),
			})
		end

		return e("Frame", {
			Size = UDim2.new(1, 0, 0, 32),
			LayoutOrder = props.layoutOrder,
			BackgroundTransparency = 1,
		}, {
			Logo = e("ImageLabel", {
				Image = props.icon or Assets.Images.Logo,
				ImageColor3 = Color3.new(1, 1, 1),
				ImageTransparency = props.transparency,
				ScaleType = Enum.ScaleType.Fit,
				Size = UDim2.fromOffset(27, 27),
				Position = UDim2.new(0, 0, 0.5, 0),
				AnchorPoint = Vector2.new(0, 0.5),
				BackgroundTransparency = 1,
			}),
			Name = e("TextLabel", {
				Text = Branding.Name,
				FontFace = theme.Font.Bold,
				TextSize = theme.TextSize.Medium,
				TextColor3 = theme.Tokens.PrimaryText,
				TextTransparency = props.transparency,
				TextXAlignment = Enum.TextXAlignment.Left,
				BackgroundTransparency = 1,
				Position = UDim2.new(0, 36, 0, 0),
				Size = UDim2.new(0, 62, 1, 0),
			}),
			Version = e("TextLabel", {
				Text = Version.display(Config.version),
				FontFace = theme.Font.Thin,
				TextSize = theme.TextSize.Small,
				TextColor3 = theme.Header.VersionColor,
				TextTransparency = props.transparency,
				TextXAlignment = Enum.TextXAlignment.Right,
				TextTruncate = Enum.TextTruncate.AtEnd,
				BackgroundTransparency = 1,
				Position = UDim2.new(1, -74, 0, 0),
				Size = UDim2.new(0, 74, 1, 0),
			}),
		})
	end)
end

return Header
