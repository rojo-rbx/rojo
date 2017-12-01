if not plugin then
	return
end

local Plugin = require(script.Parent.Plugin)

local function main()
	local pluginInstance = Plugin.new()

	local toolbar = plugin:CreateToolbar("Rojo Plugin vDEV")

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
