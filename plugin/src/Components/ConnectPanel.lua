local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local Config = require(Plugin.Config)

local Theme = require(Plugin.Components.Theme)
local Panel = require(Plugin.Components.Panel)
local FitList = require(Plugin.Components.FitList)
local FitText = require(Plugin.Components.FitText)
local FormButton = require(Plugin.Components.FormButton)
local FormTextInput = require(Plugin.Components.FormTextInput)
local PluginSettings = require(Plugin.Components.PluginSettings)

local e = Roact.createElement

local ConnectPanel = Roact.Component:extend("ConnectPanel")

function ConnectPanel:init()
	self:setState({
		address = "",
		port = "",
	})
end

function ConnectPanel:render()
	local startSession = self.props.startSession
	local openSettings = self.props.openSettings

	return Theme.with(function(theme)
		return PluginSettings.with(function(settings)
			return e(Panel, nil, {
				Layout = e("UIListLayout", {
					SortOrder = Enum.SortOrder.LayoutOrder,
					HorizontalAlignment = Enum.HorizontalAlignment.Center,
					VerticalAlignment = Enum.VerticalAlignment.Center,
				}),

				Inputs = e(FitList, {
					containerProps = {
						BackgroundTransparency = 1,
						LayoutOrder = 1,
					},
					layoutProps = {
						FillDirection = Enum.FillDirection.Horizontal,
						Padding = UDim.new(0, 8),
					},
					paddingProps = {
						PaddingTop = UDim.new(0, 20),
						PaddingBottom = UDim.new(0, 10),
						PaddingLeft = UDim.new(0, 24),
						PaddingRight = UDim.new(0, 24),
					},
				}, {
					Address = e(FitList, {
						containerProps = {
							LayoutOrder = 1,
							BackgroundTransparency = 1,
						},
						layoutProps = {
							Padding = UDim.new(0, 4),
						},
					}, {
						Label = e(FitText, {
							Kind = "TextLabel",
							LayoutOrder = 1,
							BackgroundTransparency = 1,
							TextXAlignment = Enum.TextXAlignment.Left,
							Font = theme.TitleFont,
							TextSize = 20,
							Text = "Address",
							TextColor3 = theme.Text1,
						}),

						Input = e(FormTextInput, {
							layoutOrder = 2,
							width = UDim.new(0, 220),
							value = self.state.address,
							placeholderValue = Config.defaultHost,
							onValueChange = function(newValue)
								self:setState({
									address = newValue,
								})
							end,
						}),
					}),

					Port = e(FitList, {
						containerProps = {
							LayoutOrder = 2,
							BackgroundTransparency = 1,
						},
						layoutProps = {
							Padding = UDim.new(0, 4),
						},
					}, {
						Label = e(FitText, {
							Kind = "TextLabel",
							LayoutOrder = 1,
							BackgroundTransparency = 1,
							TextXAlignment = Enum.TextXAlignment.Left,
							Font = theme.TitleFont,
							TextSize = 20,
							Text = "Port",
							TextColor3 = theme.Text1,
						}),

						Input = e(FormTextInput, {
							layoutOrder = 2,
							width = UDim.new(0, 80),
							value = self.state.port,
							placeholderValue = Config.defaultPort,
							onValueChange = function(newValue)
								self:setState({
									port = newValue,
								})
							end,
						}),
					}),
				}),

				Buttons = e(FitList, {
					fitAxes = "Y",
					containerProps = {
						BackgroundTransparency = 1,
						LayoutOrder = 2,
						Size = UDim2.new(1, 0, 0, 0),
					},
					layoutProps = {
						FillDirection = Enum.FillDirection.Horizontal,
						HorizontalAlignment = Enum.HorizontalAlignment.Right,
						Padding = UDim.new(0, 8),
					},
					paddingProps = {
						PaddingTop = UDim.new(0, 0),
						PaddingBottom = UDim.new(0, 20),
						PaddingLeft = UDim.new(0, 24),
						PaddingRight = UDim.new(0, 24),
					},
				}, {
					e(FormButton, {
						layoutOrder = 1,
						text = "Settings",
						secondary = true,
						onClick = function()
							if openSettings ~= nil then
								openSettings()
							end
						end,
					}),

					e(FormButton, {
						layoutOrder = 2,
						text = "Connect",
						onClick = function()
							if startSession ~= nil then
								local address = self.state.address
								if address:len() == 0 then
									address = Config.defaultHost
								end

								local port = self.state.port
								if port:len() == 0 then
									port = Config.defaultPort
								end

								local sessionOptions = {
									openScriptsExternally = settings:get("openScriptsExternally"),
								}

								startSession(address, port, sessionOptions)
							end
						end,
					}),
				}),
			})
		end)
	end)
end

return ConnectPanel