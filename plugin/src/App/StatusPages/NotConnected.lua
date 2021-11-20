local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local Config = require(Plugin.Config)

local Theme = require(Plugin.App.Theme)
local BorderedContainer = require(Plugin.App.Components.BorderedContainer)
local TextButton = require(Plugin.App.Components.TextButton)
local Header = require(Plugin.App.Components.Header)

local PORT_WIDTH = 74
local DIVIDER_WIDTH = 1
local HOST_OFFSET = 12

local lastHost, lastPort

local e = Roact.createElement

local function AddressEntry(props)
	return Theme.with(function(theme)
		return e(BorderedContainer, {
			transparency = props.transparency,
			size = UDim2.new(1, 0, 0, 36),
			layoutOrder = props.layoutOrder,
		}, {
			Host = e("TextBox", {
				Text = lastHost or "",
				Font = Enum.Font.Code,
				TextSize = 18,
				TextColor3 = theme.AddressEntry.TextColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = props.transparency,
				PlaceholderText = Config.defaultHost,
				PlaceholderColor3 = theme.AddressEntry.PlaceholderColor,

				Size = UDim2.new(1, -(HOST_OFFSET + DIVIDER_WIDTH + PORT_WIDTH), 1, 0),
				Position = UDim2.new(0, HOST_OFFSET, 0, 0),

				ClipsDescendants = true,
				BackgroundTransparency = 1,

				[Roact.Ref] = props.hostRef,
			}),

			Port = e("TextBox", {
				Text = lastPort or "",
				Font = Enum.Font.Code,
				TextSize = 18,
				TextColor3 = theme.AddressEntry.TextColor,
				TextTransparency = props.transparency,
				PlaceholderText = Config.defaultPort,
				PlaceholderColor3 = theme.AddressEntry.PlaceholderColor,

				Size = UDim2.new(0, PORT_WIDTH, 1, 0),
				Position = UDim2.new(1, 0, 0, 0),
				AnchorPoint = Vector2.new(1, 0),

				ClipsDescendants = true,
				BackgroundTransparency = 1,

				[Roact.Ref] = props.portRef,

				[Roact.Change.Text] = function(object)
					local text = object.Text
					text = text:gsub("%D", "")
					object.Text = text
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

function NotConnectedPage:init()
	self.hostRef = Roact.createRef()
	self.portRef = Roact.createRef()
end

function NotConnectedPage:render()
	return Roact.createFragment({
		Header = e(Header, {
			transparency = self.props.transparency,
			layoutOrder = 1,
		}),

		AddressEntry = e(AddressEntry, {
			hostRef = self.hostRef,
			portRef = self.portRef,
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
			}),

			Connect = e(TextButton, {
				text = "Connect",
				style = "Solid",
				transparency = self.props.transparency,
				layoutOrder = 2,
				onClick = function()
					local hostText = self.hostRef.current.Text
					local portText = self.portRef.current.Text

					lastHost = hostText
					lastPort = portText

					self.props.onConnect(
						#hostText > 0 and hostText or Config.defaultHost,
						#portText > 0 and portText or Config.defaultPort
					)
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