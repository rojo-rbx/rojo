local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Branding = require(Plugin.Branding)
local Config = require(Plugin.Config)
local Version = require(Plugin.Version)

local Theme = require(Plugin.App.Theme)
local BorderedContainer = require(Plugin.App.Components.BorderedContainer)
local TextButton = require(Plugin.App.Components.TextButton)
local Header = require(Plugin.App.Components.Header)
local Tooltip = require(Plugin.App.Components.Tooltip)

local PORT_WIDTH = 76
local DIVIDER_WIDTH = 1
local HOST_OFFSET = 12

local e = Roact.createElement

local AddressEntry = Roact.Component:extend("AddressEntry")

function AddressEntry:init()
	self:setState({
		focused = false,
	})
end

function AddressEntry:render()
	return Theme.with(function(theme)
		local function focus()
			self:setState({
				focused = true,
			})
		end

		local function blur()
			self:setState({
				focused = false,
			})
		end

		return e(BorderedContainer, {
			transparency = self.props.transparency,
			borderColor = if self.state.focused
				then theme.AddressEntry.FocusBorderColor
				else theme.BorderedContainer.BorderColor,
			size = UDim2.new(1, 0, 0, 64),
			layoutOrder = self.props.layoutOrder,
		}, {
			Status = e("Frame", {
				Size = UDim2.new(1, -22, 0, 22),
				Position = UDim2.fromOffset(11, 3),
				BackgroundTransparency = 1,
			}, {
				Dot = e("Frame", {
					Size = UDim2.fromOffset(7, 7),
					Position = UDim2.new(0, 0, 0.5, 0),
					AnchorPoint = Vector2.new(0, 0.5),
					BackgroundColor3 = theme.Tokens.Warning,
					BackgroundTransparency = self.props.transparency,
					BorderSizePixel = 0,
				}, {
					Corner = e("UICorner", {
						CornerRadius = UDim.new(1, 0),
					}),
				}),
				Text = e("TextLabel", {
					Text = "Disconnected",
					FontFace = theme.Font.Main,
					TextSize = theme.TextSize.Small,
					TextColor3 = theme.Tokens.SecondaryText,
					TextTransparency = self.props.transparency,
					TextXAlignment = Enum.TextXAlignment.Left,
					BackgroundTransparency = 1,
					Position = UDim2.fromOffset(14, 0),
					Size = UDim2.new(1, -14, 1, 0),
				}),
			}),

			Host = e("TextBox", {
				Text = self.props.host or "",
				FontFace = theme.Font.Code,
				TextSize = theme.TextSize.Medium,
				TextColor3 = theme.AddressEntry.TextColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = self.props.transparency,
				PlaceholderText = Config.defaultHost,
				PlaceholderColor3 = theme.AddressEntry.PlaceholderColor,
				ClearTextOnFocus = false,
				Selectable = true,
				Size = UDim2.new(1, -(HOST_OFFSET + DIVIDER_WIDTH + PORT_WIDTH), 0, 37),
				Position = UDim2.new(0, HOST_OFFSET, 0, 25),
				ClipsDescendants = true,
				BackgroundTransparency = 1,
				[Roact.Event.Focused] = focus,
				[Roact.Event.FocusLost] = blur,
				[Roact.Change.Text] = function(object)
					if self.props.onHostChange ~= nil then
						self.props.onHostChange(object.Text)
					end
				end,
			}),

			Port = e("TextBox", {
				Text = self.props.port or "",
				FontFace = theme.Font.Code,
				TextSize = theme.TextSize.Medium,
				TextColor3 = theme.AddressEntry.TextColor,
				TextTransparency = self.props.transparency,
				PlaceholderText = Config.defaultPort,
				PlaceholderColor3 = theme.AddressEntry.PlaceholderColor,
				ClearTextOnFocus = false,
				Selectable = true,
				Size = UDim2.new(0, PORT_WIDTH, 0, 37),
				Position = UDim2.new(1, 0, 0, 25),
				AnchorPoint = Vector2.new(1, 0),
				ClipsDescendants = true,
				BackgroundTransparency = 1,
				[Roact.Event.Focused] = focus,
				[Roact.Event.FocusLost] = blur,
				[Roact.Change.Text] = function(object)
					local text = object.Text:gsub("%D", "")
					object.Text = text

					if self.props.onPortChange ~= nil then
						self.props.onPortChange(text)
					end
				end,
			}, {
				Divider = e("Frame", {
					BackgroundColor3 = theme.BorderedContainer.BorderColor,
					BackgroundTransparency = self.props.transparency,
					Size = UDim2.new(0, DIVIDER_WIDTH, 1, -8),
					Position = UDim2.new(0, 0, 0, 4),
					AnchorPoint = Vector2.new(1, 0),
					BorderSizePixel = 0,
				}),
			}),
		})
	end)
end

local NotConnectedPage = Roact.Component:extend("NotConnectedPage")

function NotConnectedPage:render()
	return Theme.with(function(theme)
		return e("Frame", {
			Size = UDim2.fromScale(1, 1),
			BackgroundTransparency = 1,
		}, {
			Header = e(Header, {
				full = true,
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
				ZIndex = 2,
			}, {
				Settings = e(TextButton, {
					text = "Settings",
					style = "Bordered",
					transparency = self.props.transparency,
					layoutOrder = 1,
					onClick = self.props.onNavigateSettings,
				}, {
					Tip = e(Tooltip.Trigger, {
						text = "View and modify plugin settings",
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
						text = "Connect to a Prism sync server",
					}),
				}),

				Layout = e("UIListLayout", {
					HorizontalAlignment = Enum.HorizontalAlignment.Right,
					FillDirection = Enum.FillDirection.Horizontal,
					SortOrder = Enum.SortOrder.LayoutOrder,
					Padding = UDim.new(0, 10),
				}),
			}),

			Version = e("TextLabel", {
				Text = `{Branding.Name} {Version.display(Config.version)}`,
				FontFace = theme.Font.Thin,
				TextSize = theme.TextSize.Small,
				TextColor3 = theme.Tokens.MutedText,
				TextTransparency = self.props.transparency,
				BackgroundTransparency = 1,
				Size = UDim2.new(1, 0, 0, 14),
				LayoutOrder = 4,
			}),

			Layout = e("UIListLayout", {
				HorizontalAlignment = Enum.HorizontalAlignment.Center,
				VerticalAlignment = Enum.VerticalAlignment.Center,
				FillDirection = Enum.FillDirection.Vertical,
				SortOrder = Enum.SortOrder.LayoutOrder,
				Padding = UDim.new(0, 8),
			}),

			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, 16),
				PaddingRight = UDim.new(0, 16),
			}),
		})
	end)
end

return NotConnectedPage
