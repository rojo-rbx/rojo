local t = require(script.Parent.Parent.t)

local strict = require(script.Parent.strict)

local Status = strict("Session.Status", {
	NotStarted = "NotStarted",
	Connecting = "Connecting",
	Connected = "Connected",
	Disconnected = "Disconnected",
})

local function DEBUG_printPatch(patch)
	local HttpService = game:GetService("HttpService")


	for removed in ipairs(patch.removed) do
		print("Remove:", removed)
	end

	for id, added in pairs(patch.added) do
		print("Add:", id, HttpService:JSONEncode(added))
	end

	for updated in ipairs(patch.updated) do
		print("Update:", HttpService:JSONEncode(updated))
	end
end

local ServeSession = {}
ServeSession.__index = ServeSession

ServeSession.Status = Status

local validateServeOptions = t.strictInterface({
	apiContext = t.table,
	reconciler = t.table,
})

function ServeSession.new(options)
	assert(validateServeOptions(options))

	local self = {
		__status = Status.NotStarted,
		__apiContext = options.apiContext,
		__reconciler = options.reconciler,
		__statusChangedCallback = nil,
	}

	setmetatable(self, ServeSession)

	return self
end

function ServeSession:onStatusChanged(callback)
	self.__statusChangedCallback = callback
end

function ServeSession:start()
	self:__setStatus(Status.Connecting)

	self.__apiContext:connect()
		:andThen(function(serverInfo)
			self:__setStatus(Status.Connected)

			local rootInstanceId = serverInfo.rootInstanceId

			return self.__apiContext:read({ rootInstanceId })
				:andThen(function(readResponseBody)
					local hydratePatch = self.__reconciler:hydrate(
						readResponseBody.instances,
						rootInstanceId,
						game
					)

					-- TODO: Prompt user to notify them of this patch, since
					-- it's effectively a conflict between the Rojo server and
					-- the client.

					self.__reconciler:applyPatch(hydratePatch)

					-- TODO: Applying a patch may eventually only apply part of
					-- the patch and start a content negotiation process with
					-- the Rojo server. We should handle that!

					local function mainLoop()
						return self.__apiContext:retrieveMessages()
							:andThen(function(messages)
								for _, message in ipairs(messages) do
									-- TODO: Update server to return patches in
									-- correct format so that we don't have to
									-- transform them for the reconciler.

									local asPatch = {
										removed = message.removedInstances,
										updated = message.updatedInstances,
										added = message.addedInstances,
									}

									self.__reconciler:applyPatch(asPatch)
								end

								if self.__status ~= Status.Disconnected then
									return mainLoop()
								end
							end)
					end

					return mainLoop()
				end)
		end)
		:catch(function(err)
			self:__setStatus(Status.Disconnected, err)
		end)
end

function ServeSession:stop()
	self:__setStatus(Status.Disconnected)
end

function ServeSession:__setStatus(status, detail)
	self.__status = status

	if self.__statusChangedCallback ~= nil then
		self.__statusChangedCallback(status, detail)
	end
end

return ServeSession