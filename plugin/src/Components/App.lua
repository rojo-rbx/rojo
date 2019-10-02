local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)
local Log = require(Rojo.Log)

local Assets = require(Plugin.Assets)
local Config = require(Plugin.Config)
local DevSettings = require(Plugin.DevSettings)
local Session = require(Plugin.Session)
local Version = require(Plugin.Version)
local preloadAssets = require(Plugin.preloadAssets)

local ConnectPanel = require(Plugin.Components.ConnectPanel)
local ConnectionActivePanel = require(Plugin.Components.ConnectionActivePanel)

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

local SessionStatus = {
	Disconnected = "Disconnected",
	Connected = "Connected",
}

setmetatable(SessionStatus, {
	__index = function(_, key)
		error(("%q is not a valid member of SessionStatus"):format(tostring(key)), 2)
	end,
})

local App = Roact.Component:extend("App")

function App:init()
	self:setState({
		sessionStatus = SessionStatus.Disconnected,
	})

	self.signals = {}
	self.currentSession = nil

	self.displayedVersion = DevSettings:isEnabled()
		and Config.codename
		or Version.display(Config.version)

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

	self.dockWidget = self.props.plugin:CreateDockWidgetPluginGui("Rojo-0.5.x", widgetInfo)
	self.dockWidget.Name = "Rojo " .. self.displayedVersion
	self.dockWidget.Title = "Rojo " .. self.displayedVersion
	self.dockWidget.AutoLocalize = false
	self.dockWidget.ZIndexBehavior = Enum.ZIndexBehavior.Sibling

	self.signals.dockWidgetEnabled = self.dockWidget:GetPropertyChangedSignal("Enabled"):Connect(function()
		self.toggleButton:SetActive(self.dockWidget.Enabled)
	end)
end

function App:render()
	local children

	if self.state.sessionStatus == SessionStatus.Connected then
		children = {
			ConnectionActivePanel = e(ConnectionActivePanel, {
				stopSession = function()
					Log.trace("Disconnecting session")

					self.currentSession:disconnect()
					self.currentSession = nil
					self:setState({
						sessionStatus = SessionStatus.Disconnected,
					})

					Log.trace("Session terminated by user")
				end,
			}),
		}
	elseif self.state.sessionStatus == SessionStatus.Disconnected then
		children = {
			ConnectPanel = e(ConnectPanel, {
				startSession = function(address, port)
					Log.trace("Starting new session")

					local success, session = Session.new({
						address = address,
						port = port,
						onError = function(message)
							Log.warn("Rojo session terminated because of an error:\n%s", tostring(message))
							self.currentSession = nil

							self:setState({
								sessionStatus = SessionStatus.Disconnected,
							})
						end
					})

					if success then
						self.currentSession = session
						self:setState({
							sessionStatus = SessionStatus.Connected,
						})
					end
				end,
				cancel = function()
					Log.trace("Canceling session configuration")

					self:setState({
						sessionStatus = SessionStatus.Disconnected,
					})
				end,
			}),
		}
	end

	return Roact.createElement(Roact.Portal, {
		target = self.dockWidget,
	}, children)
end

function App:didMount()
	Log.trace("Rojo %s initializing", self.displayedVersion)

	checkUpgrade(self.props.plugin)
	preloadAssets()
end

function App:willUnmount()
	if self.currentSession ~= nil then
		self.currentSession:disconnect()
		self.currentSession = nil
	end

	for _, signal in pairs(self.signals) do
		signal:Disconnect()
	end
end

return App