local Players = game:GetService("Players")
local ServerStorage = game:GetService("ServerStorage")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Log = require(Packages.Log)

local Assets = require(Plugin.Assets)
local Version = require(Plugin.Version)
local Config = require(Plugin.Config)
local Settings = require(Plugin.Settings)
local strict = require(Plugin.strict)
local Dictionary = require(Plugin.Dictionary)
local ServeSession = require(Plugin.ServeSession)
local ApiContext = require(Plugin.ApiContext)
local HeadlessAPI = require(Plugin.HeadlessAPI)
local preloadAssets = require(Plugin.preloadAssets)
local soundPlayer = require(Plugin.soundPlayer)
local Theme = require(script.Theme)

local Page = require(script.Page)
local Notifications = require(script.Notifications)
local PermissionPopup = require(script.PermissionPopup)
local ConflictAPIPopup = require(script.ConflictAPIPopup)
local Tooltip = require(script.Components.Tooltip)
local StudioPluginAction = require(script.Components.Studio.StudioPluginAction)
local StudioToolbar = require(script.Components.Studio.StudioToolbar)
local StudioToggleButton = require(script.Components.Studio.StudioToggleButton)
local StudioPluginGui = require(script.Components.Studio.StudioPluginGui)
local StudioPluginContext = require(script.Components.Studio.StudioPluginContext)
local StatusPages = require(script.StatusPages)

local AppStatus = strict("AppStatus", {
	NotConnected = "NotConnected",
	Settings = "Settings",
	Permissions = "Permissions",
	Connecting = "Connecting",
	Connected = "Connected",
	Error = "Error",
})

local e = Roact.createElement

local App = Roact.Component:extend("App")

function App:init()
	preloadAssets()

	local priorHost, priorPort = self:getPriorEndpoint()
	self.host, self.setHost = Roact.createBinding(priorHost or "")
	self.port, self.setPort = Roact.createBinding(priorPort or "")

	self.patchInfo, self.setPatchInfo = Roact.createBinding({
		changes = 0,
		timestamp = os.time(),
	})

	self:setState({
		appStatus = AppStatus.NotConnected,
		guiEnabled = false,
		notifications = {},
		toolbarIcon = Assets.Images.PluginButton,
		popups = {},
	})

	self.headlessAPI, self.readOnlyHeadlessAPI = HeadlessAPI.new(self)

	-- selene: allow(global_usage)
	local existingAPI = _G.Rojo
	if existingAPI then
		local responseEvent = Instance.new("BindableEvent")
		responseEvent.Event:Once(function(accepted)
			if accepted then
				-- selene: allow(global_usage)
				_G.Rojo = self.readOnlyHeadlessAPI -- Expose headless to other plugins and command bar
			end

			responseEvent:Destroy()
			self:setState(function(state)
				state.popups["apiReplacement"] = nil
				return state
			end)
		end)

		self:setState(function(state)
			state.popups["apiReplacement"] = {
				name = "Headless API Conflict",
				dockState = Enum.InitialDockState.Float,
				onClose = function()
					responseEvent:Fire(false)
				end,
				content = e(ConflictAPIPopup, {
					existingAPI = existingAPI,
					onAccept = function()
						responseEvent:Fire(true)
					end,
					onDeny = function()
						responseEvent:Fire(false)
					end,
					transparency = Roact.createBinding(0),
				}),
			}
			return state
		end)
	else
		-- selene: allow(global_usage)
		_G.Rojo = self.readOnlyHeadlessAPI -- Expose headless to other plugins and command bar
	end
end

function App:addNotification(text: string, timeout: number?)
	if not Settings:get("showNotifications") then
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

function App:addThirdPartyNotification(source: string, text: string, timeout: number?)
	if not Settings:get("showNotifications") then
		return
	end

	local notifications = table.clone(self.state.notifications)
	table.insert(notifications, {
		text = text,
		timestamp = DateTime.now().UnixTimestampMillis,
		timeout = timeout or 3,
		thirdParty = true,
		source = source,
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

function App:getPriorEndpoint()
	local priorEndpoints = Settings:get("priorEndpoints")
	if not priorEndpoints then return end

	local place = priorEndpoints[tostring(game.PlaceId)]
	if not place then return end

	return place.host, place.port
end

function App:setPriorEndpoint(host: string, port: string)
	local priorEndpoints = Settings:get("priorEndpoints")
	if not priorEndpoints then
		priorEndpoints = {}
	end

	-- Clear any stale saves to avoid disc bloat
	for placeId, endpoint in priorEndpoints do
		if os.time() - endpoint.timestamp > 12_960_000 then
			priorEndpoints[placeId] = nil
			Log.trace("Cleared stale saved endpoint for {}", placeId)
		end
	end

	if host == Config.defaultHost and port == Config.defaultPort then
		-- Don't save default
		priorEndpoints[tostring(game.PlaceId)] = nil
	else
		priorEndpoints[tostring(game.PlaceId)] = {
			host = host ~= Config.defaultHost and host or nil,
			port = port ~= Config.defaultPort and port or nil,
			timestamp = os.time(),
		}
		Log.trace("Saved last used endpoint for {}", game.PlaceId)
	end

	Settings:set("priorEndpoints", priorEndpoints)
end

function App:getHostAndPort(): (string, string)
	local host = self.host:getValue()
	local port = self.port:getValue()

	local host = if #host > 0 then host else Config.defaultHost
	local port = if #port > 0 then port else Config.defaultPort

	return host, port
end

function App:claimSyncLock()
	if #Players:GetPlayers() == 0 then
		Log.trace("Skipping sync lock because this isn't in Team Create")
		return true
	end

	local lock = ServerStorage:FindFirstChild("__Rojo_SessionLock")
	if not lock then
		lock = Instance.new("ObjectValue")
		lock.Name = "__Rojo_SessionLock"
		lock.Archivable = false
		lock.Value = Players.LocalPlayer
		lock.Parent = ServerStorage
		Log.trace("Created and claimed sync lock")
		return true
	end

	if lock.Value and lock.Value ~= Players.LocalPlayer and lock.Value.Parent then
		Log.trace("Found existing sync lock owned by {}", lock.Value)
		return false, lock.Value
	end

	lock.Value = Players.LocalPlayer
	Log.trace("Claimed existing sync lock")
	return true
end

function App:releaseSyncLock()
	local lock = ServerStorage:FindFirstChild("__Rojo_SessionLock")
	if not lock then
		Log.trace("No sync lock found, assumed released")
		return
	end

	if lock.Value == Players.LocalPlayer then
		lock.Value = nil
		Log.trace("Released sync lock")
		return
	end

	Log.trace("Could not relase sync lock because it is owned by {}", lock.Value)
end

function App:requestPermission(source: string, name: string, apis: {string}, initialState: {[string]: boolean?}): {[string]: boolean}
	local responseEvent = Instance.new("BindableEvent")

	self:setState(function(state)
		state.popups[source  .. " Permissions"] = {
			name = name,
			content = e(PermissionPopup, {
				responseEvent = responseEvent,
				initialState = initialState,
				source = source,
				name = name,
				apis = apis,
				apiDescriptions = self.headlessAPI._apiDescriptions,
				transparency = Roact.createBinding(0),
			}),
			onClose = function()
				responseEvent:Fire(initialState)
			end,
		}
		return state
	end)

	local response = responseEvent.Event:Wait()
	responseEvent:Destroy()

	self:setState(function(state)
		state.popups[source  .. " Permissions"] = nil
		return state
	end)

	return response
end

function App:startSession(host: string?, port: string?)
	local claimedLock, priorOwner = self:claimSyncLock()
	if not claimedLock then
		local msg = string.format("Could not sync because user '%s' is already syncing", tostring(priorOwner))

		Log.warn(msg)
		self:addNotification(msg, 10)
		self:setState({
			appStatus = AppStatus.Error,
			errorMessage = msg,
			toolbarIcon = Assets.Images.PluginButtonWarning,
		})

		return
	end

	if host == nil or port == nil then
		host, port = self:getHostAndPort()
	end

	local sessionOptions = {
		openScriptsExternally = Settings:get("openScriptsExternally"),
		twoWaySync = Settings:get("twoWaySync"),
	}

	local baseUrl = if string.find(host, "^https?://")
		then string.format("%s:%s", host, port)
		else string.format("http://%s:%s", host, port)
	local apiContext = ApiContext.new(baseUrl)

	local serveSession = ServeSession.new({
		apiContext = apiContext,
		openScriptsExternally = sessionOptions.openScriptsExternally,
		twoWaySync = sessionOptions.twoWaySync,
	})

	serveSession:onPatchApplied(function(patch, unapplied)
		local now = os.time()
		local changes = 0

		for _, set in patch do
			for _ in set do
				changes += 1
			end
		end
		for _, set in unapplied do
			for _ in set do
				changes -= 1
			end
		end

		if changes == 0 then return end

		local old = self.patchInfo:getValue()
		if now - old.timestamp < 2 then
			changes += old.changes
		end

		self.setPatchInfo({
			changes = changes,
			timestamp = now,
		})
	end)

	serveSession:onStatusChanged(function(status, details)
		if status == ServeSession.Status.Connecting then
			self:setPriorEndpoint(host, port)

			self:setState({
				appStatus = AppStatus.Connecting,
				toolbarIcon = Assets.Images.PluginButton,
			})
			self:addNotification("Connecting to session...")
		elseif status == ServeSession.Status.Connected then
			local address = string.format("%s:%s", host :: string, port :: string)

			self.headlessAPI:_updateProperty("Address", address)
			self.headlessAPI:_updateProperty("ProjectName", details)

			self:setState({
				appStatus = AppStatus.Connected,
				projectName = details,
				address = address,
				toolbarIcon = Assets.Images.PluginButtonConnected,
			})
			self:addNotification(string.format("Connected to session '%s' at %s.", details, address), 5)
		elseif status == ServeSession.Status.Disconnected then
			self.serveSession = nil
			self:releaseSyncLock()

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

		self.headlessAPI:_updateProperty("Connected", status == ServeSession.Status.Connected)
		if not self.headlessAPI.Connected then
			self.headlessAPI:_updateProperty("Address", nil)
			self.headlessAPI:_updateProperty("ProjectName", nil)
		end
	end)

	serveSession:start()

	self.serveSession = serveSession

	task.defer(function()
		while self.serveSession == serveSession do
			-- Trigger rerender to update timestamp text
			local patchInfo = table.clone(self.patchInfo:getValue())
			self.setPatchInfo(patchInfo)
			local elapsed = os.time() - patchInfo.timestamp
			task.wait(elapsed < 60 and 1 or elapsed/5)
		end
	end)
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

	local popups = {}
	for id, popup in self.state.popups do
		popups["Rojo_"..id] = e(StudioPluginGui, {
			id = id,
			title = popup.name,
			active = true,

			initDockState = popup.dockState or Enum.InitialDockState.Top,
			initEnabled = true,
			overridePreviousState = true,
			floatingSize = Vector2.new(400, 300),
			minimumSize = Vector2.new(390, 240),

			zIndexBehavior = Enum.ZIndexBehavior.Sibling,

			onClose = popup.onClose,
		}, {
			Content = popup.content,

			Background = Theme.with(function(theme)
				return e("Frame", {
					Size = UDim2.new(1, 0, 1, 0),
					BackgroundColor3 = theme.BackgroundColor,
					ZIndex = 0,
					BorderSizePixel = 0,
				})
			end),
		})
	end

	return e(StudioPluginContext.Provider, {
		value = self.props.plugin,
	}, {
		e(Theme.StudioProvider, nil, {
			e(Tooltip.Provider, nil, {
				popups = Roact.createFragment(popups),

				gui = e(StudioPluginGui, {
					id = pluginName,
					title = pluginName,
					active = self.state.guiEnabled,

					initDockState = Enum.InitialDockState.Right,
					initEnabled = false,
					overridePreviousState = false,
					floatingSize = Vector2.new(300, 200),
					minimumSize = Vector2.new(300, 120),

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
					Tooltips = e(Tooltip.Container, nil),

					NotConnectedPage = createPageElement(AppStatus.NotConnected, {
						host = self.host,
						onHostChange = self.setHost,
						port = self.port,
						onPortChange = self.setPort,

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
						patchInfo = self.patchInfo,

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

						onNavigatePermissions = function()
							self:setState({
								appStatus = AppStatus.Permissions,
							})
						end,
					}),

					Permissions = createPageElement(AppStatus.Permissions, {
						headlessAPI = self.headlessAPI,

						onBack = function()
							self:setState({
								appStatus = AppStatus.Settings,
							})
						end,

						onEdit = function(source, meta, apiMap)
							local apiList = {}
							for api in apiMap do
								table.insert(apiList, api)
							end
							self:requestPermission(
								source,
								meta.Name .. if meta.Creator then " by " .. meta.Creator else "",
								apiList,
								apiMap
							)
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
					layout = e("UIListLayout", {
						SortOrder = Enum.SortOrder.LayoutOrder,
						HorizontalAlignment = Enum.HorizontalAlignment.Right,
						VerticalAlignment = Enum.VerticalAlignment.Bottom,
						Padding = UDim.new(0, 5),
					}),
					padding = e("UIPadding", {
						PaddingTop = UDim.new(0, 5);
						PaddingBottom = UDim.new(0, 5);
						PaddingLeft = UDim.new(0, 5);
						PaddingRight = UDim.new(0, 5);
					}),
					notifs = e(Notifications, {
						soundPlayer = self.props.soundPlayer,
						notifications = self.state.notifications,
						onClose = function(index)
							self:closeNotification(index)
						end,
					}),
				}),
			}),

			toggleAction = e(StudioPluginAction, {
				name = "RojoConnection",
				title = "Rojo: Connect/Disconnect",
				description = "Toggles the server for a Rojo sync session",
				icon = Assets.Images.PluginButton,
				bindable = true,
				onTriggered = function()
					if self.serveSession == nil or self.serveSession:getStatus() == ServeSession.Status.NotStarted then
						self:startSession()
					elseif self.serveSession ~= nil and self.serveSession:getStatus() == ServeSession.Status.Connected then
						self:endSession()
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
					if self.serveSession == nil or self.serveSession:getStatus() == ServeSession.Status.NotStarted then
						self:startSession()
					end
				end,
			}),

			disconnectAction = e(StudioPluginAction, {
				name = "RojoDisconnect",
				title = "Rojo: Disconnect",
				description = "Disconnects the server for a Rojo sync session",
				icon = Assets.Images.PluginButton,
				bindable = true,
				onTriggered = function()
					if self.serveSession ~= nil and self.serveSession:getStatus() == ServeSession.Status.Connected then
						self:endSession()
					end
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
	})
end

return function(props)
	local mergedProps = Dictionary.merge(props, {
		soundPlayer = soundPlayer.new(Settings),
	})

	return e(App, mergedProps)
end
