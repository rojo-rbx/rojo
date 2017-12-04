if not plugin then
	return
end

local Plugin = require(script.Parent.Plugin)
local Config = require(script.Parent.Config)

local function main()
	local pluginInstance = Plugin.new()

	local displayedVersion = Config.dev and "DEV" or Config.version

	local toolbar = plugin:CreateToolbar("Rojo Plugin " .. displayedVersion)

	toolbar:CreateButton("Test Connection", "Connect to Rojo Server", "")
		.Click:Connect(function()
			pluginInstance:connect()
				:catch(function(err)
					warn(err)
				end)
		end)

	toolbar:CreateButton("Sync In", "Sync into Roblox Studio", "")
		.Click:Connect(function()
			pluginInstance:syncIn()
				:catch(function(err)
					warn(err)
				end)
		end)

	toolbar:CreateButton("Toggle Polling", "Poll server for changes", "")
		.Click:Connect(function()
			spawn(function()
				pluginInstance:togglePolling()
					:catch(function(err)
						warn(err)
					end)
			end)
		end)
end

main()
