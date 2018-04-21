local HttpService = game:GetService("HttpService")

local HTTP_DEBUG = false

local Promise = require(script.Parent.Parent.modules.Promise)

local HttpError = require(script.Parent.HttpError)
local HttpResponse = require(script.Parent.HttpResponse)

local function dprint(...)
	if HTTP_DEBUG then
		print(...)
	end
end

local Http = {}
Http.__index = Http

function Http.new(baseUrl)
	assert(type(baseUrl) == "string", "Http.new needs a baseUrl!")

	local http = {
		baseUrl = baseUrl
	}

	setmetatable(http, Http)

	return http
end

function Http:get(endpoint)
	dprint("\nGET", endpoint)
	return Promise.new(function(resolve, reject)
		spawn(function()
			local ok, result = pcall(function()
				return HttpService:GetAsync(self.baseUrl .. endpoint, true)
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

function Http:post(endpoint, body)
	dprint("\nPOST", endpoint)
	dprint(body)
	return Promise.new(function(resolve, reject)
		spawn(function()
			local ok, result = pcall(function()
				return HttpService:PostAsync(self.baseUrl .. endpoint, body)
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

return Http
