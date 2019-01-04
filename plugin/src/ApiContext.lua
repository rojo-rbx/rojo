local Promise = require(script.Parent.Parent.Promise)

local Config = require(script.Parent.Config)
local Version = require(script.Parent.Version)
local Http = require(script.Parent.Http)
local HttpError = require(script.Parent.HttpError)

local ApiContext = {}
ApiContext.__index = ApiContext

-- TODO: Audit cases of errors and create enum values for each of them.
ApiContext.Error = {
	ServerIdMismatch = "ServerIdMismatch",
}

setmetatable(ApiContext.Error, {
	__index = function(_, key)
		error("Invalid ApiContext.Error name " .. key, 2)
	end
})

-- TODO: Switch to onMessages and batch processing
function ApiContext.new(baseUrl)
	assert(type(baseUrl) == "string")

	local self = {
		baseUrl = baseUrl,
		onMessageCallback = nil,
		serverId = nil,
		rootInstanceId = nil,
		instanceMetadataMap = nil,
		connected = false,
		messageCursor = -1,
		partitionRoutes = nil,
	}

	setmetatable(self, ApiContext)

	return self
end

function ApiContext:onMessage(callback)
	self.onMessageCallback = callback
end

function ApiContext:connect()
	local url = ("%s/api/rojo"):format(self.baseUrl)

	return Http.get(url)
		:andThen(function(response)
			local body = response:json()

			if body.protocolVersion ~= Config.protocolVersion then
				local message = (
					"Found a Rojo dev server, but it's using a different protocol version, and is incompatible." ..
					"\nMake sure you have matching versions of both the Rojo plugin and server!" ..
					"\n\nYour client is version %s, with protocol version %s. It expects server version %s." ..
					"\nYour server is version %s, with protocol version %s." ..
					"\n\nGo to https://github.com/LPGhatguy/rojo for more details."
				):format(
					Version.display(Config.version), Config.protocolVersion,
					Config.expectedApiContextVersionString,
					body.serverVersion, body.protocolVersion
				)

				return Promise.reject(message)
			end

			if body.expectedPlaceIds ~= nil then
				local foundId = false

				for _, id in ipairs(body.expectedPlaceIds) do
					if id == game.PlaceId then
						foundId = true
						break
					end
				end

				if not foundId then
					local idList = {}
					for _, id in ipairs(body.expectedPlaceIds) do
						table.insert(idList, "- " .. tostring(id))
					end

					local message = (
						"Found a Rojo server, but its project is set to only be used with a specific list of places." ..
						"\nYour place ID is %s, but needs to be one of these:" ..
						"\n%s" ..
						"\n\nTo change this list, edit 'servePlaceIds' in roblox-project.json"
					):format(
						tostring(game.PlaceId),
						table.concat(idList, "\n")
					)

					return Promise.reject(message)
				end
			end

			self.serverId = body.serverId
			self.connected = true
			self.partitionRoutes = body.partitions
			self.rootInstanceId = body.rootInstanceId
			self.instanceMetadataMap = body.instanceMetadataMap
		end)
end

function ApiContext:read(ids)
	if not self.connected then
		return Promise.reject()
	end

	local url = ("%s/api/read/%s"):format(self.baseUrl, table.concat(ids, ","))

	return Http.get(url)
		:catch(function(err)
			self.connected = false

			return Promise.reject(err)
		end)
		:andThen(function(response)
			local body = response:json()

			if body.serverId ~= self.serverId then
				return Promise.reject("Server changed ID")
			end

			self.messageCursor = body.messageCursor

			return body
		end)
end

function ApiContext:retrieveMessages()
	if not self.connected then
		return Promise.reject()
	end

	local url = ("%s/api/subscribe/%s"):format(self.baseUrl, self.messageCursor)

	return Http.get(url)
		:catch(function(err)
			if err.type == HttpError.Error.Timeout then
				return self:retrieveMessages()
			end

			self.connected = false

			return Promise.reject(err)
		end)
		:andThen(function(response)
			local body = response:json()

			if body.serverId ~= self.serverId then
				return Promise.reject("Server changed ID")
			end

			local promise = Promise.resolve(nil)

			for _, message in ipairs(body.messages) do
				promise = promise:andThen(function()
					return self.onMessageCallback(message)
				end)
			end

			self.messageCursor = body.messageCursor

			return promise
		end)
		:andThen(function()
			return self:retrieveMessages()
		end)
end

return ApiContext