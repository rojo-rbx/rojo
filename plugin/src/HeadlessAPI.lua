local MarketplaceService = game:GetService("MarketplaceService")

local Parent = script:FindFirstAncestor("Rojo")
local Plugin = Parent.Plugin
local Packages = Parent.Packages

local Log = require(Packages.Log)

local Config = require(Plugin.Config)
local Settings = require(Plugin.Settings)
local ApiContext = require(Plugin.ApiContext)

local cloudIdInfoCache = {}
local apiPermissionAllowlist = {
	Version = true,
	ProtocolVersion = true,
	RequestAccess = true,
}

local API  = {}

function API.new(app)
	local Rojo = {}

	Rojo._rateLimit = {}
	Rojo._permissions = Settings:get("apiPermissions") or {}
	Rojo._changedEvent = Instance.new("BindableEvent")
	Rojo._apiDescriptions = {}

	Rojo.Changed = Rojo._changedEvent.Event
	Rojo._apiDescriptions.Changed = "An event that fires when a headless API property changes"

	Rojo.Connected = if app.serveSession then app.serveSession:getStatus() == "Connected" else false
	Rojo._apiDescriptions.Connected = "Whether or not the plugin is connected to a Rojo server"

	Rojo.Address = nil
	Rojo._apiDescriptions.Address = "The address (host:port) that the plugin is connected to"

	Rojo.ProjectName = nil
	Rojo._apiDescriptions.ProjectName = "The name of the project that the plugin is connected to"

	Rojo.Version = table.clone(Config.version)
	Rojo.ProtocolVersion = Config.protocolVersion

	function Rojo:_updateProperty(property: string, value: any?)
		local oldValue = Rojo[property]
		Rojo[property] = value
		Rojo._changedEvent:Fire(property, value, oldValue)
	end

	function Rojo:_getCallerSource()
		local traceback = string.split(debug.traceback(), "\n")
		local topLevel = traceback[#traceback - 1]

		local localPlugin = string.match(topLevel, "(user_.-)%.")
		if localPlugin then
			return localPlugin
		end

		local cloudPlugin = string.match(topLevel, "(cloud_%d-)%.")
		if cloudPlugin then
			return cloudPlugin
		end

		return "RobloxStudio_CommandBar"
	end

	function Rojo:_getCallerName()
		local traceback = string.split(debug.traceback(), "\n")
		local topLevel = traceback[#traceback - 1]

		local localPlugin = string.match(topLevel, "user_(.-)%.")
		if localPlugin then
			return localPlugin
		end

		local cloudId, cloudInstance = string.match(topLevel, "cloud_(%d-)%.(.-)[^%w_%-]")
		if cloudId then
			local info = cloudIdInfoCache[cloudId]
			if info then
				return info.Name .. " by " .. info.Creator.Name
			else
				local success, newInfo = pcall(MarketplaceService.GetProductInfo, MarketplaceService, tonumber(cloudId), Enum.InfoType.Asset)
				if success then
					cloudIdInfoCache[cloudId] = newInfo
					return newInfo.Name .. " by " .. newInfo.Creator.Name
				end
			end

			-- Fallback to the name of the instance uploaded inside this plugin
			-- The reason this is not ideal is because creators often upload a folder named "Main" or something
			return cloudInstance
		end

		return "Command Bar"
	end

	function Rojo:_getMetaFromSource(source)
		local localPlugin = string.match(source, "user_(.+)")
		if localPlugin then
			return {
				Type = "Local",
				Name = localPlugin,
			}
		end

		local cloudId = string.match(source, "cloud_(%d+)")
		if cloudId then
			local info = cloudIdInfoCache[cloudId]
			if info then
				return {
					Type = "Cloud",
					Name = info.Name,
					Creator = info.Creator.Name,
				}
			else
				local success, newInfo = pcall(MarketplaceService.GetProductInfo, MarketplaceService, tonumber(cloudId), Enum.InfoType.Asset)
				if success then
					cloudIdInfoCache[cloudId] = newInfo
					return {
						Type = "Cloud",
						Name = newInfo.Name,
						Creator = newInfo.Creator.Name,
					}
				end
			end
		end

		return {
			Type = "Studio",
			Name = "Command Bar",
		}
	end

	function Rojo:_getCallerType()
		local traceback = string.split(debug.traceback(), "\n")
		local topLevel = traceback[#traceback - 1]

		if string.find(topLevel, "user_") then
			return "Local"
		end

		if string.find(topLevel, "cloud_%d+%.") then
			return "Cloud"
		end

		return "CommandBar"
	end

	function Rojo:_permissionCheck(key: string): boolean
		if apiPermissionAllowlist[key] then return true end

		local source = Rojo:_getCallerSource()
		if Rojo._permissions[source] == nil then
			Rojo._permissions[source] = {}
		end

		return not not Rojo._permissions[source][key]
	end

	local BUCKET, LIMIT = 10, 15
	function Rojo:_checkRateLimit(api: string): boolean
		local source = Rojo:_getCallerSource()

		if Rojo._rateLimit[source] == nil then
			Rojo._rateLimit[source] = {
				[api] = 0,
			}

		elseif Rojo._rateLimit[source][api] == nil then
			Rojo._rateLimit[source][api] = 0

		elseif Rojo._rateLimit[source][api] >= LIMIT then
			-- No more than LIMIT requests per BUCKET seconds
			return true

		end

		Rojo._rateLimit[source][api] += 1
		task.delay(BUCKET, function()
			Rojo._rateLimit[source][api] -= 1
		end)

		return false
	end

	function Rojo:RequestAccess(apis: {string}): {[string]: boolean}
		assert(type(apis) == "table", "Rojo:RequestAccess expects an array of valid API names")

		if Rojo:_checkRateLimit("RequestAccess") then
			-- Because this opens a popup, we dont want to let users get spammed by it
			return {}
		end

		local source, name = Rojo:_getCallerSource(), Rojo:_getCallerName()

		if Rojo._permissions[source] == nil then
			Rojo._permissions[source] = {}
		end

		-- Sanitize request
		local sanitizedApis = {}
		for _, api in apis do
			if Rojo[api] ~= nil then
				table.insert(sanitizedApis, api)
			else
				warn(string.format("Rojo.%s is not a valid API", tostring(api)))
			end
		end
		assert(#sanitizedApis > 0, "Rojo:RequestAccess expects an array of valid API names")

		local alreadyAllowed = true
		for _, api in sanitizedApis do
			if not Rojo._permissions[source][api] then
				alreadyAllowed = false
				break
			end
		end

		if alreadyAllowed then
			local response = {}
			for _, api in sanitizedApis do
				response[api] = true
			end
			return response
		end

		local response = app:requestPermission(source, name, sanitizedApis, Rojo._permissions[source])

		for api, granted in response do
			Log.warn(string.format(
				"%s Rojo.%s for '%s'",
				granted and "Granting permission to" or "Denying permission to", api, name
			))
			Rojo._permissions[source][api] = granted
		end
		Settings:set("apiPermissions", Rojo._permissions)

		return response
	end

	Rojo._apiDescriptions.Test = "Prints the given arguments to the console"
	function Rojo:Test(...)
		local args = table.pack(...)
		for i=1, args.n do
			local v = args[i]
			local t = type(v)
			if t == "string" then
				args[i] = string.format("%q", v)
			else
				args[i] = tostring(v)
			end
		end

		print(string.format(
			"Rojo:Test(%s) called from '%s' (%s)",
			table.concat(args, ", "), Rojo:_getCallerName(), Rojo:_getCallerSource()
		))
	end

	Rojo._apiDescriptions.ConnectAsync = "Connects to a Rojo server"
	function Rojo:ConnectAsync(host: string?, port: string?)
		assert(type(host) == "string" or host == nil, "Host must be type `string?`")
		assert(type(port) == "string" or port == nil, "Port must be type `string?`")

		if Rojo:_checkRateLimit("ConnectAsync") then
			return
		end

		app:startSession(host, port)
	end

	Rojo._apiDescriptions.DisconnectAsync = "Disconnects from the Rojo server"
	function Rojo:DisconnectAsync()
		if Rojo:_checkRateLimit("DisconnectAsync") then
			return
		end

		app:endSession()
	end

	Rojo._apiDescriptions.GetSetting = "Gets a Rojo setting"
	function Rojo:GetSetting(setting: string): any
		assert(type(setting) == "string", "Setting must be type `string`")

		return Settings:get(setting)
	end

	Rojo._apiDescriptions.SetSetting = "Sets a Rojo setting"
	function Rojo:SetSetting(setting: string, value: any)
		assert(type(setting) == "string", "Setting must be type `string`")

		if Rojo:_checkRateLimit("SetSetting") then
			return
		end

		return Settings:set(setting, value)
	end

	Rojo._apiDescriptions.Notify = "Shows a notification in the Rojo UI"
	function Rojo:Notify(msg: string, timeout: number?)
		assert(type(msg) == "string", "Message must be type `string`")
		assert(type(timeout) == "number" or timeout == nil, "Timeout must be type `number?`")

		if Rojo:_checkRateLimit("Notify") then
			return
		end

		app:addThirdPartyNotification(Rojo:_getCallerName(), msg, timeout)
		return
	end

	Rojo._apiDescriptions.GetHostAndPort = "Gets the host and port that Rojo is set to"
	function Rojo:GetHostAndPort(): (string, string)
		return app:getHostAndPort()
	end

	Rojo._apiDescriptions.CreateApiContext = "Creates a new API context"
	function Rojo:CreateApiContext(baseUrl: string)
		assert(type(baseUrl) == "string", "Base URL must be type `string`")

		return ApiContext.new(baseUrl)
	end

	local ReadOnly = setmetatable({}, {
		__index = function(_, key)
			-- Don't expose private members
			if string.find(key, "^_") then
				return nil
			end

			-- Existence check
			if Rojo[key] == nil then
				warn(string.format("Rojo.%s is not a valid API", tostring(key)))
				return nil
			end

			-- Permissions check
			local granted = Rojo:_permissionCheck(key)
			if not granted then
				error(string.format(
					"Attempted to read Rojo.%s, but the plugin does not have permission to do so.\nPlease first use Rojo:RequestAccess({ \"%s\" }) to gain access to this API.",
					key, key
				), 2)
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
