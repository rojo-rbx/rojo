if not plugin then
	return
end

local Plugin = require(script.Parent.Plugin)
local Config = require(script.Parent.Config)
local Version = require(script.Parent.Version)

--[[
	Check if the user is using a newer version of Rojo than last time. If they
	are, show them a reminder to make sure they check their server version.
]]
local function checkUpgrade()
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

	-- When developing Rojo, there's no use in storing that version number.
	if not Config.dev then
		plugin:SetSetting("LastRojoVersion", Config.version)
	end
end

local function main()
	local pluginInstance = Plugin.new()

	local displayedVersion = Config.dev and "DEV" or Version.display(Config.version)

	local toolbar = plugin:CreateToolbar("Rojo Plugin " .. displayedVersion)

	toolbar:CreateButton("Test Connection", "Connect to Rojo Server", "")
		.Click:Connect(function()
			checkUpgrade()

			pluginInstance:connect()
				:catch(function(err)
					warn(err)
				end)
		end)

	toolbar:CreateButton("Sync In", "Sync into Roblox Studio", "")
		.Click:Connect(function()
			checkUpgrade()

			pluginInstance:syncIn()
				:catch(function(err)
					warn(err)
				end)
		end)

	toolbar:CreateButton("Toggle Polling", "Poll server for changes", "")
		.Click:Connect(function()
			checkUpgrade()

			pluginInstance:togglePolling()
				:catch(function(err)
					warn(err)
				end)
		end)
end

main()
