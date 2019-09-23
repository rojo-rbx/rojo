local HttpService = game:GetService("HttpService")

local Promise = require(script.Parent.Parent.Promise)

local Logging = require(script.Parent.Logging)
local HttpError = require(script.Parent.HttpError)
local HttpResponse = require(script.Parent.HttpResponse)

local lastRequestId = 0

local isBuiltInPlugin = pcall(function ()
	-- This is a hack, but there is no explicit way to detect the security context of a thread.
	-- If this routine runs without throwing, then we are running from the BuiltInPlugins folder!
	local CorePackages = game:GetService("CorePackages")
	tostring(CorePackages)
end)

-- TODO: Factor out into separate library, especially error handling
local Http = {}

local function tryEnableHttp()
	if isBuiltInPlugin and not HttpService:GetHttpEnabled() then
		Logging.warn("Automatically enabling the HttpService.")
		HttpService:SetHttpEnabled(true)
	end
end

function Http.get(url)
	local requestId = lastRequestId + 1
	lastRequestId = requestId
	
	tryEnableHttp()
	Logging.trace("GET(%d) %s", requestId, url)

	return Promise.new(function(resolve, reject)
		coroutine.wrap(function()
			local success, response = pcall(function()
				return HttpService:RequestAsync({
					Url = url,
					Method = "GET",
				})
			end)

			if success then
				Logging.trace("Request %d success: status code %s", requestId, response.StatusCode)
				resolve(HttpResponse.fromRobloxResponse(response))
			else
				Logging.trace("Request %d failure: %s", requestId, response)
				reject(HttpError.fromErrorString(response))
			end
		end)()
	end)
end

function Http.post(url, body)
	local requestId = lastRequestId + 1
	lastRequestId = requestId

	tryEnableHttp()
	Logging.trace("POST(%d) %s\n%s", requestId, url, body)

	return Promise.new(function(resolve, reject)
		coroutine.wrap(function()
			local success, response = pcall(function()
				return HttpService:RequestAsync({
					Url = url,
					Method = "POST",
					Body = body,
				})
			end)

			if success then
				Logging.trace("Request %d success: status code %s", requestId, response.StatusCode)
				resolve(HttpResponse.fromRobloxResponse(response))
			else
				Logging.trace("Request %d failure: %s", requestId, response)
				reject(HttpError.fromErrorString(response))
			end
		end)()
	end)
end

function Http.jsonEncode(object)
	return HttpService:JSONEncode(object)
end

function Http.jsonDecode(source)
	return HttpService:JSONDecode(source)
end

return Http
