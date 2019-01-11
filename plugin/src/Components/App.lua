local Roact = require(script:FindFirstAncestor("Rojo").Roact)

local Plugin = script:FindFirstAncestor("Plugin")

local Assets = require(Plugin.Assets)
local Session = require(Plugin.Session)
local Config = require(Plugin.Config)
local Version = require(Plugin.Version)
local Logging = require(Plugin.Logging)
local DevSettings = require(Plugin.DevSettings)

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

	Logging.info(message)
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
	ConfiguringSession = "ConfiguringSession",
	-- TODO: Error?
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

	self.connectButton = nil
	self.configButton = nil
	self.currentSession = nil

	self.displayedVersion = DevSettings:isEnabled()
		and Config.codename
		or Version.display(Config.version)
end

function App:render()
	local children

	if self.state.sessionStatus == SessionStatus.Connected then
		children = {
			ConnectionActivePanel = e(ConnectionActivePanel),
		}
	elseif self.state.sessionStatus == SessionStatus.ConfiguringSession then
		children = {
			ConnectPanel = e(ConnectPanel, {
				startSession = function(address, port)
					Logging.trace("Starting new session")

					local success, session = Session.new({
						address = address,
						port = port,
						onError = function(message)
							Logging.warn("%s", tostring(message))
							Logging.trace("Session terminated due to error")
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
					Logging.trace("Canceling session configuration")

					self:setState({
						sessionStatus = SessionStatus.Disconnected,
					})
				end,
			}),
		}
	end

	return e("ScreenGui", {
		AutoLocalize = false,
		ZIndexBehavior = Enum.ZIndexBehavior.Sibling,
	}, children)
end

function App:didMount()
	Logging.trace("Rojo %s initializing", self.displayedVersion)

	local toolbar = self.props.plugin:CreateToolbar("Rojo " .. self.displayedVersion)

	self.connectButton = toolbar:CreateButton(
		"Connect",
		"Connect to a running Rojo session",
		Assets.StartSession)
	self.connectButton.ClickableWhenViewportHidden = false
	self.connectButton.Click:Connect(function()
		checkUpgrade(self.props.plugin)

		if self.state.sessionStatus == SessionStatus.Connected then
			Logging.trace("Disconnecting session")

			self.currentSession:disconnect()
			self.currentSession = nil
			self:setState({
				sessionStatus = SessionStatus.Disconnected,
			})

			Logging.trace("Session terminated by user")
		elseif self.state.sessionStatus == SessionStatus.Disconnected then
			Logging.trace("Starting session configuration")

			self:setState({
				sessionStatus = SessionStatus.ConfiguringSession,
			})
		elseif self.state.sessionStatus == SessionStatus.ConfiguringSession then
			Logging.trace("Canceling session configuration")

			self:setState({
				sessionStatus = SessionStatus.Disconnected,
			})
		end
	end)

	self.configButton = toolbar:CreateButton(
		"Configure",
		"Configure the Rojo plugin",
		Assets.Configure)
	self.configButton.ClickableWhenViewportHidden = false
	self.configButton.Click:Connect(function()
		self.configButton:SetActive(false)
	end)
end

function App:didUpdate()
	local connectActive = self.state.sessionStatus == SessionStatus.ConfiguringSession
		or self.state.sessionStatus == SessionStatus.Connected

	self.connectButton:SetActive(connectActive)

	if self.state.sessionStatus == SessionStatus.Connected then
		self.connectButton.Icon = Assets.SessionActive
	else
		self.connectButton.Icon = Assets.StartSession
	end
end

return App