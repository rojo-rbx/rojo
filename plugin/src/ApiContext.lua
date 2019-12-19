local Http = require(script.Parent.Parent.Http)
local Log = require(script.Parent.Parent.Log)
local Promise = require(script.Parent.Parent.Promise)

local Config = require(script.Parent.Config)
local Types = require(script.Parent.Types)
local Version = require(script.Parent.Version)

local validateApiInfo = Types.ifEnabled(Types.ApiInfoResponse)
local validateApiRead = Types.ifEnabled(Types.ApiReadResponse)
local validateApiSubscribe = Types.ifEnabled(Types.ApiSubscribeResponse)

--[[
	Returns a promise that will never resolve nor reject.
]]
local function hangingPromise()
	return Promise.new(function() end)
end

local function rejectFailedRequests(response)
	if response.code >= 400 then
		-- TODO: Nicer error types for responses, using response JSON if valid.
		return Promise.reject(tostring(response.code))
	end

	return response
end

local function rejectWrongProtocolVersion(infoResponseBody)
	if infoResponseBody.protocolVersion ~= Config.protocolVersion then
		local message = (
			"Found a Rojo dev server, but it's using a different protocol version, and is incompatible." ..
			"\nMake sure you have matching versions of both the Rojo plugin and server!" ..
			"\n\nYour client is version %s, with protocol version %s. It expects server version %s." ..
			"\nYour server is version %s, with protocol version %s." ..
			"\n\nGo to https://github.com/rojo-rbx/rojo for more details."
		):format(
			Version.display(Config.version), Config.protocolVersion,
			Config.expectedServerVersionString,
			infoResponseBody.serverVersion, infoResponseBody.protocolVersion
		)

		return Promise.reject(message)
	end

	return Promise.resolve(infoResponseBody)
end

local function rejectWrongPlaceId(infoResponseBody)
	if infoResponseBody.expectedPlaceIds ~= nil then
		local foundId = false

		for _, id in ipairs(infoResponseBody.expectedPlaceIds) do
			if id == game.PlaceId then
				foundId = true
				break
			end
		end

		if not foundId then
			local idList = {}
			for _, id in ipairs(infoResponseBody.expectedPlaceIds) do
				table.insert(idList, "- " .. tostring(id))
			end

			local message = (
				"Found a Rojo server, but its project is set to only be used with a specific list of places." ..
				"\nYour place ID is %s, but needs to be one of these:" ..
				"\n%s" ..
				"\n\nTo change this list, edit 'servePlaceIds' in your .project.json file."
			):format(
				tostring(game.PlaceId),
				table.concat(idList, "\n")
			)

			return Promise.reject(message)
		end
	end

	return Promise.resolve(infoResponseBody)
end

local ApiContext = {}
ApiContext.__index = ApiContext

function ApiContext.new(baseUrl)
	assert(type(baseUrl) == "string")

	local self = {
		__baseUrl = baseUrl,
		__sessionId = nil,
		__messageCursor = -1,
		__connected = true,
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

	return Http.get(url)
		:andThen(rejectFailedRequests)
		:andThen(Http.Response.json)
		:andThen(function(body)
			if body.sessionId ~= self.__sessionId then
				return Promise.reject("Server changed ID")
			end

			assert(validateApiRead(body))

			return body
		end)
end

function ApiContext:write(patch)
	local url = ("%s/api/write"):format(self.__baseUrl)

	local body = {
		sessionId = self.__sessionId,
		removed = patch.removed,
		updated = patch.updated,
	}

	-- Only add the 'added' field if the table is non-empty, or else Roblox's
	-- JSON implementation will turn the table into an array instead of an
	-- object, causing API validation to fail.
	if next(patch.added) ~= nil then
		body.added = patch.added
	end

	body = Http.jsonEncode(body)

	return Http.post(url, body)
		:andThen(rejectFailedRequests)
		:andThen(Http.Response.json)
		:andThen(function(body)
			Log.info("Write response: {:?}", body)

			return body
		end)
end

function ApiContext:retrieveMessages()
	local url = ("%s/api/subscribe/%s"):format(self.__baseUrl, self.__messageCursor)

	local function sendRequest()
		return Http.get(url)
			:catch(function(err)
				if err.type == Http.Error.Kind.Timeout then
					if self.__connected then
						return sendRequest()
					else
						return hangingPromise()
					end
				end

				return Promise.reject(err)
			end)
	end

	return sendRequest()
		:andThen(rejectFailedRequests)
		:andThen(Http.Response.json)
		:andThen(function(body)
			if body.sessionId ~= self.__sessionId then
				return Promise.reject("Server changed ID")
			end

			assert(validateApiSubscribe(body))

			self:setMessageCursor(body.messageCursor)

			return body.messages
		end)
end

return ApiContext