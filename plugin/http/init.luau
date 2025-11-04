local HttpService = game:GetService("HttpService")

local Promise = require(script.Parent.Promise)
local Log = require(script.Parent.Log)

local HttpError = require(script.Error)
local HttpResponse = require(script.Response)

local lastRequestId = 0

local Http = {}

Http.Error = HttpError
Http.Response = HttpResponse

local function performRequest(requestParams)
	local requestId = lastRequestId + 1
	lastRequestId = requestId

	Log.trace("HTTP {}({}) {}", requestParams.Method, requestId, requestParams.Url)

	if requestParams.Body ~= nil then
		Log.trace("{}", requestParams.Body)
	end

	return Promise.new(function(resolve, reject)
		coroutine.wrap(function()
			local success, response = pcall(function()
				return HttpService:RequestAsync(requestParams)
			end)

			if success then
				Log.trace("Request {} success, response {:#?}", requestId, response)
				local httpResponse = HttpResponse.fromRobloxResponse(response)
				if httpResponse:isSuccess() then
					resolve(httpResponse)
				else
					reject(HttpError.fromResponse(httpResponse))
				end
			else
				Log.trace("Request {} failure: {:?}", requestId, response)
				reject(HttpError.fromRobloxErrorString(response))
			end
		end)()
	end)
end

function Http.get(url)
	return performRequest({
		Url = url,
		Method = "GET",
	})
end

function Http.post(url, body)
	return performRequest({
		Url = url,
		Method = "POST",
		Body = body,
	})
end

function Http.jsonEncode(object)
	return HttpService:JSONEncode(object)
end

function Http.jsonDecode(source)
	return HttpService:JSONDecode(source)
end

return Http
