local ChangeHistoryService = game:GetService("ChangeHistoryService")
local HttpService = game:GetService("HttpService")
local RunService = game:GetService("RunService")

export type ExecSpikeResult = {
	ok: boolean,
	phase: "success" | "compile" | "runtime" | "rejected" | "internal",
	result: (string | number | boolean)?,
	error: string?,
	traceback: string?,
	cleanupError: string?,
}

local ExecSpike = {}

local temporaryNameCounter = 0

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
		"__RojoExecSpike_%s_%d_%s",
		sanitizeScriptName(scriptName),
		temporaryNameCounter,
		sanitizeScriptName(uniqueToken)
	)
end

local function buildWrapper(source: string): string
	-- Shadow the plugin global as defense in depth. The trusted source receives
	-- only the deliberately minimal rojoExec table below.
	return "return function(rojoExec)\n"
		.. "\tlocal plugin = nil\n"
		.. "\t-- submitted source begins here\n"
		.. source
		.. "\nend\n"
end

local function validateResult(value: any): (boolean, string?)
	local valueType = type(value)
	if valueType == "nil" or valueType == "string" or valueType == "number" or valueType == "boolean" then
		return true
	end

	return false,
		string.format(
			"Unsupported exec result type '%s'; this spike accepts only nil, string, number, or boolean",
			valueType
		)
end

local function formatErrorValue(errorValue: any): string
	local ok, message = pcall(tostring, errorValue)
	if ok then
		return message
	end

	return "<error value could not be formatted>"
end

local function formatTraceback(errorValue: any, tracebackFunction: (string, number) -> any): string
	local tracebackOk, traceback = pcall(tracebackFunction, formatErrorValue(errorValue), 2)
	if tracebackOk then
		return tostring(traceback)
	end

	return "<traceback could not be formatted>"
end

local function captureError(errorValue: any, tracebackFunction: (string, number) -> any)
	return {
		error = formatErrorValue(errorValue),
		traceback = formatTraceback(errorValue, tracebackFunction),
	}
end

local function makeProtectedOnce(callback)
	local called = false

	return function(...)
		if called then
			return true
		end

		called = true
		return pcall(callback, ...)
	end
end

local function failure(phase, errorMessage, traceback): ExecSpikeResult
	return {
		ok = false,
		phase = phase,
		error = tostring(errorMessage),
		traceback = traceback,
	}
end

local function runWithDependencies(source: any, scriptName: any, dependencies): ExecSpikeResult
	if type(source) ~= "string" then
		return failure("rejected", "Exec source must be a string")
	end

	if scriptName ~= nil and type(scriptName) ~= "string" then
		return failure("rejected", "Exec script name must be a string when provided")
	end

	local displayName = sanitizeScriptName(scriptName or "script.lua")
	local temporaryModule = nil
	local recording = nil
	local finalResult: ExecSpikeResult? = nil

	local finishRecordingOnce = makeProtectedOnce(function()
		dependencies.finishRecording(recording)
	end)
	local cleanupOnce = makeProtectedOnce(function()
		dependencies.cleanupTemporaryModule(temporaryModule)
	end)

	local controllerOk, controllerFailure = xpcall(function()
		if not dependencies.isEdit() then
			finalResult = failure("rejected", "Rojo exec spike is available only in Studio edit mode")
			return
		end

		local wrapper = buildWrapper(source)
		local temporaryName = makeTemporaryName(displayName, dependencies.generateUniqueToken())

		-- Keep the instance in controller scope before any protected Source or
		-- parenting operation so every later failure can destroy it.
		temporaryModule = dependencies.createTemporaryModule()
		dependencies.configureTemporaryModule(temporaryModule, temporaryName, wrapper)

		-- Re-check at the actual compile boundary in case Studio changed modes
		-- while the temporary module was being prepared.
		if not dependencies.isEdit() then
			finalResult = failure("rejected", "Studio left edit mode before the exec wrapper could be compiled")
			return
		end

		local loadOk, loadedValue = pcall(dependencies.requireTemporaryModule, temporaryModule)
		if not loadOk then
			finalResult = failure("compile", loadedValue)
			return
		end

		if type(loadedValue) ~= "function" then
			finalResult = failure(
				"compile",
				string.format("Exec wrapper returned '%s' instead of a callable function", type(loadedValue))
			)
			return
		end

		recording = dependencies.tryBeginRecording("Rojo Exec Spike: " .. displayName)
		if recording == nil then
			finalResult = failure("internal", "Could not begin the Rojo exec spike undo recording")
			return
		end

		-- The recording is open now, making this the final check immediately
		-- before invoking user code. A rejection still finishes the recording.
		if not dependencies.isEdit() then
			finalResult = failure("rejected", "Studio left edit mode before the exec function could be invoked")
			return
		end

		local runtimeOk, runtimePayload = xpcall(function()
			local rojoExec = table.freeze({})
			return {
				value = loadedValue(rojoExec),
			}
		end, function(errorValue)
			return captureError(errorValue, dependencies.traceback)
		end)

		-- Commit even after a runtime failure so any partial mutations remain a
		-- single undoable action owned by this controller.
		local finishOk, finishError = finishRecordingOnce()
		if not finishOk then
			local captured = captureError(finishError, dependencies.traceback)
			finalResult = failure(
				"internal",
				"Failed to finish the Rojo exec spike undo recording: " .. captured.error,
				captured.traceback
			)
			return
		end

		if not runtimeOk then
			finalResult = failure("runtime", runtimePayload.error, runtimePayload.traceback)
			return
		end

		local resultOk, resultError = validateResult(runtimePayload.value)
		if not resultOk then
			finalResult = failure("runtime", resultError)
			return
		end

		finalResult = {
			ok = true,
			phase = "success",
			result = runtimePayload.value,
		}
	end, function(errorValue)
		return captureError(errorValue, dependencies.traceback)
	end)

	if not controllerOk then
		finalResult = failure("internal", controllerFailure.error, controllerFailure.traceback)
	end

	if recording ~= nil then
		local finishOk, finishError = finishRecordingOnce()
		if not finishOk then
			local captured = captureError(finishError, dependencies.traceback)
			finalResult = failure(
				"internal",
				"Failed to finish the Rojo exec spike undo recording: " .. captured.error,
				captured.traceback
			)
		end
	end

	if temporaryModule ~= nil then
		local cleanupOk, cleanupError = cleanupOnce()
		if not cleanupOk then
			local cleanupMessage = tostring(cleanupError)
			if finalResult == nil or finalResult.ok then
				finalResult = failure("internal", "Execution completed, but temporary ModuleScript cleanup failed")
			end
			finalResult.cleanupError = cleanupMessage
		end
	end

	if finalResult == nil then
		return failure("internal", "Rojo exec spike ended without a result")
	end

	return finalResult
end

local defaultDependencies = {
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

		-- A child of this ModuleScript stays in the plugin-owned hierarchy and
		-- never enters Workspace or another user project container.
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
}

function ExecSpike.run(source: string, scriptName: string?): ExecSpikeResult
	return runWithDependencies(source, scriptName, defaultDependencies)
end

-- These are intentionally fixed scalar-result cases for manual Studio
-- verification. They are not run by the automated suite because mocks cannot
-- answer whether plugin security permits dynamic Source assignment and require.
ExecSpike.ManualSources = {
	readOnly = [[return #workspace:GetChildren()]],
	createPart = [[local part = Instance.new("Part")
part.Name = "RojoExecSpike"
part.Parent = workspace
return part.Name]],
	attachmentAndAttribute = [[local part = Instance.new("Part")
part.Name = "RojoExecSpike"
part.Parent = workspace

local attachment = Instance.new("Attachment")
attachment.Parent = part

part:SetAttribute("CreatedByExecSpike", true)

return part:GetAttribute("CreatedByExecSpike")]],
	compileFailure = [[local =]],
	runtimeFailure = [[local part = Instance.new("Part")
part.Name = "RojoExecPartial"
part.Parent = workspace
error("intentional runtime failure")]],
	yielding = [[task.wait(0.25)
return "yielded"]],
}

-- Exposed only so TestEZ can verify the controller's pure structure. Calling
-- these helpers does not establish Studio execution feasibility.
ExecSpike._test = {
	sanitizeScriptName = sanitizeScriptName,
	makeTemporaryName = makeTemporaryName,
	buildWrapper = buildWrapper,
	validateResult = validateResult,
	formatTraceback = formatTraceback,
	makeProtectedOnce = makeProtectedOnce,
	runWithDependencies = runWithDependencies,
}

return ExecSpike
