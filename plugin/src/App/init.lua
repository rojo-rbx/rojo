local ChangeHistoryService = game:GetService("ChangeHistoryService")
local Players = game:GetService("Players")
local ServerStorage = game:GetService("ServerStorage")
local RunService = game:GetService("RunService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Log = require(Packages.Log)
local Promise = require(Packages.Promise)

local Assets = require(Plugin.Assets)
local Version = require(Plugin.Version)
local Config = require(Plugin.Config)
local Settings = require(Plugin.Settings)
local strict = require(Plugin.strict)
local Dictionary = require(Plugin.Dictionary)
local ServeSession = require(Plugin.ServeSession)
local ApiContext = require(Plugin.ApiContext)
local PatchSet = require(Plugin.PatchSet)
local PatchTree = require(Plugin.PatchTree)
local preloadAssets = require(Plugin.preloadAssets)
local soundPlayer = require(Plugin.soundPlayer)
local ignorePlaceIds = require(Plugin.ignorePlaceIds)
local timeUtil = require(Plugin.timeUtil)
local Theme = require(script.Theme)

local Page = require(script.Page)
local Notifications = require(script.Components.Notifications)
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

	local priorSyncInfo = self:getPriorSyncInfo()
	self.host, self.setHost = Roact.createBinding(priorSyncInfo.host or "")
	self.port, self.setPort = Roact.createBinding(priorSyncInfo.port or "")

	self.confirmationBindable = Instance.new("BindableEvent")
	self.confirmationEvent = self.confirmationBindable.Event
	self.knownProjects = {}
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
		local dismissNotif = self:addNotification({
			text = string.format("You've undone '%s'.\nIf this was not intended, please restore.", action),
			timeout = 10,
			onClose = function()
				cleanup()
			end,
			actions = {
				Restore = {
					text = "Restore",
					style = "Solid",
					layoutOrder = 1,
					onClick = function()
						ChangeHistoryService:Redo()
					end,
				},
				Dismiss = {
					text = "Dismiss",
					style = "Bordered",
					layoutOrder = 2,
				},
			},
		})

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

	self.disconnectUpdatesCheckChanged = Settings:onChanged("checkForUpdates", function()
		self:checkForUpdates()
	end)
	self.disconnectPrereleasesCheckChanged = Settings:onChanged("checkForPrereleases", function()
		self:checkForUpdates()
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

	if RunService:IsEdit() then
		self:checkForUpdates()

		self:startSyncReminderPolling()
		self.disconnectSyncReminderPollingChanged = Settings:onChanged("syncReminderPolling", function(enabled)
			if enabled then
				self:startSyncReminderPolling()
			else
				self:stopSyncReminderPolling()
			end
		end)

		self:tryAutoReconnect():andThen(function(didReconnect)
			if not didReconnect then
				self:checkSyncReminder()
			end
		end)
	end

	if self:isAutoConnectPlaytestServerAvailable() then
		self:useRunningConnectionInfo()
		self:startSession()
	end
	self.autoConnectPlaytestServerListener = Settings:onChanged("autoConnectPlaytestServer", function(enabled)
		if enabled then
			if self:isAutoConnectPlaytestServerWriteable() and self.serveSession ~= nil then
				-- Write the existing session
				local baseUrl = self.serveSession.__apiContext.__baseUrl
				self:setRunningConnectionInfo(baseUrl)
			end
		else
			self:clearRunningConnectionInfo()
		end
	end)
end

function App:willUnmount()
	self.waypointConnection:Disconnect()
	self.confirmationBindable:Destroy()

	self.disconnectUpdatesCheckChanged()
	self.disconnectPrereleasesCheckChanged()
	if self.disconnectSyncReminderPollingChanged then
		self.disconnectSyncReminderPollingChanged()
	end

	self:stopSyncReminderPolling()

	self.autoConnectPlaytestServerListener()
	self:clearRunningConnectionInfo()
end

function App:addNotification(notif: {
	text: string,
	isFullscreen: boolean?,
	timeout: number?,
	actions: { [string]: { text: string, style: string, layoutOrder: number, onClick: (any) -> ()? } }?,
	onClose: (any) -> ()?,
})
	if not Settings:get("showNotifications") then
		return
	end

	self.notifId += 1
	local id = self.notifId

	self:setState(function(prevState)
		local notifications = table.clone(prevState.notifications)
		notifications[id] = Dictionary.merge({
			timeout = notif.timeout or 5,
			isFullscreen = notif.isFullscreen or false,
		}, notif)

		return {
			notifications = notifications,
		}
	end)

	return function()
		self:closeNotification(id)
	end
end

function App:closeNotification(id: number)
	if not self.state.notifications[id] then
		return
	end

	self:setState(function(prevState)
		local notifications = table.clone(prevState.notifications)
		notifications[id] = nil

		return {
			notifications = notifications,
		}
	end)
end

function App:checkForUpdates()
	local updateMessage = Version.getUpdateMessage()

	if updateMessage then
		self:addNotification({
			text = updateMessage,
			timeout = 500,
			actions = {
				Dismiss = {
					text = "Dismiss",
					style = "Bordered",
					layoutOrder = 2,
				},
			},
		})
	end
end

function App:getPriorSyncInfo(): { host: string?, port: string?, projectName: string?, timestamp: number? }
	local priorSyncInfos = Settings:get("priorEndpoints")
	if not priorSyncInfos then
		return {}
	end

	local id = tostring(game.PlaceId)
	if ignorePlaceIds[id] then
		return {}
	end

	return priorSyncInfos[id] or {}
end

function App:setPriorSyncInfo(host: string, port: string, projectName: string)
	local priorSyncInfos = Settings:get("priorEndpoints")
	if not priorSyncInfos then
		priorSyncInfos = {}
	end

	local now = os.time()

	-- Clear any stale saves to avoid disc bloat
	for placeId, syncInfo in priorSyncInfos do
		if now - (syncInfo.timestamp or now) > 12_960_000 then
			priorSyncInfos[placeId] = nil
			Log.trace("Cleared stale saved endpoint for {}", placeId)
		end
	end

	local id = tostring(game.PlaceId)
	if ignorePlaceIds[id] then
		return
	end

	priorSyncInfos[id] = {
		host = if host ~= Config.defaultHost then host else nil,
		port = if port ~= Config.defaultPort then port else nil,
		projectName = projectName,
		timestamp = now,
	}
	Log.trace("Saved last used endpoint for {}", game.PlaceId)

	Settings:set("priorEndpoints", priorSyncInfos)
end

function App:getHostAndPort()
	local host = self.host:getValue()
	local port = self.port:getValue()

	return if #host > 0 then host else Config.defaultHost, if #port > 0 then port else Config.defaultPort
end

function App:isSyncLockAvailable()
	if #Players:GetPlayers() == 0 then
		-- Team Create is not active, so no one can be holding the lock
		return true
	end

	local lock = ServerStorage:FindFirstChild("__Rojo_SessionLock")
	if not lock then
		-- No lock is made yet, so it is available
		return true
	end

	if lock.Value and lock.Value ~= Players.LocalPlayer and lock.Value.Parent then
		-- Someone else is holding the lock
		return false, lock.Value
	end

	-- The lock exists, but is not claimed
	return true
end

function App:claimSyncLock()
	if #Players:GetPlayers() == 0 then
		Log.trace("Skipping sync lock because this isn't in Team Create")
		return true
	end

	local isAvailable, priorOwner = self:isSyncLockAvailable()
	if not isAvailable then
		Log.trace("Skipping sync lock because it is already claimed")
		return false, priorOwner
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

function App:findActiveServer()
	local host, port = self:getHostAndPort()
	local baseUrl = if string.find(host, "^https?://")
		then string.format("%s:%s", host, port)
		else string.format("http://%s:%s", host, port)

	Log.trace("Checking for active sync server at {}", baseUrl)

	local apiContext = ApiContext.new(baseUrl)
	return apiContext:connect():andThen(function(serverInfo)
		apiContext:disconnect()
		return serverInfo, host, port
	end)
end

function App:tryAutoReconnect()
	if not Settings:get("autoReconnect") then
		return Promise.resolve(false)
	end

	local priorSyncInfo = self:getPriorSyncInfo()
	if not priorSyncInfo.projectName then
		Log.trace("No prior sync info found, skipping auto-reconnect")
		return Promise.resolve(false)
	end

	return self:findActiveServer()
		:andThen(function(serverInfo)
			-- change
			if serverInfo.projectName == priorSyncInfo.projectName then
				Log.trace("Auto-reconnect found matching server, reconnecting...")
				self:addNotification({
					text = `Auto-reconnect discovered project '{serverInfo.projectName}'...`,
				})
				self:startSession()
				return true
			end
			Log.trace("Auto-reconnect found different server, not reconnecting")
			return false
		end)
		:catch(function()
			Log.trace("Auto-reconnect did not find a server, not reconnecting")
			return false
		end)
end

function App:checkSyncReminder()
	local syncReminderMode = Settings:get("syncReminderMode")
	if syncReminderMode == "None" then
		return
	end

	if self.serveSession ~= nil or not self:isSyncLockAvailable() then
		-- Already syncing or cannot sync, no reason to remind
		return
	end

	local priorSyncInfo = self:getPriorSyncInfo()

	self:findActiveServer()
		:andThen(function(serverInfo, host, port)
			self:sendSyncReminder(
				`Project '{serverInfo.projectName}' is serving at {host}:{port}.\nWould you like to connect?`
			)
		end)
		:catch(function()
			if priorSyncInfo.timestamp and priorSyncInfo.projectName then
				-- We didn't find an active server,
				-- but this place has a prior sync
				-- so we should remind the user to serve

				local timeSinceSync = timeUtil.elapsedToText(os.time() - priorSyncInfo.timestamp)
				self:sendSyncReminder(
					`You synced project '{priorSyncInfo.projectName}' to this place {timeSinceSync}.\nDid you mean to run 'rojo serve' and then connect?`
				)
			end
		end)
end

function App:startSyncReminderPolling()
	if
		self.syncReminderPollingThread ~= nil
		or Settings:get("syncReminderMode") == "None"
		or not Settings:get("syncReminderPolling")
	then
		return
	end

	Log.trace("Starting sync reminder polling thread")
	self.syncReminderPollingThread = task.spawn(function()
		while task.wait(30) do
			if self.syncReminderPollingThread == nil then
				-- The polling thread was stopped, so exit
				return
			end
			if self.dismissSyncReminder then
				-- There is already a sync reminder being shown
				task.wait(5)
				continue
			end
			self:checkSyncReminder()
		end
	end)
end

function App:stopSyncReminderPolling()
	if self.syncReminderPollingThread then
		Log.trace("Stopping sync reminder polling thread")
		task.cancel(self.syncReminderPollingThread)
		self.syncReminderPollingThread = nil
	end
end

function App:sendSyncReminder(message: string)
	local syncReminderMode = Settings:get("syncReminderMode")
	if syncReminderMode == "None" then
		return
	end

	self.dismissSyncReminder = self:addNotification({
		text = message,
		timeout = 120,
		isFullscreen = Settings:get("syncReminderMode") == "Fullscreen",
		onClose = function()
			self.dismissSyncReminder = nil
		end,
		actions = {
			Connect = {
				text = "Connect",
				style = "Solid",
				layoutOrder = 1,
				onClick = function()
					self:startSession()
				end,
			},
			Dismiss = {
				text = "Dismiss",
				style = "Bordered",
				layoutOrder = 2,
				onClick = function()
					-- If the user dismisses the reminder,
					-- then we don't need to remind them again
					self:stopSyncReminderPolling()
				end,
			},
		},
	})
end

function App:isAutoConnectPlaytestServerAvailable()
	return RunService:IsRunning()
		and RunService:IsStudio()
		and RunService:IsServer()
		and Settings:get("autoConnectPlaytestServer")
		and workspace:GetAttribute("__Rojo_ConnectionUrl")
end

function App:isAutoConnectPlaytestServerWriteable()
	return RunService:IsEdit() and Settings:get("autoConnectPlaytestServer")
end

function App:setRunningConnectionInfo(baseUrl: string)
	if not self:isAutoConnectPlaytestServerWriteable() then
		return
	end

	Log.trace("Setting connection info for play solo auto-connect")
	workspace:SetAttribute("__Rojo_ConnectionUrl", baseUrl)
end

function App:clearRunningConnectionInfo()
	if not RunService:IsEdit() then
		-- Only write connection info from edit mode
		return
	end

	Log.trace("Clearing connection info for play solo auto-connect")
	workspace:SetAttribute("__Rojo_ConnectionUrl", nil)
end

function App:useRunningConnectionInfo()
	local connectionInfo = workspace:GetAttribute("__Rojo_ConnectionUrl")
	if not connectionInfo then
		return
	end

	Log.trace("Using connection info for play solo auto-connect")
	local host, port = string.match(connectionInfo, "^(.+):(.-)$")

	self.setHost(host)
	self.setPort(port)
end

function App:startSession()
	local claimedLock, priorOwner = self:claimSyncLock()
	if not claimedLock then
		local msg = string.format("Could not sync because user '%s' is already syncing", tostring(priorOwner))

		Log.warn(msg)
		self:addNotification({
			text = msg,
			timeout = 10,
		})
		self:setState({
			appStatus = AppStatus.Error,
			errorMessage = msg,
			toolbarIcon = Assets.Images.PluginButtonWarning,
		})

		return
	end

	local host, port = self:getHostAndPort()

	local baseUrl = if string.find(host, "^https?://")
		then string.format("%s:%s", host, port)
		else string.format("http://%s:%s", host, port)
	local apiContext = ApiContext.new(baseUrl)

	local serveSession = ServeSession.new({
		apiContext = apiContext,
		twoWaySync = Settings:get("twoWaySync"),
	})

	self.cleanupPrecommit = serveSession:hookPrecommit(function(patch, instanceMap)
		-- Build new tree for patch
		self:setState({
			patchTree = PatchTree.build(patch, instanceMap, { "Property", "Old", "New" }),
		})
	end)
	self.cleanupPostcommit = serveSession:hookPostcommit(function(patch, instanceMap, unappliedPatch)
		local now = DateTime.now().UnixTimestamp
		self:setState(function(prevState)
			local oldPatchData = prevState.patchData
			local newPatchData = {
				patch = patch,
				unapplied = unappliedPatch,
				timestamp = now,
			}

			if PatchSet.isEmpty(patch) then
				-- Keep existing patch info, but use new timestamp
				newPatchData.patch = oldPatchData.patch
				newPatchData.unapplied = oldPatchData.unapplied
			elseif now - oldPatchData.timestamp < 2 then
				-- Patches that apply in the same second are combined for human clarity
				newPatchData.patch = PatchSet.assign(PatchSet.newEmpty(), oldPatchData.patch, patch)
				newPatchData.unapplied = PatchSet.assign(PatchSet.newEmpty(), oldPatchData.unapplied, unappliedPatch)
			end

			return {
				patchTree = PatchTree.updateMetadata(prevState.patchTree, patch, instanceMap, unappliedPatch),
				patchData = newPatchData,
			}
		end)
	end)

	serveSession:onStatusChanged(function(status, details)
		if status == ServeSession.Status.Connecting then
			if self.dismissSyncReminder then
				self.dismissSyncReminder()
				self.dismissSyncReminder = nil
			end

			self:setState({
				appStatus = AppStatus.Connecting,
				toolbarIcon = Assets.Images.PluginButton,
			})
			self:addNotification({
				text = "Connecting to session...",
			})
		elseif status == ServeSession.Status.Connected then
			self.knownProjects[details] = true
			self:setPriorSyncInfo(host, port, details)
			self:setRunningConnectionInfo(baseUrl)

			local address = ("%s:%s"):format(host, port)
			self:setState({
				appStatus = AppStatus.Connected,
				projectName = details,
				address = address,
				toolbarIcon = Assets.Images.PluginButtonConnected,
			})
			self:addNotification({
				text = string.format("Connected to session '%s' at %s.", details, address),
			})
		elseif status == ServeSession.Status.Disconnected then
			self.serveSession = nil
			self:releaseSyncLock()
			self:clearRunningConnectionInfo()
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
				self:addNotification({
					text = tostring(details),
					timeout = 10,
				})
			else
				self:setState({
					appStatus = AppStatus.NotConnected,
					toolbarIcon = Assets.Images.PluginButton,
				})
				self:addNotification({
					text = "Disconnected from session.",
					timeout = 10,
				})
			end
		end
	end)

	serveSession:setConfirmCallback(function(instanceMap, patch, serverInfo)
		if PatchSet.isEmpty(patch) then
			Log.trace("Accepting patch without confirmation because it is empty")
			return "Accept"
		end

		-- Play solo auto-connect does not require confirmation
		if self:isAutoConnectPlaytestServerAvailable() then
			Log.trace("Accepting patch without confirmation because play solo auto-connect is enabled")
			return "Accept"
		end

		local confirmationBehavior = Settings:get("confirmationBehavior")
		if confirmationBehavior == "Initial" then
			-- Only confirm if we haven't synced this project yet this session
			if self.knownProjects[serverInfo.projectName] then
				Log.trace(
					"Accepting patch without confirmation because project has already been connected and behavior is set to Initial"
				)
				return "Accept"
			end
		elseif confirmationBehavior == "Large Changes" then
			-- Only confirm if the patch impacts many instances
			if PatchSet.countInstances(patch) < Settings:get("largeChangesConfirmationThreshold") then
				Log.trace(
					"Accepting patch without confirmation because patch is small and behavior is set to Large Changes"
				)
				return "Accept"
			end
		elseif confirmationBehavior == "Unlisted PlaceId" then
			-- Only confirm if the current placeId is not in the servePlaceIds allowlist
			if serverInfo.expectedPlaceIds then
				local isListed = table.find(serverInfo.expectedPlaceIds, game.PlaceId) ~= nil
				if isListed then
					Log.trace(
						"Accepting patch without confirmation because placeId is listed and behavior is set to Unlisted PlaceId"
					)
					return "Accept"
				end
			end
		elseif confirmationBehavior == "Never" then
			Log.trace("Accepting patch without confirmation because behavior is set to Never")
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

		self:addNotification({
			text = string.format(
				"Please accept%sor abort the initializing sync session.",
				Settings:get("twoWaySync") and ", reject, " or " "
			),
			timeout = 7,
		})

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

	if self.cleanupPrecommit ~= nil then
		self.cleanupPrecommit()
	end
	if self.cleanupPostcommit ~= nil then
		self.cleanupPostcommit()
	end

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
			tooltip = e(Tooltip.Provider, nil, {
				gui = e(StudioPluginGui, {
					id = pluginName,
					title = pluginName,
					active = self.state.guiEnabled,
					isEphemeral = false,

					initDockState = Enum.InitialDockState.Right,
					overridePreviousState = false,
					floatingSize = Vector2.new(320, 210),
					minimumSize = Vector2.new(300, 210),

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
						patchTree = self.state.patchTree,
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
						syncActive = self.serveSession ~= nil
							and self.serveSession:getStatus() == ServeSession.Status.Connected,

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
					Notifications = e(Notifications, {
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
