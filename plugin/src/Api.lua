local HttpService = game:GetService("HttpService")

local Config = require(script.Parent.Config)
local Promise = require(script.Parent.Promise)
local Version = require(script.Parent.Version)

local Api = {}
Api.__index = Api

Api.Error = {
	ServerIdMismatch = "ServerIdMismatch",
}

setmetatable(Api.Error, {
	__index = function(_, key)
		error("Invalid API.Error name " .. key, 2)
	end
})

--[[
	Api.connect(Http) -> Promise<Api>

	Create a new Api using the given HTTP implementation.

	Attempting to invoke methods on an invalid conext will throw errors!
]]
function Api.connect(http)
	local context = {
		http = http,
		serverId = nil,
		currentTime = 0,
	}

	setmetatable(context, Api)

	return context:_start()
end

function Api:_start()
	return self.http:get("/")
		:andThen(function(response)
			response = response:json()

			if response.protocolVersion ~= Config.protocolVersion then
				local message = (
					"Found a Rojo dev server, but it's using a different protocol version, and is incompatible." ..
					"\nMake sure you have matching versions of both the Rojo plugin and server!" ..
					"\n\nYour client is version %s, with protocol version %s. It expects server version %s." ..
					"\nYour server is version %s, with protocol version %s." ..
					"\n\nGo to https://github.com/LPGhatguy/rojo for more details."
				):format(
					Version.display(Config.version), Config.protocolVersion,
					Config.expectedApiVersionString,
					response.serverVersion, response.protocolVersion
				)

				return Promise.reject(message)
			end

			self.serverId = response.serverId
			self.currentTime = response.currentTime

			return self
		end)
end

function Api:getInfo()
	return self.http:get("/")
		:andThen(function(response)
			response = response:json()

			if response.serverId ~= self.serverId then
				return Promise.reject(Api.Error.ServerIdMismatch)
			end

			return response
		end)
end

function Api:read(paths)
	local body = HttpService:JSONEncode(paths)

	return self.http:post("/read", body)
		:andThen(function(response)
			response = response:json()

			if response.serverId ~= self.serverId then
				return Promise.reject(Api.Error.ServerIdMismatch)
			end

			return response.items
		end)
end

function Api:getChanges()
	local url = ("/changes/%f"):format(self.currentTime)

	return self.http:get(url)
		:andThen(function(response)
			response = response:json()

			if response.serverId ~= self.serverId then
				return Promise.reject(Api.Error.ServerIdMismatch)
			end

			self.currentTime = response.currentTime

			return response.changes
		end)
end

return Api
