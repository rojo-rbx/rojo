local HttpService = game:GetService("HttpService")

local HTTP_DEBUG = false

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
		spawn(function()
			local ok, result = pcall(function()
				return HttpService:GetAsync(url, true)
			end)

			if ok then
				dprint("\t", result, "\n")
				resolve(HttpResponse.new(result))
			else
				reject(HttpError.fromErrorString(result))
			end
		end)
	end)
end

function Http.post(url, body)
	dprint("\nPOST", url)
	dprint(body)
	return Promise.new(function(resolve, reject)
		spawn(function()
			local ok, result = pcall(function()
				return HttpService:PostAsync(url, body)
			end)

			if ok then
				dprint("\t", result, "\n")
				resolve(HttpResponse.new(result))
			else
				reject(HttpError.fromErrorString(result))
			end
		end)
	end)
end

function Http.jsonEncode(object)
	return HttpService:JSONEncode(object)
end

function Http.jsonDecode(source)
	return HttpService:JSONDecode(source)
end

return Http
