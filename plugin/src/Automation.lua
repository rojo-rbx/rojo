local RunService = game:GetService("RunService")

local Packages = script.Parent.Parent.Packages
local Log = require(Packages.Log)
local Promise = require(Packages.Promise)

local Inspect = require(script.Parent.AutomationHandlers.Inspect)
local InstanceReferences = require(script.Parent.InstanceReferences)

local POLL_INTERVAL_SECONDS = 0.25
local COMPLETION_RETRY_SECONDS = 0.25
local COMPLETION_ATTEMPTS = 3

local function currentStudioMode()
	if RunService:IsEdit() then
		return "edit"
	elseif RunService:IsRunMode() then
		return "run"
	elseif RunService:IsRunning() then
		return "play"
	else
		return "unknown"
	end
end

local function dispatch(job, references)
	if job.request.kind ~= "inspect" then
		return { outcome = "failure", error = "Unsupported automation job kind: " .. tostring(job.request.kind) }
	end
	local ok, result, handlerError = pcall(Inspect.run, job.request, { references = references })
	if not ok then
		return { outcome = "failure", error = "Inspect handler failed: " .. tostring(result) }
	end
	if result == nil then
		return { outcome = "failure", error = tostring(handlerError) }
	end
	return {
		outcome = "success",
		result = {
			kind = "inspect",
			root = result.root,
			visitedInstances = result.visitedInstances,
			truncated = result.truncated,
			truncationReason = result.truncationReason,
		},
	}
end

local Automation = {}
Automation.__index = Automation

function Automation.new(options)
	local dependencies = options.dependencies or {}
	return setmetatable({
		__apiContext = options.apiContext,
		__delay = dependencies.delay or Promise.delay,
		__studioMode = dependencies.studioMode or currentStudioMode,
		__dispatch = dependencies.dispatch or dispatch,
		__makeReferences = dependencies.makeReferences or InstanceReferences.new,
		__onError = options.onError or function(errorValue)
			Log.error("Prism automation poller failed: {}", errorValue)
		end,
		__running = false,
		__generation = 0,
		__busy = false,
		__scheduledPromise = nil,
		__references = nil,
	}, Automation)
end

function Automation:__isCurrent(generation)
	return self.__running and self.__generation == generation
end

function Automation:__fail(generation, errorValue)
	if self:__isCurrent(generation) then
		self.__onError(errorValue)
	end
end

function Automation:__schedule(generation, delaySeconds)
	if not self:__isCurrent(generation) then
		return
	end
	self.__scheduledPromise = self.__delay(delaySeconds):andThen(function()
		if self:__isCurrent(generation) then
			self:__poll(generation)
		end
	end)
end

function Automation:__release(generation)
	if self:__isCurrent(generation) then
		self.__busy = false
		self:__schedule(generation, POLL_INTERVAL_SECONDS)
	end
end

function Automation:__complete(generation, jobId, payload, attempt)
	if not self:__isCurrent(generation) then
		return
	end
	local ok, promise =
		pcall(self.__apiContext.completeAutomationJob, self.__apiContext, jobId, payload, self.__studioMode())
	if not ok then
		self:__fail(generation, promise)
		return
	end
	promise
		:andThen(function(response)
			if not self:__isCurrent(generation) then
				return
			end
			if response.status == "conflict" then
				Log.warn("Prism automation completion for job {} returned HTTP 409", jobId)
			end
			self:__release(generation)
		end)
		:catch(function(errorValue)
			if not self:__isCurrent(generation) then
				return
			end
			if attempt >= COMPLETION_ATTEMPTS then
				self:__fail(generation, errorValue)
				return
			end
			self.__delay(COMPLETION_RETRY_SECONDS):andThen(function()
				if self:__isCurrent(generation) then
					self:__complete(generation, jobId, payload, attempt + 1)
				end
			end)
		end)
end

function Automation:__poll(generation)
	if not self:__isCurrent(generation) or self.__busy then
		return
	end
	local mode = self.__studioMode()
	if mode ~= "edit" then
		self:__schedule(generation, POLL_INTERVAL_SECONDS)
		return
	end
	self.__busy = true
	local ok, promise = pcall(self.__apiContext.claimNextAutomationJob, self.__apiContext, mode)
	if not ok then
		self:__fail(generation, promise)
		return
	end
	promise
		:andThen(function(job)
			if not self:__isCurrent(generation) then
				return
			end
			if job == nil then
				self:__release(generation)
				return
			end
			local payload = self.__dispatch(job, self.__references)
			self:__complete(generation, job.jobId, payload, 1)
		end)
		:catch(function(errorValue)
			self:__fail(generation, errorValue)
		end)
end

function Automation:start()
	if self.__running then
		return
	end
	local sessionId = self.__apiContext:getPluginSessionId()
	if sessionId == nil then
		self.__onError("Cannot start automation poller without a plugin session ID")
		return
	end
	self.__references = self.__makeReferences(sessionId)
	self.__running = true
	self.__generation += 1
	self:__schedule(self.__generation, 0)
end

function Automation:stop()
	if not self.__running then
		return
	end
	self.__running = false
	self.__generation += 1
	self.__busy = false
	if self.__scheduledPromise ~= nil then
		self.__scheduledPromise:cancel()
		self.__scheduledPromise = nil
	end
	if self.__references ~= nil then
		self.__references:clear()
		self.__references = nil
	end
end

Automation._test = { dispatch = dispatch, currentStudioMode = currentStudioMode }

return Automation
