local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)
local Log = require(Rojo.Log)

local ApiContext = require(Plugin.ApiContext)
local Assets = require(Plugin.Assets)
local Config = require(Plugin.Config)
local DevSettings = require(Plugin.DevSettings)
local ServeSession = require(Plugin.ServeSession)
local Version = require(Plugin.Version)
local preloadAssets = require(Plugin.preloadAssets)
local strict = require(Plugin.strict)

local ConnectPanel = require(Plugin.Components.ConnectPanel)
local ConnectingPanel = require(Plugin.Components.ConnectingPanel)
local ConnectionActivePanel = require(Plugin.Components.ConnectionActivePanel)
local ErrorPanel = require(Plugin.Components.ErrorPanel)
local SettingsPanel = require(Plugin.Components.SettingsPanel)

local e = Roact.createElement

local function showUpgradeMessage(lastVersion)
	local message = (
		"Rojo detected an upgrade from version %s to version %s." ..
		"\nMake sure you have also upgraded your server!" ..
		"\n\nRojo plugin version %s is intended for use with server version %s."
	):format(
		Version.display(lastVersion), Version.display(Config.version),
		Version.display(Config.version), Config.expectedServerVersionString
	)

	Log.info(message)
end

--[[
	Check if the user is using a newer version of Rojo than last time. If they
	are, show them a reminder to make sure they check their server version.
]]
local function checkUpgrade(plugin)
	-- When developing Rojo, there's no use in doing version checks
	if DevSettings:isEnabled() then
		return
	end

	local lastVersion = plugin:GetSetting("LastRojoVersion")

	if lastVersion then
		local wasUpgraded = Version.compare(Config.version, lastVersion) == 1

		if wasUpgraded then
			showUpgradeMessage(lastVersion)
		end
	end

	plugin:SetSetting("LastRojoVersion", Config.version)
end

local AppStatus = strict("AppStatus", {
	NotStarted = "NotStarted",
	Connecting = "Connecting",
	Connected = "Connected",
	Error = "Error",
	Settings = "Settings",
})

local App = Roact.Component:extend("App")

function App:init()
	self:setState({
		appStatus = AppStatus.NotStarted,
		errorMessage = nil,
	})

	self.signals = {}
	self.serveSession = nil
	self.displayedVersion = Version.display(Config.version)

	local toolbar = self.props.plugin:CreateToolbar("Rojo " .. self.displayedVersion)

	self.toggleButton = toolbar:CreateButton(
		"Rojo",
		"Show or hide the Rojo panel",
		Assets.Images.Icon)
	self.toggleButton.ClickableWhenViewportHidden = true
	self.toggleButton.Click:Connect(function()
		self.dockWidget.Enabled = not self.dockWidget.Enabled
	end)

	local widgetInfo = DockWidgetPluginGuiInfo.new(
		Enum.InitialDockState.Right,
		false, -- Initially enabled state
		false, -- Whether to override the widget's previous state
		360, 190, -- Floating size
		360, 190 -- Minimum size
	)

	self.dockWidget = self.props.plugin:CreateDockWidgetPluginGui("Rojo-" .. self.displayedVersion, widgetInfo)
	self.dockWidget.Name = "Rojo " .. self.displayedVersion
	self.dockWidget.Title = "Rojo " .. self.displayedVersion
	self.dockWidget.AutoLocalize = false
	self.dockWidget.ZIndexBehavior = Enum.ZIndexBehavior.Sibling

	self.signals.dockWidgetEnabled = self.dockWidget:GetPropertyChangedSignal("Enabled"):Connect(function()
		self.toggleButton:SetActive(self.dockWidget.Enabled)
	end)
end

function App:startSession(address, port, sessionOptions)
	Log.trace("Starting new session")

	local baseUrl = ("http://%s:%s"):format(address, port)
	self.serveSession = ServeSession.new({
		apiContext = ApiContext.new(baseUrl),
		openScriptsExternally = sessionOptions.openScriptsExternally,
		twoWaySync = sessionOptions.twoWaySync,
	})

	self.serveSession:onStatusChanged(function(status, details)
		if status == ServeSession.Status.Connecting then
			self:setState({
				appStatus = AppStatus.Connecting,
			})
		elseif status == ServeSession.Status.Connected then
			self:setState({
				appStatus = AppStatus.Connected,
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
					appStatus = AppStatus.NotStarted,
				})
			end
		end
	end)

	self.serveSession:start()
end

function App:render()
	local children

	if self.state.appStatus == AppStatus.NotStarted then
		children = {
			ConnectPanel = e(ConnectPanel, {
				startSession = function(address, port, settings)
					self:startSession(address, port, settings)
				end,
				openSettings = function()
					self:setState({
						appStatus = AppStatus.Settings,
					})
				end,
				cancel = function()
					Log.trace("Canceling session configuration")

					self:setState({
						appStatus = AppStatus.NotStarted,
					})
				end,
			}),
		}
	elseif self.state.appStatus == AppStatus.Connecting then
		children = {
			ConnectingPanel = e(ConnectingPanel),
		}
	elseif self.state.appStatus == AppStatus.Connected then
		children = {
			ConnectionActivePanel = e(ConnectionActivePanel, {
				stopSession = function()
					Log.trace("Disconnecting session")

					self.serveSession:stop()
					self.serveSession = nil
					self:setState({
						appStatus = AppStatus.NotStarted,
					})

					Log.trace("Session terminated by user")
				end,
			}),
		}
	elseif self.state.appStatus == AppStatus.Settings then
		children = {
			e(SettingsPanel, {
				back = function()
					self:setState({
						appStatus = AppStatus.NotStarted,
					})
				end,
			}),
		}
	elseif self.state.appStatus == AppStatus.Error then
		children = {
			ErrorPanel = e(ErrorPanel, {
				errorMessage = self.state.errorMessage,
				onDismiss = function()
					self:setState({
						appStatus = AppStatus.NotStarted,
					})
				end,
			}),
		}
	end

	return e(Roact.Portal, {
		target = self.dockWidget,
	}, children)
end

function App:didMount()
	Log.trace("Rojo {} initializing", self.displayedVersion)

	checkUpgrade(self.props.plugin)
	preloadAssets()
end

function App:willUnmount()
	if self.serveSession ~= nil then
		self.serveSession:stop()
		self.serveSession = nil
	end

	for _, signal in pairs(self.signals) do
		signal:Disconnect()
	end
end

return App