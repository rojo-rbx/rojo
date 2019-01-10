local Roact = require(script:FindFirstAncestor("Rojo").Roact)

local Session = require(script.Parent.Parent.Session)
local Config = require(script.Parent.Parent.Config)
local Version = require(script.Parent.Parent.Version)
local Logging = require(script.Parent.Parent.Logging)
local DevSettings = require(script.Parent.Parent.DevSettings)

local ConnectPanel = require(script.Parent.ConnectPanel)
local ConnectionActivePanel = require(script.Parent.ConnectionActivePanel)

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
	Configuring = "Configuring",
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
	elseif self.state.sessionStatus == SessionStatus.Configuring then
		children = {
			ConnectPanel = e(ConnectPanel, {
				startSession = function(address, port)
					Logging.trace("Starting new session")

					self.currentSession = Session.new({
						address = address,
						port = port,
						onError = function()
							Logging.trace("Session terminated")
							self.currentSession = nil

							self:setState({
								sessionStatus = SessionStatus.Disconnected,
							})
						end
					})

					self:setState({
						sessionStatus = SessionStatus.Connected,
					})
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

	return e("ScreenGui", nil, children)
end

function App:didMount()
	Logging.trace("Rojo %s initializing", self.displayedVersion)

	local toolbar = self.props.plugin:CreateToolbar("Rojo " .. self.displayedVersion)

	-- TODO: Icon!
	local connectButton = toolbar:CreateButton("Connect", "Connect to Rojo Session", "")
	connectButton.ClickableWhenViewportHidden = true

	connectButton.Click:Connect(function()
		connectButton:SetActive(false)
		checkUpgrade(self.props.plugin)

		if self.state.sessionStatus == SessionStatus.Connected then
			Logging.trace("Disconnecting session")

			error("NYI")
		elseif self.state.sessionStatus == SessionStatus.Disconnected then
			Logging.trace("Starting session configuration")

			self:setState({
				sessionStatus = SessionStatus.Configuring,
			})
		elseif self.state.sessionStatus == SessionStatus.Configuring then
			Logging.trace("Canceling session configuration")

			self:setState({
				sessionStatus = SessionStatus.Disconnected,
			})
		end
	end)
end

return App