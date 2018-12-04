local testLogLevel = nil
local configValue = game:FindFirstChild("ROJO_LOG")

local Level = {
	Error = 0,
	Warning = 1,
	Info = 2,
	Trace = 3,
}

local function getLogLevel()
	if testLogLevel ~= nil then
		return testLogLevel
	end

	if _G.ROJO_LOG ~= nil then
		return _G.ROJO_LOG
	end

	if configValue ~= nil then
		return configValue.Value
	end

	return Level.Info
end

local Log = {}

Log.Level = Level

function Log.trace(template, ...)
	if getLogLevel() >= Level.Trace then
		print("[Rojo-Trace] " .. string.format(template, ...))
	end
end

function Log.info(template, ...)
	if getLogLevel() >= Level.Info then
		print("[Rojo-Info] " .. string.format(template, ...))
	end
end

function Log.warn(template, ...)
	if getLogLevel() >= Level.Warning then
		warn("[Rojo-Warn] " .. string.format(template, ...))
	end
end

return Log