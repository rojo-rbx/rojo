if not plugin then
	return
end

local Session = require(script.Parent.Session)
local Config = require(script.Parent.Config)
local Version = require(script.Parent.Version)

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

	local toolbar = plugin:CreateToolbar("Rojo " .. displayedVersion)

	toolbar:CreateButton("Connect", "Connect to Rojo Session", "")
		.Click:Connect(function()
			checkUpgrade()
			Session.new()

			error("NYI")
		end)
end

main()
