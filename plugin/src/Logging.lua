local DevSettings = require(script.Parent.DevSettings)

local Level = {
	Error = 0,
	Warning = 1,
	Info = 2,
	Trace = 3,
}

local testLogLevel = nil

local function getLogLevel()
	if testLogLevel ~= nil then
		return testLogLevel
	end

	return DevSettings:getLogLevel()
end

local function addTags(tag, message)
	return tag .. message:gsub("\n", "\n" .. tag)
end

local INFO_TAG = (" "):rep(15) .. "[Rojo-Info] "
local TRACE_TAG = (" "):rep(15) .. "[Rojo-Trace] "
local WARN_TAG = "[Rojo-Warn] "

local Log = {}

Log.Level = Level

function Log.trace(template, ...)
	if getLogLevel() >= Level.Trace then
		print(addTags(TRACE_TAG, string.format(template, ...)))
	end
end

function Log.info(template, ...)
	if getLogLevel() >= Level.Info then
		print(addTags(INFO_TAG, string.format(template, ...)))
	end
end

function Log.warn(template, ...)
	if getLogLevel() >= Level.Warning then
		warn(addTags(WARN_TAG, string.format(template, ...)))
	end
end

return Log