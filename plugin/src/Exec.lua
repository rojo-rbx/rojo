local ChangeHistoryService = game:GetService("ChangeHistoryService")
local HttpService = game:GetService("HttpService")
local RunService = game:GetService("RunService")

local Packages = script.Parent.Parent.Packages
local Http = require(Packages.Http)
local Log = require(Packages.Log)
local Promise = require(Packages.Promise)

local POLL_INTERVAL_SECONDS = 0.25
local COMPLETION_RETRY_DELAY_SECONDS = 0.25
local COMPLETION_ATTEMPTS = 3
local MAX_COMPLETION_BODY_BYTES = 256 * 1024
local MAX_RESULT_DEPTH = 32
local MAX_RESULT_ELEMENTS = 10_000
local MAX_LOG_ENTRIES = 1_000
local MAX_LOG_ENTRY_BYTES = 8 * 1024
local MAX_ERROR_BYTES = 64 * 1024
local MAX_TRACEBACK_BYTES = 64 * 1024
local LOG_TRUNCATION_MESSAGE = "[Prism Exec] Logs truncated to fit protocol limits"

-- The current Rust claim response does not expose its execution deadline. Its
-- store uses 30 seconds, so leave one second for the completion request. This
-- should become a wire value instead of a compatibility constant when the
-- protocol grows an execution-timeout field.
local EXECUTION_TIMEOUT_SECONDS = 29

local temporaryNameCounter = 0

local function safeTostring(value, stringify): string
	local ok, rendered = pcall(stringify or tostring, value)
	if ok and type(rendered) == "string" then
		return rendered
	end

	return "<value could not be formatted>"
end

local function truncateUtf8(value: string, limit: number, suffix: string?): string
	if #value <= limit then
		return value
	end

	suffix = suffix or ""
	local contentLimit = math.max(0, limit - #suffix)
	local truncated = value:sub(1, contentLimit)
	while #truncated > 0 and utf8.len(truncated) == nil do
		truncated = truncated:sub(1, #truncated - 1)
	end

	return truncated .. suffix
end

local function captureError(errorValue, tracebackFunction)
	local message = truncateUtf8(safeTostring(errorValue, tostring), MAX_ERROR_BYTES, "... [truncated]")
	local tracebackOk, traceback = pcall(tracebackFunction, message, 2)
	if not tracebackOk then
		traceback = "<traceback could not be formatted>"
	else
		traceback = safeTostring(traceback, tostring)
	end

	return {
		error = message,
		traceback = truncateUtf8(traceback, MAX_TRACEBACK_BYTES, "... [truncated]"),
	}
end

local function makeProtectedOnce(callback)
	local called = false
	local packedResult = nil

	return function(...)
		if not called then
			called = true
			packedResult = table.pack(pcall(callback, ...))
		end

		return table.unpack(packedResult, 1, packedResult.n)
	end
end

local function sanitizeScriptName(scriptName: string): string
	local sanitized = scriptName:gsub("[^%w%._%-]", "_"):gsub("_+", "_")
	sanitized = sanitized:sub(1, 48)

	if sanitized == "" or sanitized:find("%w") == nil then
		return "script"
	end

	return sanitized
end

local function makeTemporaryName(scriptName: string, uniqueToken: string): string
	temporaryNameCounter += 1

	return string.format(
		"__RojoExec_%s_%d_%s",
		sanitizeScriptName(scriptName),
		temporaryNameCounter,
		sanitizeScriptName(uniqueToken)
	)
end

local function buildWrapper(source: string): string
	return "return function(rojoExec)\n" .. "\tlocal plugin = nil\n" .. source .. "\nend\n"
end

local function encodeExecValue(value)
	local seen = {}
	local elementCount = 0

	local function encode(current, depth, path)
		elementCount += 1
		if elementCount > MAX_RESULT_ELEMENTS then
			return nil, string.format("Exec result exceeds the %d-element limit", MAX_RESULT_ELEMENTS)
		end
		if depth > MAX_RESULT_DEPTH then
			return nil, string.format("Exec result exceeds the maximum depth of %d at %s", MAX_RESULT_DEPTH, path)
		end

		local valueType = type(current)
		if valueType == "nil" then
			return { kind = "nil" }
		elseif valueType == "string" then
			return { kind = "string", value = current }
		elseif valueType == "number" then
			if current ~= current or current == math.huge or current == -math.huge then
				return nil, string.format("Exec result contains a non-finite number at %s", path)
			end
			return { kind = "number", value = current }
		elseif valueType == "boolean" then
			return { kind = "boolean", value = current }
		elseif valueType ~= "table" then
			return nil, string.format("Unsupported exec result type '%s' at %s", typeof(current), path)
		end

		if seen[current] then
			return nil, string.format("Exec result contains a cycle at %s", path)
		end
		seen[current] = true

		local numericKeys = 0
		local stringKeys = {}
		local maximumIndex = 0
		for key in current do
			local keyType = type(key)
			if keyType == "number" and key >= 1 and key % 1 == 0 then
				numericKeys += 1
				maximumIndex = math.max(maximumIndex, key)
			elseif keyType == "string" then
				table.insert(stringKeys, key)
			else
				seen[current] = nil
				return nil, string.format("Exec result table at %s has unsupported key type '%s'", path, typeof(key))
			end
		end

		if numericKeys > 0 and #stringKeys > 0 then
			seen[current] = nil
			return nil, string.format("Exec result table at %s mixes array and dictionary keys", path)
		end

		local encoded
		if numericKeys > 0 then
			if maximumIndex ~= numericKeys then
				seen[current] = nil
				return nil, string.format("Exec result array at %s is sparse", path)
			end

			local values = table.create(numericKeys)
			for index = 1, numericKeys do
				local child, childError = encode(current[index], depth + 1, string.format("%s[%d]", path, index))
				if child == nil then
					seen[current] = nil
					return nil, childError
				end
				values[index] = child
			end
			encoded = { kind = "array", value = values }
		else
			table.sort(stringKeys)
			local entries = table.create(#stringKeys)
			for index, key in stringKeys do
				local child, childError = encode(current[key], depth + 1, string.format("%s.%s", path, key))
				if child == nil then
					seen[current] = nil
					return nil, childError
				end
				entries[index] = {
					key = key,
					value = child,
				}
			end
			encoded = { kind = "table", value = entries }
		end

		seen[current] = nil
		return encoded
	end

	return encode(value, 1, "result")
end

local function makeLogCollector(stringify)
	local entries = {}
	local wasTruncated = false

	local collector = {}

	function collector:add(level, ...)
		if #entries >= MAX_LOG_ENTRIES then
			wasTruncated = true
			return
		end

		local renderedArguments = table.create(select("#", ...))
		for index = 1, select("#", ...) do
			renderedArguments[index] = safeTostring(select(index, ...), stringify)
		end

		local message = table.concat(renderedArguments, "\t")
		if #message > MAX_LOG_ENTRY_BYTES then
			message = truncateUtf8(message, MAX_LOG_ENTRY_BYTES, "... [truncated]")
			wasTruncated = true
		end

		table.insert(entries, {
			level = level,
			message = message,
		})
	end

	function collector:finish()
		local result = table.clone(entries)
		if wasTruncated then
			if #result >= MAX_LOG_ENTRIES then
				table.remove(result)
			end
			table.insert(result, {
				level = "warn",
				message = LOG_TRUNCATION_MESSAGE,
			})
		end

		return result
	end

	return collector
end

local function completionPayloadSize(payload, encodePayload)
	local ok, body = pcall(encodePayload, payload)
	if not ok then
		return nil, safeTostring(body, tostring)
	end
	if type(body) ~= "string" then
		return nil, "MessagePack encoder did not return a string"
	end

	return #body
end

local function boundCompletionPayload(payload, encodePayload)
	if payload.error ~= nil then
		payload.error = truncateUtf8(payload.error, MAX_ERROR_BYTES, "... [truncated]")
	end
	if payload.traceback ~= nil then
		payload.traceback = truncateUtf8(payload.traceback, MAX_TRACEBACK_BYTES, "... [truncated]")
	end

	local size, sizeError = completionPayloadSize(payload, encodePayload)
	if size == nil then
		return {
			outcome = "runtimeFailure",
			error = "Could not encode exec completion payload: " .. sizeError,
			logs = {},
		}
	end
	if size <= MAX_COMPLETION_BODY_BYTES then
		return payload
	end

	local removedLogs = false
	while #payload.logs > 0 and size > MAX_COMPLETION_BODY_BYTES do
		table.remove(payload.logs)
		removedLogs = true
		size = completionPayloadSize(payload, encodePayload)
		if size == nil then
			break
		end
	end

	if removedLogs then
		local marker = {
			level = "warn",
			message = LOG_TRUNCATION_MESSAGE,
		}
		table.insert(payload.logs, marker)
		local markedSize = completionPayloadSize(payload, encodePayload)
		if markedSize == nil or markedSize > MAX_COMPLETION_BODY_BYTES then
			table.remove(payload.logs)
		else
			size = markedSize
		end
	end

	if size ~= nil and size <= MAX_COMPLETION_BODY_BYTES then
		return payload
	end

	if payload.outcome == "success" then
		return boundCompletionPayload({
			outcome = "runtimeFailure",
			error = string.format(
				"Exec result exceeds the %d-byte completion payload limit",
				MAX_COMPLETION_BODY_BYTES
			),
			logs = payload.logs,
		}, encodePayload)
	end

	payload.traceback = nil
	return payload
end

local function failurePayload(outcome, errorMessage, traceback)
	return {
		outcome = outcome,
		error = truncateUtf8(safeTostring(errorMessage, tostring), MAX_ERROR_BYTES, "... [truncated]"),
		traceback = traceback,
		logs = {},
	}
end

local function makeExecutionController(dependencies)
	local controller = {
		temporaryModule = nil,
		recording = nil,
		logs = makeLogCollector(dependencies.stringify),
	}

	controller.finishRecording = makeProtectedOnce(function()
		if controller.recording ~= nil then
			dependencies.finishRecording(controller.recording)
		end
	end)

	controller.cleanup = makeProtectedOnce(function()
		if controller.temporaryModule ~= nil then
			dependencies.cleanupTemporaryModule(controller.temporaryModule)
		end
	end)

	return controller
end

local function finalizeController(controller, payload, dependencies)
	local finishOk, finishError = controller.finishRecording()
	if not finishOk then
		local captured = captureError(finishError, dependencies.traceback)
		local message = "Failed to finish the Prism exec undo recording: " .. captured.error
		if payload.outcome == "success" then
			payload = failurePayload("runtimeFailure", message, captured.traceback)
		else
			payload.error = payload.error .. "\n" .. message
			payload.traceback = payload.traceback or captured.traceback
		end
	end

	local cleanupOk, cleanupError = controller.cleanup()
	if not cleanupOk then
		local captured = captureError(cleanupError, dependencies.traceback)
		local message = "Failed to clean up the temporary Prism exec ModuleScript: " .. captured.error
		if payload.outcome == "success" then
			payload = failurePayload("runtimeFailure", message, captured.traceback)
		else
			payload.error = payload.error .. "\n" .. message
			payload.traceback = payload.traceback or captured.traceback
		end
	end

	payload.logs = controller.logs:finish()
	return boundCompletionPayload(payload, dependencies.encodePayload)
end

local function runJob(job, dependencies, controller)
	local payload = nil
	local controllerOk, controllerError = xpcall(function()
		if not dependencies.isEdit() then
			payload = failurePayload("rejected", "Prism exec is available only in Studio edit mode")
			return
		end

		local temporaryName = makeTemporaryName(job.scriptName, dependencies.generateUniqueToken())
		local wrapper = buildWrapper(job.source)
		controller.temporaryModule = dependencies.createTemporaryModule()
		dependencies.configureTemporaryModule(controller.temporaryModule, temporaryName, wrapper)

		if not dependencies.isEdit() then
			payload = failurePayload("rejected", "Studio left edit mode before the exec wrapper could be compiled")
			return
		end

		local loadOk, loadedValue = pcall(dependencies.requireTemporaryModule, controller.temporaryModule)
		if not loadOk then
			-- Studio reports the detailed parser diagnostic to Output, but require
			-- only returns the generic module-load error. Keep the unique module
			-- name in our error so the two messages can be correlated.
			payload = failurePayload(
				"compileFailure",
				string.format("%s (temporary module: %s)", safeTostring(loadedValue, tostring), temporaryName)
			)
			return
		end

		if type(loadedValue) ~= "function" then
			payload = failurePayload(
				"compileFailure",
				string.format("Exec wrapper returned '%s' instead of a callable function", type(loadedValue))
			)
			return
		end

		controller.recording = dependencies.tryBeginRecording("Prism Exec: " .. job.scriptName)
		if controller.recording == nil then
			payload = failurePayload("rejected", "Could not begin the Prism exec undo recording")
			return
		end

		if not dependencies.isEdit() then
			payload = failurePayload("rejected", "Studio left edit mode before the exec function could be invoked")
			return
		end

		local hasExplicitOutput = false
		local explicitOutput = nil
		local rojoExec = table.freeze({
			output = function(value)
				hasExplicitOutput = true
				explicitOutput = value
			end,
			print = function(...)
				controller.logs:add("print", ...)
			end,
			warn = function(...)
				controller.logs:add("warn", ...)
			end,
		})

		local runtimeOk, runtimeValue = xpcall(function()
			return loadedValue(rojoExec)
		end, function(errorValue)
			return captureError(errorValue, dependencies.traceback)
		end)

		if not runtimeOk then
			payload = failurePayload("runtimeFailure", runtimeValue.error, runtimeValue.traceback)
			return
		end

		local resultValue = if hasExplicitOutput then explicitOutput else runtimeValue
		local encodedResult, resultError = encodeExecValue(resultValue)
		if encodedResult == nil then
			payload = failurePayload("runtimeFailure", resultError)
			return
		end

		payload = {
			outcome = "success",
			result = encodedResult,
			logs = {},
		}
	end, function(errorValue)
		return captureError(errorValue, dependencies.traceback)
	end)

	if not controllerOk then
		payload = failurePayload("runtimeFailure", controllerError.error, controllerError.traceback)
	elseif payload == nil then
		payload = failurePayload("runtimeFailure", "Prism exec ended without producing a completion payload")
	end

	return finalizeController(controller, payload, dependencies)
end

local defaultExecutionDependencies = {
	isEdit = function()
		return RunService:IsEdit()
	end,
	generateUniqueToken = function()
		return HttpService:GenerateGUID(false)
	end,
	createTemporaryModule = function()
		return Instance.new("ModuleScript")
	end,
	configureTemporaryModule = function(temporaryModule, name, wrapper)
		temporaryModule.Name = name
		temporaryModule.Archivable = false
		temporaryModule.Source = wrapper
		temporaryModule.Parent = script
	end,
	requireTemporaryModule = function(temporaryModule)
		return require(temporaryModule)
	end,
	tryBeginRecording = function(label)
		return ChangeHistoryService:TryBeginRecording(label)
	end,
	finishRecording = function(recording)
		ChangeHistoryService:FinishRecording(recording, Enum.FinishRecordingOperation.Commit)
	end,
	cleanupTemporaryModule = function(temporaryModule)
		temporaryModule:Destroy()
	end,
	traceback = debug.traceback,
	stringify = tostring,
	encodePayload = Http.msgpackEncode,
	spawn = task.spawn,
	cancel = task.cancel,
	delay = Promise.delay,
}

local function startExecution(job, dependencies, timeoutSeconds)
	dependencies = dependencies or defaultExecutionDependencies
	timeoutSeconds = timeoutSeconds or EXECUTION_TIMEOUT_SECONDS

	local controller = makeExecutionController(dependencies)
	local workerThread = nil
	local workerPromise = Promise.new(function(resolve, _reject, onCancel)
		onCancel(function()
			if workerThread ~= nil then
				pcall(dependencies.cancel, workerThread)
			end
			controller.finishRecording()
			controller.cleanup()
		end)

		workerThread = dependencies.spawn(function()
			resolve({
				kind = "completed",
				payload = runJob(job, dependencies, controller),
			})
		end)
	end)

	local timeoutPromise = dependencies.delay(timeoutSeconds):andThen(function()
		return {
			kind = "timeout",
		}
	end)

	return Promise.race({ workerPromise, timeoutPromise }):andThen(function(event)
		if event.kind == "completed" then
			return event.payload
		end

		-- This is a soft timeout. A yielding worker can be cancelled, but a
		-- non-yielding infinite loop can block this controller until Studio's
		-- own script watchdog intervenes.
		local payload = failurePayload("timeout", string.format("Execution exceeded %.3g seconds", timeoutSeconds))
		return finalizeController(controller, payload, dependencies)
	end)
end

local Exec = {}
Exec.__index = Exec

local function classifyStudioMode(isEdit, isRunMode, isRunning)
	if isEdit then
		return "edit"
	elseif isRunMode then
		return "run"
	elseif isRunning then
		return "play"
	else
		return "unknown"
	end
end

local function currentStudioMode()
	return classifyStudioMode(RunService:IsEdit(), RunService:IsRunMode(), RunService:IsRunning())
end

function Exec.new(options)
	assert(type(options) == "table", "Exec options must be a table")
	assert(type(options.apiContext) == "table", "Exec apiContext must be a table")

	local dependencies = options.dependencies or {}
	local isEdit = dependencies.isEdit or defaultExecutionDependencies.isEdit
	local self = {
		__apiContext = options.apiContext,
		__studioMode = dependencies.studioMode or function()
			if dependencies.isEdit ~= nil then
				return if isEdit() then "edit" else "play"
			end
			return currentStudioMode()
		end,
		__delay = dependencies.delay or Promise.delay,
		__execute = dependencies.execute or function(job)
			return startExecution(job, defaultExecutionDependencies, EXECUTION_TIMEOUT_SECONDS)
		end,
		__onError = options.onError or function(errorValue)
			Log.error("Prism exec poller failed: {}", errorValue)
		end,
		__running = false,
		__generation = 0,
		__busy = false,
		__scheduledPromise = nil,
		__executionPromise = nil,
		__currentJobId = nil,
	}

	return setmetatable(self, Exec)
end

function Exec:__isCurrent(generation)
	return self.__running and self.__generation == generation
end

function Exec:__fail(generation, errorValue)
	if not self:__isCurrent(generation) then
		return
	end

	self.__onError(errorValue)
end

function Exec:__schedulePoll(generation, delaySeconds)
	if not self:__isCurrent(generation) then
		return
	end

	local delayPromise = if delaySeconds == 0 then Promise.resolve() else self.__delay(delaySeconds)
	self.__scheduledPromise = delayPromise
		:andThen(function()
			if self:__isCurrent(generation) then
				self:__poll(generation)
			end
		end)
		:catch(function(errorValue)
			self:__fail(generation, errorValue)
		end)
end

function Exec:__releaseJobAndPoll(generation)
	if not self:__isCurrent(generation) then
		return
	end

	self.__busy = false
	self.__executionPromise = nil
	self.__currentJobId = nil
	self:__schedulePoll(generation, POLL_INTERVAL_SECONDS)
end

function Exec:__complete(generation, jobId, payload, attempt)
	if not self:__isCurrent(generation) then
		return
	end

	local requestOk, completionPromise =
		pcall(self.__apiContext.completeExecJob, self.__apiContext, jobId, payload, self.__studioMode())
	if not requestOk then
		self:__fail(generation, completionPromise)
		return
	end

	completionPromise
		:andThen(function(result)
			if not self:__isCurrent(generation) then
				return
			end

			if result.status == "conflict" then
				Log.warn(
					"Prism exec completion for job {} returned HTTP 409; treating it as already committed or expired",
					jobId
				)
			end

			self:__releaseJobAndPoll(generation)
		end)
		:catch(function(errorValue)
			if not self:__isCurrent(generation) then
				return
			end

			if attempt >= COMPLETION_ATTEMPTS then
				self:__fail(
					generation,
					string.format(
						"Could not acknowledge Prism exec job %s after %d attempts: %s",
						jobId,
						COMPLETION_ATTEMPTS,
						safeTostring(errorValue, tostring)
					)
				)
				return
			end

			self.__delay(COMPLETION_RETRY_DELAY_SECONDS):andThen(function()
				if self:__isCurrent(generation) then
					self:__complete(generation, jobId, payload, attempt + 1)
				end
			end)
		end)
end

function Exec:__executeClaimedJob(generation, job)
	if not self:__isCurrent(generation) then
		return
	end

	self.__currentJobId = job.jobId
	local executionOk, executionPromise = pcall(self.__execute, job)
	if not executionOk then
		self:__fail(generation, executionPromise)
		return
	end

	self.__executionPromise = executionPromise
	executionPromise
		:andThen(function(payload)
			if self:__isCurrent(generation) then
				self:__complete(generation, job.jobId, payload, 1)
			end
		end)
		:catch(function(errorValue)
			self:__fail(generation, errorValue)
		end)
end

function Exec:__poll(generation)
	if not self:__isCurrent(generation) or self.__busy then
		return
	end

	local studioMode = self.__studioMode()
	if studioMode ~= "edit" then
		self:__schedulePoll(generation, POLL_INTERVAL_SECONDS)
		return
	end

	self.__busy = true
	local claimOk, claimPromise = pcall(self.__apiContext.claimNextExecJob, self.__apiContext, studioMode)
	if not claimOk then
		self:__fail(generation, claimPromise)
		return
	end

	claimPromise
		:andThen(function(job)
			if not self:__isCurrent(generation) then
				return
			end

			if job == nil then
				self.__busy = false
				self:__schedulePoll(generation, POLL_INTERVAL_SECONDS)
				return
			end

			self:__executeClaimedJob(generation, job)
		end)
		:catch(function(errorValue)
			self:__fail(generation, errorValue)
		end)
end

function Exec:start()
	if self.__running then
		return
	end

	self.__running = true
	self.__generation += 1
	self:__schedulePoll(self.__generation, 0)
end

function Exec:stop()
	if not self.__running then
		return
	end

	self.__running = false
	self.__generation += 1

	if self.__scheduledPromise ~= nil then
		self.__scheduledPromise:cancel()
		self.__scheduledPromise = nil
	end
	if self.__executionPromise ~= nil then
		self.__executionPromise:cancel()
		self.__executionPromise = nil
	end

	self.__busy = false
	self.__currentJobId = nil
end

Exec._test = {
	boundCompletionPayload = boundCompletionPayload,
	buildWrapper = buildWrapper,
	encodeExecValue = encodeExecValue,
	makeExecutionController = makeExecutionController,
	makeLogCollector = makeLogCollector,
	makeProtectedOnce = makeProtectedOnce,
	makeTemporaryName = makeTemporaryName,
	runJob = runJob,
	sanitizeScriptName = sanitizeScriptName,
	startExecution = startExecution,
	classifyStudioMode = classifyStudioMode,
	constants = {
		completionAttempts = COMPLETION_ATTEMPTS,
		maxCompletionBodyBytes = MAX_COMPLETION_BODY_BYTES,
		maxLogEntries = MAX_LOG_ENTRIES,
		maxLogEntryBytes = MAX_LOG_ENTRY_BYTES,
	},
}

return Exec
