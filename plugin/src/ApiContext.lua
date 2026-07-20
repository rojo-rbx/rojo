local Packages = script.Parent.Parent.Packages
local HttpService = game:GetService("HttpService")
local Http = require(Packages.Http)
local Log = require(Packages.Log)
local Promise = require(Packages.Promise)

local Config = require(script.Parent.Config)
local Types = require(script.Parent.Types)
local Version = require(script.Parent.Version)

local validateApiInfo = Types.ifEnabled(Types.ApiInfoResponse)
local validateApiRead = Types.ifEnabled(Types.ApiReadResponse)
local validateApiSocketPacket = Types.ifEnabled(Types.ApiSocketPacket)
local validateApiSerialize = Types.ifEnabled(Types.ApiSerializeResponse)
local validateApiRefPatch = Types.ifEnabled(Types.ApiRefPatchResponse)

local EXEC_COMPLETION_BODY_LIMIT_BYTES = 256 * 1024
local AUTOMATION_COMPLETION_BODY_LIMIT_BYTES = 4 * 1024 * 1024
local AUTOMATION_HANDLER_VERSION = 2

local function generatePluginSessionId()
	return HttpService:GenerateGUID(false)
end

local function beginPluginSession(apiContext, serverSessionId, generateId)
	apiContext.__sessionId = serverSessionId
	apiContext.__pluginSessionId = (generateId or generatePluginSessionId)()
	return apiContext.__pluginSessionId
end

local function withExecSession(payload, pluginSessionId, studioMode)
	local body = table.clone(payload)
	body.pluginSessionId = pluginSessionId
	body.studioMode = studioMode
	return body
end

local function buildExecClaimUrl(baseUrl, pluginSessionId, studioMode)
	return ("%s/api/exec/jobs/next?pluginSessionId=%s&studioMode=%s"):format(baseUrl, pluginSessionId, studioMode)
end

local function buildAutomationClaimUrl(baseUrl, pluginSessionId, studioMode)
	return ("%s/api/automation/jobs/next?pluginSessionId=%s&studioMode=%s"):format(baseUrl, pluginSessionId, studioMode)
end

local function rejectFailedRequests(response)
	if response.code >= 400 then
		local message = string.format("HTTP %s:\n%s", tostring(response.code), response.body)

		return Promise.reject(message)
	end

	return response
end

local function decodeExecResponse(response, validator, description, protocolName)
	protocolName = protocolName or "Prism exec"
	local decodeOk, body = pcall(response.msgpack, response)
	if not decodeOk then
		return Promise.reject(
			string.format("%s protocol error: could not decode %s: %s", protocolName, description, tostring(body))
		)
	end

	local valid, validationError = validator(body)
	if not valid then
		return Promise.reject(
			string.format("%s protocol error: malformed %s: %s", protocolName, description, tostring(validationError))
		)
	end

	return body
end

local function isCompletionConflict(errorValue)
	-- The existing HTTP wrapper intentionally turns every non-2xx response
	-- into Http.Error.Unknown and includes the status at the start of its
	-- message. Keep this compatibility check here instead of creating another
	-- HTTP connection path just for exec.
	return tostring(errorValue):find("^Unknown HTTP error: 409:") ~= nil
end

local function rejectWrongProtocolVersion(infoResponseBody)
	if infoResponseBody.protocolVersion ~= Config.protocolVersion then
		local message = (
			"Found a Prism dev server, but it's using a different protocol version, and is incompatible."
			.. "\nMake sure you have matching versions of both the Prism plugin and server!"
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
				"Found a Prism server, but its project is set to only be used with a specific list of places."
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
				"Found a Prism server, but its project is set to not be used with a specific list of places."
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
		__pluginSessionId = nil,
		__connectionGeneration = 0,
		__messageCursor = -1,
		__wsClient = nil,
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
	output:writeLine("Plugin Session ID: {}", self.__pluginSessionId)
	output:writeLine("Message Cursor: {}", self.__messageCursor)

	output:unindent()
	output:write("}")
end

function ApiContext:disconnect()
	self.__connected = false
	self.__connectionGeneration += 1
	for request in self.__activeRequests do
		Log.trace("Cancelling request {}", request)
		request:cancel()
	end
	self.__activeRequests = {}

	if self.__wsClient then
		Log.trace("Closing WebSocket client")
		self.__wsClient:Close()
	end
	self.__wsClient = nil
	self.__pluginSessionId = nil
end

function ApiContext:setMessageCursor(index)
	self.__messageCursor = index
end

function ApiContext:connect()
	self.__connected = true
	self.__connectionGeneration += 1
	local generation = self.__connectionGeneration
	local url = ("%s/api/rojo"):format(self.__baseUrl)

	return Http.get(url)
		:andThen(rejectFailedRequests)
		:andThen(Http.Response.msgpack)
		:andThen(rejectWrongProtocolVersion)
		:andThen(function(body)
			assert(validateApiInfo(body))

			return body
		end)
		:andThen(rejectWrongPlaceId)
		:andThen(function(body)
			if not self.__connected or self.__connectionGeneration ~= generation then
				return Promise.reject("Connection was stopped before server verification completed")
			end
			beginPluginSession(self, body.sessionId)

			return body
		end)
end

function ApiContext:getPluginSessionId()
	return self.__pluginSessionId
end

function ApiContext:read(ids)
	local url = ("%s/api/read/%s"):format(self.__baseUrl, table.concat(ids, ","))

	return Http.get(url):andThen(rejectFailedRequests):andThen(Http.Response.msgpack):andThen(function(body)
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

	-- Only add the 'added' field if the table is non-empty, or else the msgpack
	-- encode implementation will turn the table into an array instead of a map,
	-- causing API validation to fail.
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

	body = Http.msgpackEncode(body)

	return Http.post(url, body)
		:andThen(rejectFailedRequests)
		:andThen(Http.Response.msgpack)
		:andThen(function(responseBody)
			Log.info("Write response: {:?}", responseBody)

			return responseBody
		end)
end

function ApiContext:connectWebSocket(packetHandlers)
	local url = ("%s/api/socket/%s"):format(self.__baseUrl, self.__messageCursor)
	-- Convert HTTP/HTTPS URL to WS/WSS
	url = url:gsub("^http://", "ws://"):gsub("^https://", "wss://")

	return Promise.new(function(resolve, reject)
		local success, wsClient =
			pcall(HttpService.CreateWebStreamClient, HttpService, Enum.WebStreamClientType.WebSocket, {
				Url = url,
			})
		if not success then
			reject("Failed to create WebSocket client: " .. tostring(wsClient))
			return
		end
		self.__wsClient = wsClient

		local closed, errored, received

		received = self.__wsClient.MessageReceived:Connect(function(msg)
			local data = Http.msgpackDecode(msg)
			if data.sessionId ~= self.__sessionId then
				Log.warn("Received message with wrong session ID; ignoring")
				return
			end

			assert(validateApiSocketPacket(data))

			Log.trace("Received websocket packet: {:#?}", data)

			local handler = packetHandlers[data.packetType]
			if handler then
				local ok, err = pcall(handler, data.body)
				if not ok then
					Log.error("Error in WebSocket packet handler for type '%s': %s", data.packetType, err)
				end
			else
				Log.warn("No handler for WebSocket packet type '%s'", data.packetType)
			end
		end)

		closed = self.__wsClient.Closed:Connect(function()
			closed:Disconnect()
			errored:Disconnect()
			received:Disconnect()

			if self.__connected then
				reject("WebSocket connection closed unexpectedly")
			else
				resolve()
			end
		end)

		errored = self.__wsClient.Error:Connect(function(code, msg)
			closed:Disconnect()
			errored:Disconnect()
			received:Disconnect()

			reject("WebSocket error: " .. code .. " - " .. msg)
		end)
	end)
end

function ApiContext:open(id)
	local url = ("%s/api/open/%s"):format(self.__baseUrl, id)

	return Http.post(url, ""):andThen(rejectFailedRequests):andThen(Http.Response.msgpack):andThen(function(body)
		if body.sessionId ~= self.__sessionId then
			return Promise.reject("Server changed ID")
		end

		return nil
	end)
end

function ApiContext:serialize(ids: { string })
	local url = ("%s/api/serialize"):format(self.__baseUrl)
	local request_body = Http.msgpackEncode({ sessionId = self.__sessionId, ids = ids })

	return Http.post(url, request_body)
		:andThen(rejectFailedRequests)
		:andThen(Http.Response.msgpack)
		:andThen(function(response_body)
			if response_body.sessionId ~= self.__sessionId then
				return Promise.reject("Server changed ID")
			end

			assert(validateApiSerialize(response_body))

			return response_body
		end)
end

function ApiContext:refPatch(ids: { string })
	local url = ("%s/api/ref-patch"):format(self.__baseUrl)
	local request_body = Http.msgpackEncode({ sessionId = self.__sessionId, ids = ids })

	return Http.post(url, request_body)
		:andThen(rejectFailedRequests)
		:andThen(Http.Response.msgpack)
		:andThen(function(response_body)
			if response_body.sessionId ~= self.__sessionId then
				return Promise.reject("Server changed ID")
			end

			assert(validateApiRefPatch(response_body))

			return response_body
		end)
end

function ApiContext:claimNextExecJob(studioMode)
	local validMode, modeError = Types.StudioMode(studioMode)
	if not validMode then
		return Promise.reject(string.format("Cannot claim exec job: %s", tostring(modeError)))
	end
	if self.__pluginSessionId == nil then
		return Promise.reject("Cannot claim exec job without an active plugin session")
	end

	local url = buildExecClaimUrl(self.__baseUrl, self.__pluginSessionId, studioMode)

	return Http.get(url):andThen(function(response)
		if response.code == 204 then
			return nil
		end

		if response.code ~= 200 then
			return Promise.reject(string.format("Unexpected exec claim response status %s", tostring(response.code)))
		end

		return decodeExecResponse(response, Types.ApiExecClaimResponse, "claimed-job response")
	end)
end

function ApiContext:completeExecJob(jobId, payload, studioMode)
	local validJobId, jobIdError = Types.Uuid(jobId)
	if not validJobId then
		return Promise.reject(string.format("Cannot complete exec job: %s", tostring(jobIdError)))
	end
	local validMode, modeError = Types.StudioMode(studioMode)
	if not validMode then
		return Promise.reject(string.format("Cannot complete exec job: %s", tostring(modeError)))
	end
	if self.__pluginSessionId == nil then
		return Promise.reject("Cannot complete exec job without an active plugin session")
	end

	payload = withExecSession(payload, self.__pluginSessionId, studioMode)

	local encodeOk, body = pcall(Http.msgpackEncode, payload)
	if not encodeOk then
		return Promise.reject("Could not encode exec completion payload: " .. tostring(body))
	end
	if #body > EXEC_COMPLETION_BODY_LIMIT_BYTES then
		return Promise.reject(
			string.format(
				"Exec completion payload is %d bytes, exceeding the %d-byte server limit",
				#body,
				EXEC_COMPLETION_BODY_LIMIT_BYTES
			)
		)
	end

	local url = ("%s/api/exec/jobs/%s/complete"):format(self.__baseUrl, jobId)

	return Http.post(url, body)
		:andThen(function(response)
			if response.code ~= 200 then
				return Promise.reject(
					string.format("Unexpected exec completion response status %s", tostring(response.code))
				)
			end

			local responseBody = decodeExecResponse(response, Types.ApiExecJobResponse, "completion response")
			return Promise.resolve(responseBody):andThen(function(validatedBody)
				return {
					status = "accepted",
					job = validatedBody,
				}
			end)
		end)
		:catch(function(errorValue)
			if isCompletionConflict(errorValue) then
				return {
					status = "conflict",
				}
			end

			return Promise.reject(errorValue)
		end)
end

function ApiContext:claimNextAutomationJob(studioMode)
	local validMode, modeError = Types.StudioMode(studioMode)
	if not validMode then
		return Promise.reject(string.format("Cannot claim automation job: %s", tostring(modeError)))
	end
	if self.__pluginSessionId == nil then
		return Promise.reject("Cannot claim automation job without an active plugin session")
	end

	local url = buildAutomationClaimUrl(self.__baseUrl, self.__pluginSessionId, studioMode)
	return Http.get(url):andThen(function(response)
		if response.code == 204 then
			return nil
		end
		if response.code ~= 200 then
			return Promise.reject(
				string.format("Unexpected automation claim response status %s", tostring(response.code))
			)
		end
		return decodeExecResponse(
			response,
			Types.ApiAutomationClaimResponse,
			"claimed-job response",
			"Prism automation"
		)
	end)
end

function ApiContext:completeAutomationJob(jobId, payload, studioMode)
	local validJobId, jobIdError = Types.Uuid(jobId)
	if not validJobId then
		return Promise.reject(string.format("Cannot complete automation job: %s", tostring(jobIdError)))
	end
	local validMode, modeError = Types.StudioMode(studioMode)
	if not validMode then
		return Promise.reject(string.format("Cannot complete automation job: %s", tostring(modeError)))
	end
	if self.__pluginSessionId == nil then
		return Promise.reject("Cannot complete automation job without an active plugin session")
	end

	payload = withExecSession(payload, self.__pluginSessionId, studioMode)
	local encodeOk, body = pcall(Http.msgpackEncode, payload)
	if not encodeOk then
		return Promise.reject("Could not encode automation completion payload: " .. tostring(body))
	end
	if #body > AUTOMATION_COMPLETION_BODY_LIMIT_BYTES then
		return Promise.reject(
			string.format(
				"Automation completion payload is %d bytes, exceeding the %d-byte server limit",
				#body,
				AUTOMATION_COMPLETION_BODY_LIMIT_BYTES
			)
		)
	end

	local url = ("%s/api/automation/jobs/%s/complete"):format(self.__baseUrl, jobId)
	return Http.post(url, body)
		:andThen(function(response)
			if response.code ~= 200 then
				return Promise.reject(
					string.format("Unexpected automation completion response status %s", tostring(response.code))
				)
			end
			local responseBody =
				decodeExecResponse(response, Types.ApiAutomationJobResponse, "completion response", "Prism automation")
			return { status = "accepted", job = responseBody }
		end)
		:catch(function(errorValue)
			if isCompletionConflict(errorValue) then
				return { status = "conflict" }
			end
			return Promise.reject(errorValue)
		end)
end

function ApiContext:updateAutomationStatus(studioMode)
	local validMode, modeError = Types.StudioMode(studioMode)
	if not validMode then
		return Promise.reject(string.format("Cannot update automation status: %s", tostring(modeError)))
	end
	if self.__pluginSessionId == nil or self.__sessionId == nil then
		return Promise.reject("Cannot update automation status without an active plugin session")
	end

	local body = Http.msgpackEncode({
		pluginSessionId = self.__pluginSessionId,
		serverSessionId = self.__sessionId,
		studioMode = studioMode,
		execHandlerAvailable = true,
		automationHandlerVersion = AUTOMATION_HANDLER_VERSION,
		pluginVersion = Version.display(Config.version),
	})
	local url = ("%s/api/automation/status"):format(self.__baseUrl)

	return Http.post(url, body):andThen(function(response)
		if response.code ~= 200 then
			return Promise.reject(string.format("Unexpected automation status response %s", tostring(response.code)))
		end

		return decodeExecResponse(
			response,
			Types.ApiAutomationHeartbeatResponse,
			"automation status response",
			"Prism automation"
		)
	end)
end

ApiContext._test = {
	beginPluginSession = beginPluginSession,
	buildAutomationClaimUrl = buildAutomationClaimUrl,
	buildExecClaimUrl = buildExecClaimUrl,
	generatePluginSessionId = generatePluginSessionId,
	withExecSession = withExecSession,
}

return ApiContext
