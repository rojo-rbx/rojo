local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Config = require(Plugin.Config)

local Theme = require(Plugin.App.Theme)
local BorderedContainer = require(Plugin.App.Components.BorderedContainer)
local TextButton = require(Plugin.App.Components.TextButton)
local Header = require(Plugin.App.Components.Header)
local Tooltip = require(Plugin.App.Components.Tooltip)

local PORT_WIDTH = 74
local DIVIDER_WIDTH = 1
local HOST_OFFSET = 12

local e = Roact.createElement

local function AddressEntry(props)
	return Theme.with(function(theme)
		return e(BorderedContainer, {
			transparency = props.transparency,
			size = UDim2.new(1, 0, 0, 36),
			layoutOrder = props.layoutOrder,
		}, {
			Host = e("TextBox", {
				Text = props.host or "",
				Font = Enum.Font.Code,
				TextSize = 18,
				TextColor3 = theme.AddressEntry.TextColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = props.transparency,
				PlaceholderText = Config.defaultHost,
				PlaceholderColor3 = theme.AddressEntry.PlaceholderColor,
				ClearTextOnFocus = false,

				Size = UDim2.new(1, -(HOST_OFFSET + DIVIDER_WIDTH + PORT_WIDTH), 1, 0),
				Position = UDim2.new(0, HOST_OFFSET, 0, 0),

				ClipsDescendants = true,
				BackgroundTransparency = 1,

				[Roact.Change.Text] = function(object)
					if props.onHostChange ~= nil then
						props.onHostChange(object.Text)
					end
				end
			}),

			Port = e("TextBox", {
				Text = props.port or "",
				Font = Enum.Font.Code,
				TextSize = 18,
				TextColor3 = theme.AddressEntry.TextColor,
				TextTransparency = props.transparency,
				PlaceholderText = Config.defaultPort,
				PlaceholderColor3 = theme.AddressEntry.PlaceholderColor,
				ClearTextOnFocus = false,

				Size = UDim2.new(0, PORT_WIDTH, 1, 0),
				Position = UDim2.new(1, 0, 0, 0),
				AnchorPoint = Vector2.new(1, 0),

				ClipsDescendants = true,
				BackgroundTransparency = 1,

				[Roact.Change.Text] = function(object)
					local text = object.Text
					text = text:gsub("%D", "")
					object.Text = text

					if props.onPortChange ~= nil then
						props.onPortChange(text)
					end
				end,
			}, {
				Divider = e("Frame", {
					BackgroundColor3 = theme.BorderedContainer.BorderColor,
					BackgroundTransparency = props.transparency,
					Size = UDim2.new(0, DIVIDER_WIDTH, 1, 0),
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
			host = self.props.host,
			port = self.props.port,
			onHostChange = self.props.onHostChange,
			onPortChange = self.props.onPortChange,
			transparency = self.props.transparency,
			layoutOrder = 2,
		}),

		Buttons = e("Frame", {
			Size = UDim2.new(1, 0, 0, 34),
			LayoutOrder = 3,
			BackgroundTransparency = 1,
		}, {
			Settings = e(TextButton, {
				text = "Settings",
				style = "Bordered",
				transparency = self.props.transparency,
				layoutOrder = 1,
				onClick = self.props.onNavigateSettings,
			}, {
				Tip = e(Tooltip.Trigger, {
					text = "View and modify plugin settings"
				}),
			}),

			Connect = e(TextButton, {
				text = "Connect",
				style = "Solid",
				transparency = self.props.transparency,
				layoutOrder = 2,
				onClick = self.props.onConnect,
			}, {
				Tip = e(Tooltip.Trigger, {
					text = "Connect to a Rojo sync server"
				}),
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
