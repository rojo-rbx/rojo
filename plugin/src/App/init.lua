local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)
local Log = require(Rojo.Log)

local Assets = require(Plugin.Assets)
local Version = require(Plugin.Version)
local Config = require(Plugin.Config)
local strict = require(Plugin.strict)
local Dictionary = require(Plugin.Dictionary)
local ServeSession = require(Plugin.ServeSession)
local ApiContext = require(Plugin.ApiContext)
local preloadAssets = require(Plugin.preloadAssets)
local Theme = require(script.Theme)
local PluginSettings = require(script.PluginSettings)

local Page = require(script.Page)
local Notifications = require(script.Notifications)
local StudioPluginAction = require(script.Components.Studio.StudioPluginAction)
local StudioToolbar = require(script.Components.Studio.StudioToolbar)
local StudioToggleButton = require(script.Components.Studio.StudioToggleButton)
local StudioPluginGui = require(script.Components.Studio.StudioPluginGui)
local StudioPluginContext = require(script.Components.Studio.StudioPluginContext)
local StatusPages = require(script.StatusPages)

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
	preloadAssets()

	self.hostRef = Roact.createRef()
	self.portRef = Roact.createRef()

	self:setState({
		appStatus = AppStatus.NotConnected,
		guiEnabled = false,
		notifications = {},
		toolbarIcon = Assets.Images.PluginButton,
	})
end

function App:addNotification(text: string, timeout: number?)
	if not self.props.settings:get("showNotifications") then
		return
	end

	local notifications = table.clone(self.state.notifications)
	table.insert(notifications, {
		text = text,
		timestamp = DateTime.now().UnixTimestampMillis,
		timeout = timeout or 3,
	})

	self:setState({
		notifications = notifications,
	})
end

function App:closeNotification(index: number)
	local notifications = table.clone(self.state.notifications)
	table.remove(notifications, index)

	self:setState({
		notifications = notifications,
	})
end

function App:startSession()
	local hostText = self.hostRef.current.Text
	local portText = self.portRef.current.Text

	local host = if #hostText > 0 then hostText else Config.defaultHost
	local port = if #portText > 0 then portText else Config.defaultPort
	local sessionOptions = {
		openScriptsExternally = self.props.settings:get("openScriptsExternally"),
		twoWaySync = self.props.settings:get("twoWaySync"),
	}

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
				toolbarIcon = Assets.Images.PluginButton,
			})
			self:addNotification("Connecting to session...")
		elseif status == ServeSession.Status.Connected then
			local address = ("%s:%s"):format(host, port)
			self:setState({
				appStatus = AppStatus.Connected,
				projectName = details,
				address = address,
				toolbarIcon = Assets.Images.PluginButtonConnected,
			})
			self:addNotification(string.format("Connected to session '%s' at %s.", details, address), 5)
		elseif status == ServeSession.Status.Disconnected then
			self.serveSession = nil

			-- Details being present indicates that this
			-- disconnection was from an error.
			if details ~= nil then
				Log.warn("Disconnected from an error: {}", details)

				self:setState({
					appStatus = AppStatus.Error,
					errorMessage = tostring(details),
					toolbarIcon = Assets.Images.PluginButtonWarning,
				})
				self:addNotification(tostring(details), 10)
			else
				self:setState({
					appStatus = AppStatus.NotConnected,
					toolbarIcon = Assets.Images.PluginButton,
				})
				self:addNotification("Disconnected from session.")
			end
		end
	end)

	serveSession:start()

	self.serveSession = serveSession
end

function App:endSession()
	if self.serveSession == nil then
		return
	end

	Log.trace("Disconnecting session")

	self.serveSession:stop()
	self.serveSession = nil
	self:setState({
		appStatus = AppStatus.NotConnected,
	})

	Log.trace("Session terminated by user")
end

function App:render()
	local pluginName = "Rojo " .. Version.display(Config.version)

	local function createPageElement(appStatus, additionalProps)
		additionalProps = additionalProps or {}

		local props = Dictionary.merge(additionalProps, {
			component = StatusPages[appStatus],
			active = self.state.appStatus == appStatus,
		})

		return e(Page, props)
	end

	return e(StudioPluginContext.Provider, {
		value = self.props.plugin,
	}, {
		e(Theme.StudioProvider, nil, {
			e(PluginSettings.StudioProvider, {
				plugin = self.props.plugin,
			}, {
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
						hostRef = self.hostRef,
						portRef = self.portRef,

						onConnect = function()
							self:startSession()
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
							self:endSession()
						end,
					}),

					Settings = createPageElement(AppStatus.Settings, {
						onBack = function()
							self:setState({
								appStatus = AppStatus.NotConnected,
							})
						end,
					}),

					Error = createPageElement(AppStatus.Error, {
						errorMessage = self.state.errorMessage,

						onClose = function()
							self:setState({
								appStatus = AppStatus.NotConnected,
								toolbarIcon = Assets.Images.PluginButton,
							})
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

				RojoNotifications = e("ScreenGui", {}, {
					layout = Roact.createElement("UIListLayout", {
						SortOrder = Enum.SortOrder.LayoutOrder,
						HorizontalAlignment = Enum.HorizontalAlignment.Right,
						VerticalAlignment = Enum.VerticalAlignment.Bottom,
						Padding = UDim.new(0, 5),
					}),
					e(Notifications, {
						notifications = self.state.notifications,
						onClose = function(index)
							self:closeNotification(index)
						end,
					}),
				}),

				toggleAction = e(StudioPluginAction, {
					name = "RojoConnection",
					title = "Rojo: Connect/Disconnect",
					description = "Toggles the server for a Rojo sync session",
					icon = Assets.Images.PluginButton,
					bindable = true,
					onTriggered = function()
						if self.serveSession then
							self:endSession()
						else
							self:startSession()
						end
					end,
				}),

				connectAction = e(StudioPluginAction, {
					name = "RojoConnect",
					title = "Rojo: Connect",
					description = "Connects the server for a Rojo sync session",
					icon = Assets.Images.PluginButton,
					bindable = true,
					onTriggered = function()
						if self.serveSession then
							return
						end

						self:startSession()
					end,
				}),

				disconnectAction = e(StudioPluginAction, {
					name = "RojoDisconnect",
					title = "Rojo: Disconnect",
					description = "Disconnects the server for a Rojo sync session",
					icon = Assets.Images.PluginButton,
					bindable = true,
					onTriggered = function()
						if not self.serveSession then
							return
						end

						self:endSession()
					end,
				}),

				toolbar = e(StudioToolbar, {
					name = pluginName,
				}, {
					button = e(StudioToggleButton, {
						name = "Rojo",
						tooltip = "Show or hide the Rojo panel",
						icon = self.state.toolbarIcon,
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
			}),
		}),
	})
end

return function(props)
	return e(PluginSettings.StudioProvider, {
		plugin = props.plugin,
	}, {
		App = PluginSettings.with(function(settings)
			local settingsProps = Dictionary.merge(props, {
				settings = settings,
			})
			return e(App, settingsProps)
		end),
	})
end
