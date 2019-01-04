if not plugin then
	return
end

local Roact = require(script.Parent.Roact)

Roact.setGlobalConfig({
	elementTracing = true,
})

local Session = require(script.Session)
local Config = require(script.Config)
local Version = require(script.Version)
local Logging = require(script.Logging)
local DevSettings = require(script.DevSettings)
local App = require(script.Components.App)

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
local function checkUpgrade()
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

local function main()
	local displayedVersion = DevSettings:isEnabled()
		and Config.codename
		or Version.display(Config.version)

	Logging.trace("Rojo %s initialized", displayedVersion)

	local toolbar = plugin:CreateToolbar("Rojo " .. displayedVersion)

	Roact.mount(Roact.createElement(App), game:GetService("CoreGui"), "Rojo UI")

	local currentSession

	-- TODO: Icon!
	local connectButton = toolbar:CreateButton("Connect", "Connect to Rojo Session", "")
	connectButton.ClickableWhenViewportHidden = true

	connectButton.Click:Connect(function()
		connectButton:SetActive(false)
		checkUpgrade()

		if currentSession ~= nil then
			Logging.warn("A session is already running!")
			return
		end

		Logging.info("Started new session.")
		currentSession = Session.new(function()
			Logging.info("Session terminated.")
			currentSession = nil
		end)
	end)
end

main()