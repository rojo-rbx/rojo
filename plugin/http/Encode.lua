local HttpService = game:GetService("HttpService")


local Encode = {}

function Encode.jsonEncode(object)
	-- Encode json 5 IEEE 754 tokens
	local body = HttpService:JSONEncode(object)
	body = body:gsub("([:\[\,][%c%s]*)\"([\[\-NI]%a+)\"", "%1%2")
	return body
end

function Encode.jsonDecode(source)
	-- Decode json 5 IEEE 754 tokens
	local body = source:gsub("([:\[\,][%c%s]*)([\-NI]%a+)", "%1\"%2\"")
	print(body)
	return HttpService:JSONDecode(body)
end

return Encode