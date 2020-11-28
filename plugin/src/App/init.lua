local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)
local Log = require(Rojo.Log)

local Assets = require(Plugin.Assets)
local Version = require(Plugin.Version)
local Config = require(Plugin.Config)
local strict = require(Plugin.strict)
local merge = require(Plugin.merge)
local ServeSession = require(Plugin.ServeSession)
local ApiContext = require(Plugin.ApiContext)

local Theme = require(script.Theme)
local Page = require(script.Page)
local StudioToolbar = require(script.components.studio.StudioToolbar)
local StudioToggleButton = require(script.components.studio.StudioToggleButton)
local StudioPluginGui = require(script.components.studio.StudioPluginGui)
local StudioPluginContext = require(script.components.studio.StudioPluginContext)
local statusPages = require(script.statusPages)

local AppStatus = strict("AppStatus", {
	NotConnected = "NotConnected",
	Settings = "Settings",
	Connecting = "Connecting",
	Connected = "Connected",
	Error = "Error",
})

local e = Roact.createElement

local App = Roact.Component:extend("App")

function App:init()
	self:setState({
		appStatus = AppStatus.NotConnected,
		guiEnabled = false,
	})
end

function App:startSession(host, port, sessionOptions)
	local baseUrl = ("http://%s:%s"):format(host, port)
	local apiContext = ApiContext.new(baseUrl)

	local serveSession = ServeSession.new({
		apiContext = apiContext,
		openScriptsExternally = sessionOptions.openScriptsExternally,
		twoWaySync = sessionOptions.twoWaySync,
	})

	serveSession:onStatusChanged(function(status, details)
		if status == ServeSession.Status.Connecting then
			self:setState({
				appStatus = AppStatus.Connecting,
			})
		elseif status == ServeSession.Status.Connected then
			local address = ("%s:%s"):format(host, port)
			self:setState({
				appStatus = AppStatus.Connected,
				projectName = details,
				address = address,
			})
		elseif status == ServeSession.Status.Disconnected then
			self.serveSession = nil

			-- Details being present indicates that this
			-- disconnection was from an error.
			if details ~= nil then
				Log.warn("Disconnected from an error: {}", details)

				self:setState({
					appStatus = AppStatus.Error,
					errorMessage = tostring(details),
				})
			else
				self:setState({
					appStatus = AppStatus.NotConnected,
				})
			end
		end
	end)

	serveSession:start()

	self.serveSession = serveSession
end

function App:render()
	local pluginName = "Rojo " .. Version.display(Config.version)

	local function createPageElement(appStatus, additionalProps)
		additionalProps = additionalProps or {}

		local props = merge(additionalProps, {
			component = statusPages[appStatus],
			active = self.state.appStatus == appStatus,
		})

		return e(Page, props)
	end

	return e(StudioPluginContext.Provider, {
		value = self.props.plugin,
	}, {
		e(Theme.StudioProvider, nil, {
			gui = e(StudioPluginGui, {
				id = pluginName,
				title = pluginName,
				active = self.state.guiEnabled,

				initDockState = Enum.InitialDockState.Right,
				initEnabled = false,
				overridePreviousState = false,
				floatingSize = Vector2.new(300, 200),
				minimumSize = Vector2.new(300, 200),

				zIndexBehavior = Enum.ZIndexBehavior.Sibling,

				onInitialState = function(initialState)
					self:setState({
						guiEnabled = initialState,
					})
				end,

				onClose = function()
					self:setState({
						guiEnabled = false,
					})
				end,
			}, {
				NotConnectedPage = createPageElement(AppStatus.NotConnected, {
					onConnect = function(address, port)
						-- TODO: Settings
						self:startSession(address, port, {
							openScriptsExternally = false,
							twoWaySync = false,
						})
					end,

					onNavigateSettings = function()
						self:setState({
							appStatus = AppStatus.Settings,
						})
					end,
				}),

				Connecting = createPageElement(AppStatus.Connecting),

				Connected = createPageElement(AppStatus.Connected, {
					projectName = self.state.projectName,
					address = self.state.address,

					onDisconnect = function()
						Log.trace("Disconnecting session")

						self.serveSession:stop()
						self.serveSession = nil
						self:setState({
							appStatus = AppStatus.NotConnected,
						})

						Log.trace("Session terminated by user")
					end,
				}),

				Background = Theme.with(function(theme)
					return e("Frame", {
						Size = UDim2.new(1, 0, 1, 0),
						BackgroundColor3 = theme.BackgroundColor,
						ZIndex = 0,
						BorderSizePixel = 0,
					})
				end),
			}),

			toolbar = e(StudioToolbar, {
				name = pluginName,
			}, {
				button = e(StudioToggleButton, {
					name = "Rojo",
					tooltip = "Show or hide the Rojo panel",
					icon = Assets.Images.Icon,
					active = self.state.guiEnabled,
					enabled = true,
					onClick = function()
						self:setState(function(state)
							return {
								guiEnabled = not state.guiEnabled,
							}
						end)
					end,
				})
			}),
		})
	})
end

return App