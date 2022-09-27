local Parent = script:FindFirstAncestor("Rojo")
local Plugin = Parent.Plugin

local Config = require(Plugin.Config)
local Settings = require(Plugin.Settings)
local ApiContext = require(Plugin.ApiContext)

local API  = {}

function API.new(app)
	local Rojo = {}

	Rojo.Connected = if app.serveSession then app.serveSession:getStatus() == "Connected" else false
	Rojo.Address = nil
	Rojo.ProjectName = nil
	Rojo.Version = table.clone(Config.version)
	Rojo.ProtocolVersion = Config.protocolVersion

	Rojo._notifRateLimit = {}

	function Rojo:Test(...)
		print("Rojo:Test called by", Rojo:_getCaller(), "with args", ...)
	end

	function Rojo:_getCallerFull()
		local traceback = string.split(debug.traceback(), "\n")
		local topLevel = traceback[#traceback - 1]

		local debugPlugin = string.match(topLevel, "^(PluginDebugService%.user_.-)%.")
		if debugPlugin then
			return debugPlugin
		end

		local localPlugin = string.match(topLevel, "^(user_.-)%.")
		if localPlugin then
			return localPlugin
		end

		local cloudPlugin = string.match(topLevel, "(cloud_%d-%..-)%.")
		if cloudPlugin then
			return cloudPlugin
		end

		return "RobloxStudio.CommandBar"
	end

	function Rojo:_getCaller()
		local traceback = string.split(debug.traceback(), "\n")
		local topLevel = traceback[#traceback - 1]

		local debugPlugin = string.match(topLevel, "^PluginDebugService%.user_(.-)%.")
		if debugPlugin then
			return debugPlugin
		end

		local localPlugin = string.match(topLevel, "^user_(.-)%.")
		if localPlugin then
			return localPlugin
		end

		local cloudPlugin = string.match(topLevel, "cloud_%d-%.(.-)%.")
		if cloudPlugin then
			return cloudPlugin
		end

		return "CommandBar"
	end

	function Rojo:_getCallerType()
		local traceback = string.split(debug.traceback(), "\n")
		local topLevel = traceback[#traceback - 1]

		if string.find(topLevel, "^PluginDebugService%.user_") then
			return "Debug"
		end

		if string.find(topLevel, "^user_") then
			return "Local"
		end

		if string.find(topLevel, "cloud_%d+%.") then
			return "Cloud"
		end

		return "CommandBar"
	end

	function Rojo:ConnectAsync(host: string?, port: number?)
		app:startSession(host, port)
	end

	function Rojo:DisconnectAsync()
		app:endSession()
	end

	function Rojo:GetSetting(setting: string): any
		return Settings:get(setting)
	end

	function Rojo:SetSetting(setting: string, value: any)
		return Settings:set(setting, value)
	end

	function Rojo:Notify(msg: string, timeout: number?)
		local source = Rojo:_getCaller()

		if Rojo._notifRateLimit[source] == nil then
			Rojo._notifRateLimit[source] = 0
		elseif Rojo._notifRateLimit[source] > 45 then
			return -- Rate limited
		end

		Rojo._notifRateLimit[source] += 1
		task.delay(30, function()
			Rojo._notifRateLimit[source] -= 1
		end)

		app:addThirdPartyNotification(source, msg, timeout)
		return
	end

	function Rojo:GetHostAndPort(): (string, string)
		return app:getHostAndPort()
	end

	function Rojo:CreateApiContext(baseUrl: string)
		return ApiContext.new(baseUrl)
	end

	local ReadOnly = setmetatable({}, {
		__index = function(_, key)
			if string.find(key, "^_") then
				return nil -- Don't expose private members
			end
			return Rojo[key]
		end,
		__newindex = function(_, key, value)
			error(string.format("Attempted to set Rojo.%s to %q but it's a read-only value", key, value), 2)
			return
		end,
	})

	return Rojo, ReadOnly
end

return API
