local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Theme = require(Plugin.App.Theme)
local Assets = require(Plugin.Assets)
local Config = require(Plugin.Config)
local Version = require(Plugin.Version)

local e = Roact.createElement

local function Header(props)
	return Theme.with(function(theme)
		return e("Frame", {
			Size = UDim2.new(1, 0, 0, 32),
			LayoutOrder = props.layoutOrder,
			BackgroundTransparency = 1,
		}, {
			Logo = e("ImageLabel", {
				Image = Assets.Images.Logo,
				ImageColor3 = theme.Header.LogoColor,
				ImageTransparency = props.transparency,

				Size = UDim2.new(0, 60, 0, 27),

				LayoutOrder = 1,
				BackgroundTransparency = 1,
			}),

			Version = e("TextLabel", {
				Text = Version.display(Config.version),
				Font = Enum.Font.Gotham,
				TextSize = 14,
				TextColor3 = theme.Header.VersionColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = props.transparency,

				Size = UDim2.new(1, 0, 0, 14),

				LayoutOrder = 2,
				BackgroundTransparency = 1,
			}),

			Layout = e("UIListLayout", {
				VerticalAlignment = Enum.VerticalAlignment.Center,
				FillDirection = Enum.FillDirection.Horizontal,
				SortOrder = Enum.SortOrder.LayoutOrder,
				Padding = UDim.new(0, 15),
			}),
		})
	end)
end

return Header
