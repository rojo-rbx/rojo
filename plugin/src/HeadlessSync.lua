local HttpService = game:GetService("HttpService")

local HeadlessSync = {
	requestAttribute = "__Rojo_HeadlessRequest",
	responseAttribute = "__Rojo_HeadlessResponse",
}

function HeadlessSync.decodeRequest(rawRequest)
	if type(rawRequest) ~= "string" or rawRequest == "" then
		return nil, "Headless Rojo request was empty."
	end

	local ok, decoded = pcall(HttpService.JSONDecode, HttpService, rawRequest)
	if not ok then
		return nil, "Failed to decode headless Rojo request."
	end

	if type(decoded) ~= "table" then
		return nil, "Headless Rojo request was not a JSON object."
	end

	if type(decoded.requestId) ~= "string" or decoded.requestId == "" then
		return nil, "Headless Rojo request is missing requestId."
	end

	if decoded.kind ~= "attach" and decoded.kind ~= "detach" and decoded.kind ~= "status" then
		return nil, "Headless Rojo request kind must be attach, detach, or status."
	end

	return decoded
end

function HeadlessSync.extractRequestId(rawRequest)
	if type(rawRequest) ~= "string" or rawRequest == "" then
		return nil
	end

	local ok, decoded = pcall(HttpService.JSONDecode, HttpService, rawRequest)
	if not ok or type(decoded) ~= "table" then
		return nil
	end

	if type(decoded.requestId) ~= "string" or decoded.requestId == "" then
		return nil
	end

	return decoded.requestId
end

function HeadlessSync.encodeResponse(response)
	return HttpService:JSONEncode(response)
end

function HeadlessSync.parseBaseUrl(baseUrl)
	if type(baseUrl) ~= "string" or baseUrl == "" then
		return nil, nil, "Rojo baseUrl is required."
	end

	local schemeHost, port = string.match(baseUrl, "^(https?://[^/]+):(%d+)$")
	if schemeHost and port then
		return schemeHost, port
	end

	local hostOnly, hostPort = string.match(baseUrl, "^([^/:]+):(%d+)$")
	if hostOnly and hostPort then
		return hostOnly, hostPort
	end

	return nil, nil, "Rojo baseUrl must look like http://localhost:34872."
end

return HeadlessSync
