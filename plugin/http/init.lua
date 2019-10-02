local HttpService = game:GetService("HttpService")

local Promise = require(script.Parent.Promise)
local Log = require(script.Parent.Log)

local HttpError = require(script.Error)
local HttpResponse = require(script.Response)

local lastRequestId = 0

local Http = {}

Http.Error = HttpError
Http.Response = HttpResponse

function Http.get(url)
	local requestId = lastRequestId + 1
	lastRequestId = requestId

	Log.trace("GET(%d) %s", requestId, url)

	return Promise.new(function(resolve, reject)
		coroutine.wrap(function()
			local success, response = pcall(function()
				return HttpService:RequestAsync({
					Url = url,
					Method = "GET",
				})
			end)

			if success then
				Log.trace("Request %d success: status code %s", requestId, response.StatusCode)
				resolve(HttpResponse.fromRobloxResponse(response))
			else
				Log.trace("Request %d failure: %s", requestId, response)
				reject(HttpError.fromErrorString(response))
			end
		end)()
	end)
end

function Http.post(url, body)
	local requestId = lastRequestId + 1
	lastRequestId = requestId

	Log.trace("POST(%d) %s\n%s", requestId, url, body)

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
				Log.trace("Request %d success: status code %s", requestId, response.StatusCode)
				resolve(HttpResponse.fromRobloxResponse(response))
			else
				Log.trace("Request %d failure: %s", requestId, response)
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