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
		error("Invalid API.Error name " .. key, 2)
	end
})

function ApiContext.new(baseUrl, onMessage)
	assert(type(baseUrl) == "string")
	assert(type(onMessage) == "function")

	local context = {
		baseUrl = baseUrl,
		onMessage = onMessage,
		serverId = nil,
		rootInstanceId = nil,
		connected = false,
		messageCursor = -1,
		partitionRoutes = nil,
	}

	setmetatable(context, ApiContext)

	return context
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

			self.serverId = body.serverId
			self.connected = true
			self.partitionRoutes = body.partitions
			self.rootInstanceId = body.rootInstanceId
		end)
end

function ApiContext:read(ids)
	if not self.connected then
		return Promise.reject()
	end

	local url = ("%s/api/read/%s"):format(self.baseUrl, table.concat(ids, ","))

	return Http.get(url)
		:andThen(function(response)
			local body = response:json()

			if body.serverId ~= self.serverId then
				return Promise.reject("Server changed ID")
			end

			self.messageCursor = body.messageCursor

			return body
		end, function(err)
			self.connected = false

			return Promise.reject(err)
		end)
end

function ApiContext:retrieveMessages()
	if not self.connected then
		return Promise.reject()
	end

	local url = ("%s/api/subscribe/%s"):format(self.baseUrl, self.messageCursor)

	return Http.get(url)
		:andThen(function(response)
			local body = response:json()

			if body.serverId ~= self.serverId then
				return Promise.reject("Server changed ID")
			end

			for _, message in ipairs(body.messages) do
				self.onMessage(message)
			end

			self.messageCursor = body.messageCursor

			return self:retrieveMessages()
		end, function(err)
			if err.type == HttpError.Error.Timeout then
				return self:retrieveMessages()
			end

			self.connected = false

			return Promise.reject(err)
		end)
end

return ApiContext
