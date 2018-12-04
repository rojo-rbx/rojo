if not plugin then
	return
end

local Session = require(script.Parent.Session)
local Config = require(script.Parent.Config)
local Version = require(script.Parent.Version)
local Logging = require(script.Parent.Logging)

--[[
	Check if the user is using a newer version of Rojo than last time. If they
	are, show them a reminder to make sure they check their server version.
]]
local function checkUpgrade()
	-- When developing Rojo, there's no use in doing version checks
	if Config.dev then
		return
	end

	local lastVersion = plugin:GetSetting("LastRojoVersion")

	if lastVersion then
		local wasUpgraded = Version.compare(Config.version, lastVersion) == 1

		if wasUpgraded then
			local message = (
				"\nRojo detected an upgrade from version %s to version %s." ..
				"\nMake sure you have also upgraded your server!" ..
				"\n\nRojo version %s is intended for use with server version %s.\n"
			):format(
				Version.display(lastVersion), Version.display(Config.version),
				Version.display(Config.version), Config.expectedServerVersionString
			)

			print(message)
		end
	end

	plugin:SetSetting("LastRojoVersion", Config.version)
end

local function main()
	local displayedVersion = Config.dev and "DEV" or Version.display(Config.version)

	Logging.trace("Rojo %s initialized", displayedVersion)

	local toolbar = plugin:CreateToolbar("Rojo " .. displayedVersion)

	local currentSession

	-- TODO: More robust session tracking to handle errors
	-- TODO: Icon!
	toolbar:CreateButton("Connect", "Connect to Rojo Session", "")
		.Click:Connect(function()
			checkUpgrade()

			if currentSession ~= nil then
				Logging.warn("A session is already running!")
				return
			end

			Logging.info("Started new session.")
			currentSession = Session.new()
		end)
end

main()
