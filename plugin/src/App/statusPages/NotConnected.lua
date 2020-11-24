local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local Config = require(Plugin.Config)

local Theme = require(Plugin.App.Theme)
local BorderedContainer = require(Plugin.App.components.BorderedContainer)
local Button = require(Plugin.App.components.Button)
local Header = require(Plugin.App.components.Header)

local e = Roact.createElement

local function AddressEntry(props)
	return Theme.with(function(theme)
		return e(BorderedContainer, {
			transparency = props.transparency,
			size = UDim2.new(1, 0, 0, 36),
			layoutOrder = props.layoutOrder,
		}, {
			Address = e("TextBox", {
				Text = "",
				Font = Enum.Font.Code,
				TextSize = 18,
				TextColor3 = theme.AddressEntry.Text,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = props.transparency,
				PlaceholderText = Config.defaultHost,
				PlaceholderColor3 = theme.AddressEntry.Placeholder,

				Size = UDim2.new(1, -(74 + 1), 1, 0),
				Position = UDim2.new(0, 12, 0, 0),

				ClipsDescendants = true,
				BackgroundTransparency = 1,
			}),

			Port = e("TextBox", {
				Text = "",
				Font = Enum.Font.Code,
				TextSize = 18,
				TextColor3 = theme.AddressEntry.Text,
				TextTransparency = props.transparency,
				PlaceholderText = Config.defaultPort,
				PlaceholderColor3 = theme.AddressEntry.Placeholder,

				Size = UDim2.new(0, 74, 1, 0),
				Position = UDim2.new(1, 0, 0, 0),
				AnchorPoint = Vector2.new(1, 0),

				BackgroundTransparency = 1,

				[Roact.Change.Text] = function(object)
					local text = object.Text

					text = text:gsub("%D", "")
					text = text:sub(1, 5)

					object.Text = text
				end,
			}, {
				Divider = e("Frame", {
					BackgroundColor3 = theme.BorderedContainer.Border,
					BackgroundTransparency = props.transparency,
					Size = UDim2.new(0, 1, 1, 0),
					AnchorPoint = Vector2.new(1, 0),
					BorderSizePixel = 0,
				}),
			}),
		})
	end)
end

local NotConnectedPage = Roact.Component:extend("NotConnectedPage")

function NotConnectedPage:render()
	return Roact.createFragment({
		Header = e(Header, {
			transparency = self.props.transparency,
			layoutOrder = 1,
		}),

		AddressEntry = e(AddressEntry, {
			transparency = self.props.transparency,
			layoutOrder = 2,
		}),

		Buttons = e("Frame", {
			Size = UDim2.new(1, 0, 0, 34),
			LayoutOrder = 3,
			BackgroundTransparency = 1,
		}, {
			Settings = e(Button, {
				text = "Settings",
				style = "Bordered",
				transparency = self.props.transparency,
				layoutOrder = 1,
				onClick = function()

				end,
			}),

			Connect = e(Button, {
				text = "Connect",
				style = "Solid",
				transparency = self.props.transparency,
				layoutOrder = 2,
				onClick = function()

				end,
			}),

			Layout = e("UIListLayout", {
				HorizontalAlignment = Enum.HorizontalAlignment.Right,
				FillDirection = Enum.FillDirection.Horizontal,
				SortOrder = Enum.SortOrder.LayoutOrder,
				Padding = UDim.new(0, 10),
			}),
		}),

		Layout = e("UIListLayout", {
			HorizontalAlignment = Enum.HorizontalAlignment.Center,
			VerticalAlignment = Enum.VerticalAlignment.Center,
			FillDirection = Enum.FillDirection.Vertical,
			SortOrder = Enum.SortOrder.LayoutOrder,
			Padding = UDim.new(0, 10),
		}),

		Padding = e("UIPadding", {
			PaddingLeft = UDim.new(0, 20),
			PaddingRight = UDim.new(0, 20),
		}),
	})
end

return NotConnectedPage