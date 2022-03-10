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
	-- Decode json 5 IEEE 754 tokens
	local json4body = self.body:gsub(":[%c ]*Infinity", ":\"Infinity\"")
	json4body = json4body:gsub(":[%c ]*-Infinity", ":\"-Infinity\"")
	json4body = json4body:gsub(":[%c ]*NaN", ":\"NaN\"")
	return HttpService:JSONDecode(json4body)
end

return Response