local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Theme = require(Plugin.App.Theme)
local Assets = require(Plugin.Assets)

local BorderedContainer = require(Plugin.App.Components.BorderedContainer)
local Spinner = require(Plugin.App.Components.Spinner)

local e = Roact.createElement

local ConnectingPage = Roact.Component:extend("ConnectingPage")

function ConnectingPage:render()
	return Theme.with(function(theme)
		return e(BorderedContainer, {
			size = UDim2.new(1, -32, 0, 118),
			position = UDim2.fromScale(0.5, 0.5),
			anchorPoint = Vector2.new(0.5, 0.5),
			transparency = self.props.transparency,
			backgroundColor = theme.Tokens.ElevatedCardBackground,
		}, {
			Logo = e("ImageLabel", {
				Image = Assets.Images.Logo,
				ImageColor3 = Color3.new(1, 1, 1),
				ImageTransparency = self.props.transparency,
				ScaleType = Enum.ScaleType.Fit,
				BackgroundTransparency = 1,
				Size = UDim2.fromOffset(30, 30),
				Position = UDim2.new(0.5, -20, 0, 12),
				AnchorPoint = Vector2.new(0.5, 0),
			}),
			Spinner = e(Spinner, {
				position = UDim2.new(0.5, 22, 0, 15),
				anchorPoint = Vector2.new(0.5, 0),
				transparency = self.props.transparency,
			}),
			Title = e("TextLabel", {
				Text = "Connecting to Prism server...",
				Position = UDim2.new(0, 12, 0, 52),
				Size = UDim2.new(1, -24, 0, 22),
				TextXAlignment = Enum.TextXAlignment.Center,
				FontFace = theme.Font.Main,
				TextSize = theme.TextSize.Medium,
				TextColor3 = theme.Tokens.PrimaryText,
				TextTransparency = self.props.transparency,
				BackgroundTransparency = 1,
			}),
			Detail = e("TextLabel", {
				Text = if type(self.props.text) == "string" and #self.props.text > 0
					then self.props.text
					else "Waiting for a response",
				Position = UDim2.new(0, 12, 0, 78),
				Size = UDim2.new(1, -24, 0, 23),
				TextXAlignment = Enum.TextXAlignment.Center,
				TextYAlignment = Enum.TextYAlignment.Top,
				RichText = true,
				FontFace = theme.Font.Thin,
				TextSize = theme.TextSize.Small,
				TextColor3 = theme.Tokens.SecondaryText,
				TextTruncate = Enum.TextTruncate.AtEnd,
				TextTransparency = self.props.transparency,
				BackgroundTransparency = 1,
			}),
		})
	end)
end

return ConnectingPage
