local Packages = script.Parent.Parent.Packages
local Http = require(Packages.Http)
local Log = require(Packages.Log)
local Promise = require(Packages.Promise)

local Config = require(script.Parent.Config)
local Types = require(script.Parent.Types)
local Version = require(script.Parent.Version)

local validateApiInfo = Types.ifEnabled(Types.ApiInfoResponse)
local validateApiRead = Types.ifEnabled(Types.ApiReadResponse)
local validateApiSubscribe = Types.ifEnabled(Types.ApiSubscribeResponse)
local validateApiSerialize = Types.ifEnabled(Types.ApiSerializeResponse)
local validateApiRefPatch = Types.ifEnabled(Types.ApiRefPatchResponse)

local function rejectFailedRequests(response)
	if response.code >= 400 then
		local message = string.format("HTTP %s:\n%s", tostring(response.code), response.body)

		return Promise.reject(message)
	end

	return response
end

local function rejectWrongProtocolVersion(infoResponseBody)
	if infoResponseBody.protocolVersion ~= Config.protocolVersion then
		local message = (
			"Found a Rojo dev server, but it's using a different protocol version, and is incompatible."
			.. "\nMake sure you have matching versions of both the Rojo plugin and server!"
			.. "\n\nYour client is version %s, with protocol version %s. It expects server version %s."
			.. "\nYour server is version %s, with protocol version %s."
			.. "\n\nGo to https://github.com/rojo-rbx/rojo for more details."
		):format(
			Version.display(Config.version),
			Config.protocolVersion,
			Config.expectedServerVersionString,
			infoResponseBody.serverVersion,
			infoResponseBody.protocolVersion
		)

		return Promise.reject(message)
	end

	return Promise.resolve(infoResponseBody)
end

local function rejectWrongPlaceId(infoResponseBody)
	if infoResponseBody.expectedPlaceIds ~= nil then
		local foundId = table.find(infoResponseBody.expectedPlaceIds, game.PlaceId)

		if not foundId then
			local idList = {}
			for _, id in ipairs(infoResponseBody.expectedPlaceIds) do
				table.insert(idList, "- " .. tostring(id))
			end

			local message = (
				"Found a Rojo server, but its project is set to only be used with a specific list of places."
				.. "\nYour place ID is %u, but needs to be one of these:"
				.. "\n%s"
				.. "\n\nTo change this list, edit 'servePlaceIds' in your .project.json file."
			):format(game.PlaceId, table.concat(idList, "\n"))

			return Promise.reject(message)
		end
	end

	if infoResponseBody.unexpectedPlaceIds ~= nil then
		local foundId = table.find(infoResponseBody.unexpectedPlaceIds, game.PlaceId)

		if foundId then
			local idList = {}
			for _, id in ipairs(infoResponseBody.unexpectedPlaceIds) do
				table.insert(idList, "- " .. tostring(id))
			end

			local message = (
				"Found a Rojo server, but its project is set to not be used with a specific list of places."
				.. "\nYour place ID is %u, but needs to not be one of these:"
				.. "\n%s"
				.. "\n\nTo change this list, edit 'blockedPlaceIds' in your .project.json file."
			):format(game.PlaceId, table.concat(idList, "\n"))

			return Promise.reject(message)
		end
	end

	return Promise.resolve(infoResponseBody)
end

local ApiContext = {}
ApiContext.__index = ApiContext

function ApiContext.new(baseUrl)
	assert(type(baseUrl) == "string", "baseUrl must be a string")

	local self = {
		__baseUrl = baseUrl,
		__sessionId = nil,
		__messageCursor = -1,
		__connected = true,
		__activeRequests = {},
	}

	return setmetatable(self, ApiContext)
end

function ApiContext:__fmtDebug(output)
	output:writeLine("ApiContext {{")
	output:indent()

	output:writeLine("Connected: {}", self.__connected)
	output:writeLine("Base URL: {}", self.__baseUrl)
	output:writeLine("Session ID: {}", self.__sessionId)
	output:writeLine("Message Cursor: {}", self.__messageCursor)

	output:unindent()
	output:write("}")
end

function ApiContext:disconnect()
	self.__connected = false
	for request in self.__activeRequests do
		Log.trace("Cancelling request {}", request)
		request:cancel()
	end
	self.__activeRequests = {}
end

function ApiContext:setMessageCursor(index)
	self.__messageCursor = index
end

function ApiContext:connect()
	local url = ("%s/api/rojo"):format(self.__baseUrl)

	return Http.get(url)
		:andThen(rejectFailedRequests)
		:andThen(Http.Response.json)
		:andThen(rejectWrongProtocolVersion)
		:andThen(function(body)
			assert(validateApiInfo(body))

			return body
		end)
		:andThen(rejectWrongPlaceId)
		:andThen(function(body)
			self.__sessionId = body.sessionId

			return body
		end)
end

function ApiContext:read(ids)
	local url = ("%s/api/read/%s"):format(self.__baseUrl, table.concat(ids, ","))

	return Http.get(url):andThen(rejectFailedRequests):andThen(Http.Response.json):andThen(function(body)
		if body.sessionId ~= self.__sessionId then
			return Promise.reject("Server changed ID")
		end

		assert(validateApiRead(body))

		return body
	end)
end

function ApiContext:write(patch)
	local url = ("%s/api/write"):format(self.__baseUrl)

	local updated = {}
	for _, update in ipairs(patch.updated) do
		local fixedUpdate = {
			id = update.id,
			changedName = update.changedName,
		}

		if next(update.changedProperties) ~= nil then
			fixedUpdate.changedProperties = update.changedProperties
		end

		table.insert(updated, fixedUpdate)
	end

	-- Only add the 'added' field if the table is non-empty, or else Roblox's
	-- JSON implementation will turn the table into an array instead of an
	-- object, causing API validation to fail.
	local added
	if next(patch.added) ~= nil then
		added = patch.added
	end

	local body = {
		sessionId = self.__sessionId,
		removed = patch.removed,
		updated = updated,
		added = added,
	}

	body = Http.jsonEncode(body)

	return Http.post(url, body):andThen(rejectFailedRequests):andThen(Http.Response.json):andThen(function(responseBody)
		Log.info("Write response: {:?}", responseBody)

		return responseBody
	end)
end

function ApiContext:retrieveMessages()
	local url = ("%s/api/subscribe/%s"):format(self.__baseUrl, self.__messageCursor)

	local function sendRequest()
		local request = Http.get(url):catch(function(err)
			if err.type == Http.Error.Kind.Timeout and self.__connected then
				return sendRequest()
			end

			return Promise.reject(err)
		end)

		Log.trace("Tracking request {}", request)
		self.__activeRequests[request] = true

		return request:finally(function(...)
			Log.trace("Cleaning up request {}", request)
			self.__activeRequests[request] = nil
			return ...
		end)
	end

	return sendRequest():andThen(rejectFailedRequests):andThen(Http.Response.json):andThen(function(body)
		if body.sessionId ~= self.__sessionId then
			return Promise.reject("Server changed ID")
		end

		assert(validateApiSubscribe(body))

		self:setMessageCursor(body.messageCursor)

		return body.messages
	end)
end

function ApiContext:open(id)
	local url = ("%s/api/open/%s"):format(self.__baseUrl, id)

	return Http.post(url, ""):andThen(rejectFailedRequests):andThen(Http.Response.json):andThen(function(body)
		if body.sessionId ~= self.__sessionId then
			return Promise.reject("Server changed ID")
		end

		return nil
	end)
end

function ApiContext:serialize(ids: { string })
	local url = ("%s/api/serialize/%s"):format(self.__baseUrl, table.concat(ids, ","))

	return Http.get(url):andThen(rejectFailedRequests):andThen(Http.Response.json):andThen(function(body)
		if body.sessionId ~= self.__sessionId then
			return Promise.reject("Server changed ID")
		end

		assert(validateApiSerialize(body))

		return body
	end)
end

function ApiContext:refPatch(ids: { string })
	local url = ("%s/api/ref-patch/%s"):format(self.__baseUrl, table.concat(ids, ","))

	return Http.get(url):andThen(rejectFailedRequests):andThen(Http.Response.json):andThen(function(body)
		if body.sessionId ~= self.__sessionId then
			return Promise.reject("Server changed ID")
		end

		assert(validateApiRefPatch(body))

		return body
	end)
end

return ApiContext
