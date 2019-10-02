local HttpService = game:GetService("HttpService")

local stringTemplate = [[
Http.Response {
	code: %d
	body: %s
}]]

local Response = {}
Response.__index = Response

function Response:__tostring()
	return stringTemplate:format(self.code, self.body)
end

function Response.fromRobloxResponse(response)
	local self = {
		body = response.Body,
		code = response.StatusCode,
		headers = response.Headers,
	}

	return setmetatable(self, Response)
end

function Response:isSuccess()
	return self.code >= 200 and self.code < 300
end

function Response:json()
	return HttpService:JSONDecode(self.body)
end

return Response