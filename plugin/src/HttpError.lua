local Logging = require(script.Parent.Logging)

local HttpError = {}
HttpError.__index = HttpError

HttpError.Error = {
	HttpNotEnabled = {
		message = "Rojo requires HTTP access, which is not enabled.\n" ..
			"Check your game settings, located in the 'Home' tab of Studio.",
	},
	ConnectFailed = {
		message = "Couldn't connect to the Rojo server.\n" ..
			"Make sure the server is running -- use 'rojo serve' to run it!",
	},
	Timeout = {
		message = "Request timed out.",
	},
	Unknown = {
		message = "Unknown error: {{message}}",
	},
}

setmetatable(HttpError.Error, {
	__index = function(_, key)
		error(("%q is not a valid member of HttpError.Error"):format(tostring(key)), 2)
	end,
})

function HttpError.new(type, extraMessage)
	extraMessage = extraMessage or ""
	local message = type.message:gsub("{{message}}", extraMessage)

	local err = {
		type = type,
		message = message,
	}

	setmetatable(err, HttpError)

	return err
end

function HttpError:__tostring()
	return self.message
end

--[[
	This method shouldn't have to exist. Ugh.
]]
function HttpError.fromErrorString(message)
	local lower = message:lower()

	if lower:find("^http requests are not enabled") then
		return HttpError.new(HttpError.Error.HttpNotEnabled)
	end

	if lower:find("^httperror: timedout") then
		return HttpError.new(HttpError.Error.Timeout)
	end

	if lower:find("^httperror: connectfail") then
		return HttpError.new(HttpError.Error.ConnectFailed)
	end

	return HttpError.new(HttpError.Error.Unknown, message)
end

function HttpError:report()
	Logging.warn(self.message)
end

return HttpError