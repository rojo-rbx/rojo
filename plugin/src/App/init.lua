local ChangeHistoryService = game:GetService("ChangeHistoryService")
local Players = game:GetService("Players")
local ServerStorage = game:GetService("ServerStorage")
local RunService = game:GetService("RunService")

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
local PatchSet = require(Plugin.PatchSet)
local preloadAssets = require(Plugin.preloadAssets)
local soundPlayer = require(Plugin.soundPlayer)
local ignorePlaceIds = require(Plugin.ignorePlaceIds)
local Theme = require(script.Theme)

local Page = require(script.Page)
local Notifications = require(script.Notifications)
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
	Connecting = "Connecting",
	Confirming = "Confirming",
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

	self.confirmationBindable = Instance.new("BindableEvent")
	self.confirmationEvent = self.confirmationBindable.Event
	self.notifId = 0

	self.waypointConnection = ChangeHistoryService.OnUndo:Connect(function(action: string)
		if not string.find(action, "^Rojo: Patch") then
			return
		end

		local undoConnection, redoConnection = nil, nil
		local function cleanup()
			undoConnection:Disconnect()
			redoConnection:Disconnect()
		end

		Log.warn(
			string.format(
				"You've undone '%s'.\nIf this was not intended, please Redo in the topbar or with Ctrl/⌘+Y.",
				action
			)
		)
		local dismissNotif = self:addNotification(
			string.format("You've undone '%s'.\nIf this was not intended, please restore.", action),
			10,
			{
				Restore = {
					text = "Restore",
					style = "Solid",
					layoutOrder = 1,
					onClick = function(notification)
						cleanup()
						notification:dismiss()
						ChangeHistoryService:Redo()
					end,
				},
				Dismiss = {
					text = "Dismiss",
					style = "Bordered",
					layoutOrder = 2,
					onClick = function(notification)
						cleanup()
						notification:dismiss()
					end,
				},
			}
		)

		undoConnection = ChangeHistoryService.OnUndo:Once(function()
			-- Our notif is now out of date- redoing will not restore the patch
			-- since we've undone even further. Dismiss the notif.
			cleanup()
			dismissNotif()
		end)
		redoConnection = ChangeHistoryService.OnRedo:Once(function(redoneAction: string)
			if redoneAction == action then
				-- The user has restored the patch, so we can dismiss the notif
				cleanup()
				dismissNotif()
			end
		end)
	end)

	self:setState({
		appStatus = AppStatus.NotConnected,
		guiEnabled = false,
		confirmData = {},
		patchData = {
			patch = PatchSet.newEmpty(),
			unapplied = PatchSet.newEmpty(),
			timestamp = os.time(),
		},
		notifications = {},
		toolbarIcon = Assets.Images.PluginButton,
	})

	if
		RunService:IsEdit()
		and self.serveSession == nil
		and Settings:get("syncReminder")
		and self:getLastSyncTimestamp()
	then
		self:addNotification("You've previously synced this place. Would you like to reconnect?", 300, {
			Connect = {
				text = "Connect",
				style = "Solid",
				layoutOrder = 1,
				onClick = function(notification)
					notification:dismiss()
					self:startSession()
				end,
			},
			Dismiss = {
				text = "Dismiss",
				style = "Bordered",
				layoutOrder = 2,
				onClick = function(notification)
					notification:dismiss()
				end,
			},
		})
	end
end

function App:willUnmount()
	self.waypointConnection:Disconnect()
	self.confirmationBindable:Destroy()
end

function App:addNotification(
	text: string,
	timeout: number?,
	actions: { [string]: { text: string, style: string, layoutOrder: number, onClick: (any) -> () } }?
)
	if not Settings:get("showNotifications") then
		return
	end

	self.notifId += 1
	local id = self.notifId

	local notifications = table.clone(self.state.notifications)
	notifications[id] = {
		text = text,
		timestamp = DateTime.now().UnixTimestampMillis,
		timeout = timeout or 3,
		actions = actions,
	}

	self:setState({
		notifications = notifications,
	})

	return function()
		self:closeNotification(id)
	end
end

function App:closeNotification(id: number)
	if not self.state.notifications[id] then
		return
	end

	local notifications = table.clone(self.state.notifications)
	notifications[id] = nil

	self:setState({
		notifications = notifications,
	})
end

function App:getPriorEndpoint()
	local priorEndpoints = Settings:get("priorEndpoints")
	if not priorEndpoints then
		return
	end

	local id = tostring(game.PlaceId)
	if ignorePlaceIds[id] then
		return
	end

	local place = priorEndpoints[id]
	if not place then
		return
	end

	return place.host, place.port
end

function App:getLastSyncTimestamp()
	local priorEndpoints = Settings:get("priorEndpoints")
	if not priorEndpoints then
		return
	end

	local id = tostring(game.PlaceId)
	if ignorePlaceIds[id] then
		return
	end

	local place = priorEndpoints[id]
	if not place then
		return
	end

	return place.timestamp
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

	local id = tostring(game.PlaceId)
	if ignorePlaceIds[id] then
		return
	end

	priorEndpoints[id] = {
		host = if host ~= Config.defaultHost then host else nil,
		port = if port ~= Config.defaultPort then port else nil,
		timestamp = os.time(),
	}
	Log.trace("Saved last used endpoint for {}", game.PlaceId)

	Settings:set("priorEndpoints", priorEndpoints)
end

function App:getHostAndPort()
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

function App:startSession()
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

	local host, port = self:getHostAndPort()

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
		local old = self.state.patchData

		if PatchSet.isEmpty(patch) then
			-- Ignore empty patch, but update timestamp
			self:setState({
				patchData = {
					patch = old.patch,
					unapplied = old.unapplied,
					timestamp = now,
				},
			})
			return
		end

		if now - old.timestamp < 2 then
			-- Patches that apply in the same second are
			-- considered to be part of the same change for human clarity
			patch = PatchSet.assign(PatchSet.newEmpty(), old.patch, patch)
			unapplied = PatchSet.assign(PatchSet.newEmpty(), old.unapplied, unapplied)
		end

		self:setState({
			patchData = {
				patch = patch,
				unapplied = unapplied,
				timestamp = now,
			},
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
			self:releaseSyncLock()
			self:setState({
				patchData = {
					patch = PatchSet.newEmpty(),
					unapplied = PatchSet.newEmpty(),
					timestamp = os.time(),
				},
			})

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

	serveSession:setConfirmCallback(function(instanceMap, patch, serverInfo)
		if PatchSet.isEmpty(patch) then
			Log.trace("Accepting patch without confirmation because it is empty")
			return "Accept"
		end

		-- The datamodel name gets overwritten by Studio, making confirmation of it intrusive
		-- and unnecessary. This special case allows it to be accepted without confirmation.
		if
			PatchSet.hasAdditions(patch) == false
			and PatchSet.hasRemoves(patch) == false
			and PatchSet.containsOnlyInstance(patch, instanceMap, game)
		then
			local datamodelUpdates = PatchSet.getUpdateForInstance(patch, instanceMap, game)
			if
				datamodelUpdates ~= nil
				and next(datamodelUpdates.changedProperties) == nil
				and datamodelUpdates.changedClassName == nil
			then
				Log.trace("Accepting patch without confirmation because it only contains a datamodel name change")
				return "Accept"
			end
		end

		self:setState({
			appStatus = AppStatus.Confirming,
			confirmData = {
				instanceMap = instanceMap,
				patch = patch,
				serverInfo = serverInfo,
			},
			toolbarIcon = Assets.Images.PluginButton,
		})

		self:addNotification(
			string.format(
				"Please accept%sor abort the initializing sync session.",
				Settings:get("twoWaySync") and ", reject, " or " "
			),
			7
		)

		return self.confirmationEvent:Wait()
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
			e(Tooltip.Provider, nil, {
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
							self.backPage = AppStatus.NotConnected
							self:setState({
								appStatus = AppStatus.Settings,
							})
						end,
					}),

					ConfirmingPage = createPageElement(AppStatus.Confirming, {
						confirmData = self.state.confirmData,
						createPopup = not self.state.guiEnabled,

						onAbort = function()
							self.confirmationBindable:Fire("Abort")
						end,
						onAccept = function()
							self.confirmationBindable:Fire("Accept")
						end,
						onReject = function()
							self.confirmationBindable:Fire("Reject")
						end,
					}),

					Connecting = createPageElement(AppStatus.Connecting),

					Connected = createPageElement(AppStatus.Connected, {
						projectName = self.state.projectName,
						address = self.state.address,
						patchData = self.state.patchData,
						serveSession = self.serveSession,

						onDisconnect = function()
							self:endSession()
						end,

						onNavigateSettings = function()
							self.backPage = AppStatus.Connected
							self:setState({
								appStatus = AppStatus.Settings,
							})
						end,
					}),

					Settings = createPageElement(AppStatus.Settings, {
						onBack = function()
							self:setState({
								appStatus = self.backPage or AppStatus.NotConnected,
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
				}),

				RojoNotifications = e("ScreenGui", {
					ZIndexBehavior = Enum.ZIndexBehavior.Sibling,
					ResetOnSpawn = false,
					DisplayOrder = 100,
				}, {
					layout = e("UIListLayout", {
						SortOrder = Enum.SortOrder.LayoutOrder,
						HorizontalAlignment = Enum.HorizontalAlignment.Right,
						VerticalAlignment = Enum.VerticalAlignment.Bottom,
						Padding = UDim.new(0, 5),
					}),
					padding = e("UIPadding", {
						PaddingTop = UDim.new(0, 5),
						PaddingBottom = UDim.new(0, 5),
						PaddingLeft = UDim.new(0, 5),
						PaddingRight = UDim.new(0, 5),
					}),
					notifs = e(Notifications, {
						soundPlayer = self.props.soundPlayer,
						notifications = self.state.notifications,
						onClose = function(id)
							self:closeNotification(id)
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
					elseif
						self.serveSession ~= nil and self.serveSession:getStatus() == ServeSession.Status.Connected
					then
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
				}),
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
