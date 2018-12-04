local Selection = game:GetService("Selection")
local HttpService = game:GetService("HttpService")

local Logging = require(script.Parent.Logging)

local HttpError = {}
HttpError.__index = HttpError

HttpError.Error = {
	HttpNotEnabled = {
		message = "Rojo requires HTTP access, which is not enabled.\n" ..
			"Check your game settings, located in the 'Home' tab of Studio.",
	},
	ConnectFailed = {
		message = "Rojo plugin couldn't connect to the Rojo server.\n" ..
			"Make sure the server is running -- use 'rojo serve' to run it!",
	},
	Timeout = {
		message = "Rojo timed out during a request.",
	},
	Unknown = {
		message = "Rojo encountered an unknown error: {{message}}",
	},
}

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
function HttpError.fromErrorString(err)
	err = err:lower()

	if err:find("^http requests are not enabled") then
		return HttpError.new(HttpError.Error.HttpNotEnabled)
	end

	if err:find("^curl error") then
		if err:find("couldn't connect to server") then
			return HttpError.new(HttpError.Error.ConnectFailed)
		elseif err:find("timeout was reached") then
			return HttpError.new(HttpError.Error.Timeout)
		end
	end

	return HttpError.new(HttpError.Error.Unknown, err)
end

function HttpError:report()
	Logging.warn(self.message)

	if self.type == HttpError.Error.HttpNotEnabled then
		Selection:Set(HttpService)
	end
end

return HttpError
