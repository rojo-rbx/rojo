local HttpService = game:GetService("HttpService")

local stringTemplate = [[
HttpResponse {
	code: %d
	body: %s
}]]

local HttpResponse = {}
HttpResponse.__index = HttpResponse

function HttpResponse:__tostring()
	return stringTemplate:format(self.code, self.body)
end

function HttpResponse.new(body)
	local response = {
		body = body,
	}

	setmetatable(response, HttpResponse)

	return response
end

function HttpResponse.fromRobloxResponse(response)
	local self = {
		body = response.Body,
		code = response.StatusCode,
		headers = response.Headers,
	}

	return setmetatable(self, HttpResponse)
end

function HttpResponse:isSuccess()
	return self.code >= 200 and self.code < 300
end

function HttpResponse:json()
	return HttpService:JSONDecode(self.body)
end

return HttpResponse
