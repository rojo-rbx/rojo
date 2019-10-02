local Promise = require(script.Parent.Parent.Promise)

local ApiContext = require(script.Parent.ApiContext)
local Reconciler = require(script.Parent.Reconciler)

local Session = {}
Session.__index = Session

function Session.new(config)
	local baseUrl = ("http://%s:%s"):format(config.address, config.port)
	local apiContext = ApiContext.new(baseUrl)

	local self = {
		onError = config.onError,
		disconnected = false,
		reconciler = Reconciler.new(),
		apiContext = apiContext,
	}

	apiContext:connect()
		:andThen(function()
			if self.disconnected then
				return
			end

			return apiContext:read({apiContext.rootInstanceId})
		end)
		:andThen(function(response)
			if self.disconnected then
				return
			end

			self.reconciler:reconcile(response.instances, apiContext.rootInstanceId, game)
			return self:__processMessages()
		end)
		:catch(function(message)
			self.disconnected = true
			self.onError(message)
		end)

	return not self.disconnected, setmetatable(self, Session)
end

function Session:__processMessages()
	if self.disconnected then
		return Promise.resolve()
	end

	return self.apiContext:retrieveMessages()
		:andThen(function(messages)
			local promise = Promise.resolve(nil)

			for _, message in ipairs(messages) do
				promise = promise:andThen(function()
					return self:__onMessage(message)
				end)
			end

			return promise
		end)
		:andThen(function()
			return self:__processMessages()
		end)
end

function Session:__onMessage(message)
	if self.disconnected then
		return Promise.resolve()
	end

	local requestedIds = {}

	for _, id in ipairs(message.added) do
		table.insert(requestedIds, id)
	end

	for _, id in ipairs(message.updated) do
		table.insert(requestedIds, id)
	end

	for _, id in ipairs(message.removed) do
		table.insert(requestedIds, id)
	end

	return self.apiContext:read(requestedIds)
		:andThen(function(response)
			return self.reconciler:applyUpdate(requestedIds, response.instances)
		end)
end

function Session:disconnect()
	self.disconnected = true
end

return Session