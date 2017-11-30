local HttpService = game:GetService("HttpService")

local HttpResponse = {}
HttpResponse.__index = HttpResponse

function HttpResponse.new(body)
	local response = {
		body = body,
	}

	setmetatable(response, HttpResponse)

	return response
end

function HttpResponse:json()
	return HttpService:JSONDecode(self.body)
end

return HttpResponse
