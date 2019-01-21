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

	-- The server gave an unexpected 400-category error, which may be the
	-- client's fault.
	ClientError = "ClientError",

	-- The server gave an unexpected 500-category error, which may be the
	-- server's fault.
	ServerError = "ServerError",
}

setmetatable(ApiContext.Error, {
	__index = function(_, key)
		error("Invalid ApiContext.Error name " .. key, 2)
	end
})

local function rejectFailedRequests(response)
	if response.code >= 400 then
		if response.code < 500 then
			return Promise.reject(ApiContext.Error.ClientError)
		else
			return Promise.reject(ApiContext.Error.ServerError)
		end
	end

	return response
end

function ApiContext.new(baseUrl)
	assert(type(baseUrl) == "string")

	local self = {
		baseUrl = baseUrl,
		serverId = nil,
		rootInstanceId = nil,
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
		:andThen(rejectFailedRequests)
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
			self.partitionRoutes = body.partitions
			self.rootInstanceId = body.rootInstanceId
		end)
end

function ApiContext:read(ids)
	local url = ("%s/api/read/%s"):format(self.baseUrl, table.concat(ids, ","))

	return Http.get(url)
		:andThen(rejectFailedRequests)
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
	local url = ("%s/api/subscribe/%s"):format(self.baseUrl, self.messageCursor)

	return Http.get(url)
		:catch(function(err)
			if err.type == HttpError.Error.Timeout then
				return self:retrieveMessages()
			end

			return Promise.reject(err)
		end)
		:andThen(rejectFailedRequests)
		:andThen(function(response)
			local body = response:json()

			if body.serverId ~= self.serverId then
				return Promise.reject("Server changed ID")
			end

			self.messageCursor = body.messageCursor

			return body.messages
		end)
end

return ApiContext