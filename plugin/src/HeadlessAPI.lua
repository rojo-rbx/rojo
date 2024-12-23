local MarketplaceService = game:GetService("MarketplaceService")

local Parent = script:FindFirstAncestor("Rojo")
local Plugin = Parent.Plugin
local Packages = Parent.Packages

local Log = require(Packages.Log)

local Config = require(Plugin.Config)
local Settings = require(Plugin.Settings)
local ApiContext = require(Plugin.ApiContext)

local cloudIdProductInfoCache = {}
local apiPermissionAllowlist = {
	Version = true,
	ProtocolVersion = true,
	RequestAccess = true,
}

export type CallerInfo = {
	Source: string,
	Type: "Local" | "Cloud" | "Studio",
	Name: string,
	Description: string,
	Creator: {
		Name: string,
		Id: number,
		HasVerifiedBadge: boolean,
	},
}

local API = {}

function API.new(app)
	local Rojo = {}

	Rojo._rateLimit = {}
	Rojo._sourceToPlugin = {}
	Rojo._permissions = Settings:get("apiPermissions") or {}
	Rojo._activePermissionRequests = {}
	Rojo._changedEvent = Instance.new("BindableEvent")
	Rojo._apiDescriptions = {}

	Rojo._apiDescriptions.Changed = {
		Type = "Event",
		Description = "An event that fires when a Rojo API property changes",
	}
	Rojo.Changed = Rojo._changedEvent.Event

	Rojo._apiDescriptions.Connected = {
		Type = "Property",
		Description = "Whether or not the plugin is connected to a Rojo server",
	}
	Rojo.Connected = if app.serveSession then app.serveSession:getStatus() == "Connected" else false

	Rojo._apiDescriptions.Address = {
		Type = "Property",
		Description = "The address (host:port) that the plugin is connected to",
	}
	Rojo.Address = nil

	Rojo._apiDescriptions.ProjectName = {
		Type = "Property",
		Description = "The name of the project that the plugin is connected to",
	}
	Rojo.ProjectName = nil

	Rojo._apiDescriptions.Version = {
		Type = "Property",
		Description = "The version of the plugin",
	}
	Rojo.Version = table.clone(Config.version)

	Rojo._apiDescriptions.ProtocolVersion = {
		Type = "Property",
		Description = "The protocol version that the plugin is using",
	}
	Rojo.ProtocolVersion = Config.protocolVersion

	function Rojo:_updateProperty(property: string, value: any?)
		local oldValue = Rojo[property]
		Rojo[property] = value
		Rojo._changedEvent:Fire(property, value, oldValue)
	end

	function Rojo:_getCallerSource()
		local traceback = string.split(debug.traceback(), "\n")
		local topLevel = traceback[#traceback - 1]

		local localPlugin = string.match(topLevel, "user_.-%.%w+")
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
			local info = cloudIdProductInfoCache[cloudId]
			if not info then
				local success, newInfo =
					pcall(MarketplaceService.GetProductInfo, MarketplaceService, tonumber(cloudId), Enum.InfoType.Asset)
				if success then
					cloudIdProductInfoCache[cloudId] = newInfo
					info = newInfo
				end
			end

			if info then
				return info.Name
					.. " by "
					.. (if info.Creator.CreatorType == "User" then "@" else "")
					.. info.Creator.Name
			else
				-- Fallback to the name of the instance uploaded inside this plugin
				-- The reason this is not ideal is because creators often upload a folder named "Main" or something
				return cloudInstance
			end
		end

		return "Command Bar"
	end

	function Rojo:_getCallerInfoFromSource(source: string): CallerInfo
		local localPlugin = string.match(source, "user_(.+)")
		if localPlugin then
			return {
				Source = source,
				Type = "Local",
				Name = localPlugin,
				Description = "Locally installed plugin.",
				Creator = {
					Name = "Unknown",
					Id = 0,
					HasVerifiedBadge = false,
				},
			}
		end

		local cloudId = string.match(source, "cloud_(%d+)")
		if cloudId then
			local info = cloudIdProductInfoCache[cloudId]
			if not info then
				local success, newInfo =
					pcall(MarketplaceService.GetProductInfo, MarketplaceService, tonumber(cloudId), Enum.InfoType.Asset)
				if success then
					cloudIdProductInfoCache[cloudId] = newInfo
					info = newInfo
				end
			end

			if info then
				return {
					Source = source,
					Type = "Cloud",
					Name = info.Name,
					Description = info.Description,
					Creator = {
						Name = (if info.Creator.CreatorType == "User" then "@" else "") .. info.Creator.Name,
						Id = info.Creator.CreatorTargetId,
						HasVerifiedBadge = info.Creator.HasVerifiedBadge,
					},
				}
			else
				return {
					Source = source,
					Type = "Cloud",
					Name = source,
					Description = "Could not retrieve plugin asset info.",
					Creator = {
						Name = "Unknown",
						Id = 0,
						HasVerifiedBadge = false,
					},
				}
			end
		end

		return {
			Source = source,
			Type = "Studio",
			Name = "Command Bar",
			Description = "Command bar in Roblox Studio.",
			Creator = {
				Name = "N/A",
				Id = 0,
				HasVerifiedBadge = false,
			},
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

	Rojo._permissionsChangedEvent = Instance.new("BindableEvent")
	Rojo._permissionsChanged = Rojo._permissionsChangedEvent.Event

	function Rojo:_permissionCheck(key: string): boolean
		if apiPermissionAllowlist[key] then
			return true
		end

		local source = Rojo:_getCallerSource()
		if Rojo._permissions[source] == nil then
			return false
		end

		return not not Rojo._permissions[source][key]
	end

	function Rojo:_setPermissions(source, name, permissions)
		if next(permissions) == nil then
			Rojo:_removePermissions(source, name)
			return
		end

		-- Set permissions
		local sourcePermissions = {}
		for _, api in permissions do
			Log.info(string.format("Granting '%s' access to Rojo.%s", name, api))
			sourcePermissions[api] = true
		end

		-- Update stored permissions
		Rojo._permissions[source] = sourcePermissions
		Settings:set("apiPermissions", Rojo._permissions)

		-- Share changes
		Rojo._permissionsChangedEvent:Fire(source, sourcePermissions)
	end

	function Rojo:_removePermissions(source, name)
		Rojo._permissions[source] = nil
		Log.info(string.format("Denying access to Rojo APIs for '%s'", name))

		-- Update stored permissions
		Settings:set("apiPermissions", Rojo._permissions)

		-- Share changes
		Rojo._permissionsChangedEvent:Fire(source, nil)
	end

	Rojo._apiDescriptions.RequestAccess = {
		Type = "Method",
		Description = "Used to gain access to Rojo API members",
	}
	function Rojo:RequestAccess(plugin: Plugin, apis: { string }): boolean
		assert(type(apis) == "table", "Rojo:RequestAccess expects an array of valid API names as the second argument")
		assert(
			typeof(plugin) == "Instance" and plugin:IsA("Plugin"),
			"Rojo:RequestAccess expects a Plugin as the first argument"
		)

		local source, name = Rojo:_getCallerSource(), Rojo:_getCallerName()
		Rojo._sourceToPlugin[source] = plugin

		if Rojo:_checkRateLimit("RequestAccess") then
			-- Because this opens a popup, we dont want to let users get spammed by it
			return false
		end

		if Rojo._activePermissionRequests[source] then
			-- If a request is already active, exit
			error(
				"Rojo:RequestAccess cannot be called in multiple threads at once. Please call it once and wait for the response before calling it again.",
				2
			)
		end
		Rojo._activePermissionRequests[source] = true

		-- Sanitize request
		local sanitizedApis = {}
		for _, api in apis do
			if Rojo._apiDescriptions[api] ~= nil and table.find(sanitizedApis, api) == nil then
				table.insert(sanitizedApis, api)
			else
				warn(string.format("Rojo.%s is not a valid API", tostring(api)))
			end
		end
		assert(#sanitizedApis > 0, "Rojo:RequestAccess expects an array of valid API names")
		table.sort(sanitizedApis)

		local alreadyAllowed = true
		if Rojo._permissions[source] == nil then
			alreadyAllowed = false
		else
			for _, api in sanitizedApis do
				if not Rojo._permissions[source][api] then
					alreadyAllowed = false
					break
				end
			end
		end

		if alreadyAllowed then
			Rojo._activePermissionRequests[source] = nil
			return true
		end

		local granted = app:requestPermission(plugin, source, name, sanitizedApis, false)
		if granted then
			Rojo:_setPermissions(source, name, sanitizedApis)
		else
			Rojo:_removePermissions(source, name)
		end

		Rojo._activePermissionRequests[source] = nil
		return granted
	end

	Rojo._apiDescriptions.Test = {
		Type = "Method",
		Description = "Prints the given arguments to the console. Useful during development for testing purposes.",
	}
	function Rojo:Test(...)
		local args = table.pack(...)
		for i = 1, args.n do
			local v = args[i]
			local t = type(v)
			if t == "string" then
				args[i] = string.format("%q", v)
			else
				args[i] = tostring(v)
			end
		end

		print(
			string.format(
				"Rojo:Test(%s) called from '%s' (%s)",
				table.concat(args, ", "),
				Rojo:_getCallerName(),
				Rojo:_getCallerSource()
			)
		)
	end

	Rojo._apiDescriptions.ConnectAsync = {
		Type = "Method",
		Description = "Connects to a Rojo server",
	}
	function Rojo:ConnectAsync(host: string?, port: string?)
		assert(type(host) == "string" or host == nil, "Host must be type `string?`")
		assert(type(port) == "string" or port == nil, "Port must be type `string?`")

		if Rojo:_checkRateLimit("ConnectAsync") then
			return
		end

		app:startSession(host, port)
	end

	Rojo._apiDescriptions.DisconnectAsync = {
		Type = "Method",
		Description = "Disconnects from the Rojo server",
	}
	function Rojo:DisconnectAsync()
		if Rojo:_checkRateLimit("DisconnectAsync") then
			return
		end

		app:endSession()
	end

	Rojo._apiDescriptions.GetSetting = {
		Type = "Method",
		Description = "Gets a Rojo setting",
	}
	function Rojo:GetSetting(setting: string): any
		assert(type(setting) == "string", "Setting must be type `string`")

		return Settings:get(setting)
	end

	Rojo._apiDescriptions.Notify = {
		Type = "Method",
		Description = "Shows a notification in the Rojo UI",
	}
	function Rojo:Notify(
		msg: string,
		timeout: number?,
		actions: { [string]: { text: string, style: string, layoutOrder: number, onClick: () -> () } }?
	): () -> ()
		assert(type(msg) == "string", "Message must be type `string`")
		assert(type(timeout) == "number" or timeout == nil, "Timeout must be type `number?`")
		assert((actions == nil) or (type(actions) == "table"), "Actions must be table or nil")

		if Rojo:_checkRateLimit("Notify") then
			return function() end
		end

		local sanitizedActions = nil
		if actions then
			sanitizedActions = {}
			for id, action in actions do
				assert(type(id) == "string", "Actions key must be string")
				local actionId = "Actions." .. id
				assert(type(action) == "table", actionId .. " must be table")
				assert(type(action.text) == "string", actionId .. ".text must be string")
				assert(type(action.style) == "string", actionId .. ".style must be string")
				assert(
					action.style == "Solid" or action.style == "Bordered",
					actionId .. ".style must be 'Solid' or 'Bordered'"
				)
				assert(type(action.layoutOrder) == "number", actionId .. ".layoutOrder must be number")
				assert(type(action.onClick) == "function", actionId .. ".onClick must be function")

				sanitizedActions[id] = {
					text = action.text,
					style = action.style,
					layoutOrder = action.layoutOrder,
					onClick = function()
						task.spawn(action.onClick)
					end,
				}
			end
		end

		return app:addThirdPartyNotification(
			Rojo:_getCallerInfoFromSource(Rojo:_getCallerSource()),
			msg,
			timeout,
			sanitizedActions
		)
	end

	Rojo._apiDescriptions.GetHostAndPort = {
		Type = "Method",
		Description = "Gets the host and port that Rojo is set to",
	}
	function Rojo:GetHostAndPort(): (string, string)
		return app:getHostAndPort()
	end

	Rojo._apiDescriptions.CreateApiContext = {
		Type = "Method",
		Description = "Creates a new API context",
	}
	function Rojo:CreateApiContext(baseUrl: string)
		assert(type(baseUrl) == "string", "Base URL must be type `string`")

		return ApiContext.new(baseUrl)
	end

	local ReadOnly = newproxy(true)
	local Metatable = getmetatable(ReadOnly)
	Metatable.__index = function(_, key)
		-- Don't expose private members
		if string.find(key, "^_") then
			return nil
		end

		-- Existence check
		if Rojo._apiDescriptions[key] == nil then
			warn(string.format("Rojo.%s is not a valid API", tostring(key)))
			return nil
		end

		-- Permissions check
		local granted = Rojo:_permissionCheck(key)
		if not granted then
			error(
				string.format(
					'Attempted to read Rojo.%s, but the plugin does not have permission to do so.\nPlease first use Rojo:RequestAccess({ "%s" }) to gain access to this API.',
					key,
					key
				),
				2
			)
		end

		return Rojo[key]
	end
	Metatable.__newindex = function(_, key, value)
		error(string.format("Attempted to set Rojo.%s to %q but it's a read-only value", key, value), 2)
		return
	end
	Metatable.__metatable = "The metatable of the Rojo API is locked"

	return Rojo, ReadOnly
end

return API
