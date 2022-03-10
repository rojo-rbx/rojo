local HttpService = game:GetService("HttpService")


local Encode = {}

function Encode.jsonEncode(object)
	-- Encode json 5 IEEE 754 tokens
	local body = HttpService:JSONEncode(object)
	body = body:gsub(":[%c ]*\"Infinity\"", ":Infinity")
	body = body:gsub(":[%c ]*\"%-Infinity\"", ":-Infinity")
	body = body:gsub(":[%c ]*\"NaN\"", ":NaN")
	return body
end

function Encode.jsonDecode(source)
	-- Decode json 5 IEEE 754 tokens
	local body = source:gsub(":[%c ]*Infinity", ":\"Infinity\"")
	body = body:gsub(":[%c ]*%-Infinity", ":\"-Infinity\"")
	body = body:gsub(":[%c ]*NaN", ":\"NaN\"")
	return HttpService:JSONDecode(body)
end

return Encode