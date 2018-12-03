local enabledByTest = false

local function isEnabled()
	return _G.ROJO_LOG or enabledByTest
end

local Logging = {}

function Logging.info(template, ...)
	if isEnabled() then
		print("[Rojo] " .. template:format(...))
	end
end

function Logging.warn(template, ...)
	if isEnabled() then
		warn("[Rojo] " .. template:format(...))
	end
end

return Logging