local DevSettings = require(script.Parent.DevSettings)

local testLogLevel = nil

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

	local hyperValue = DevSettings:getLogLevel()
	if hyperValue ~= nil then
		return hyperValue
	end

	return Level.Info
end

local function addTags(tag, message)
	return tag .. message:gsub("\n", "\n" .. tag)
end

local Log = {}

Log.Level = Level

function Log.trace(template, ...)
	if getLogLevel() >= Level.Trace then
		print(addTags("[Rojo-Trace] ", string.format(template, ...)))
	end
end

function Log.info(template, ...)
	if getLogLevel() >= Level.Info then
		print(addTags("[Rojo-Info] ", string.format(template, ...)))
	end
end

function Log.warn(template, ...)
	if getLogLevel() >= Level.Warning then
		warn(addTags("[Rojo-Warn] ", string.format(template, ...)))
	end
end

return Log