local EncodingService = game:GetService("EncodingService")

return {
	decode = function(input: string)
		return buffer.tostring(EncodingService:Base64Decode(buffer.fromstring(input)))
	end,
	encode = function(input: string)
		return buffer.tostring(EncodingService:Base64Encode(buffer.fromstring(input)))
	end,
}
