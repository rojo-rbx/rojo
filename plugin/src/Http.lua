local HttpService = game:GetService("HttpService")

local HTTP_DEBUG = true

local Promise = require(script.Parent.Parent.Promise)

local HttpError = require(script.Parent.HttpError)
local HttpResponse = require(script.Parent.HttpResponse)

local function dprint(...)
	if HTTP_DEBUG then
		print(...)
	end
end

-- TODO: Factor out into separate library, especially error handling
local Http = {}

function Http.get(url)
	dprint("\nGET", url)

	return Promise.new(function(resolve, reject)
		coroutine.wrap(function()
			local success, response = pcall(function()
				return HttpService:RequestAsync({
					Url = url,
					Method = "GET",
				})
			end)

			if success then
				dprint("\t", response)
				resolve(HttpResponse.fromRobloxResponse(response))
			else
				reject(HttpError.fromErrorString(response))
			end
		end)()
	end)
end

function Http.post(url, body)
	dprint("\nPOST", url)
	dprint(body);

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
				dprint("\t", response)
				resolve(HttpResponse.fromRobloxResponse(response))
			else
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
