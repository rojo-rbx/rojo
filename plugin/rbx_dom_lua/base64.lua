-- Thanks to Tiffany352 for this base64 implementation!

local floor = math.floor
local char = string.char

local function encodeBase64(str)
	local out = {}
	local nOut = 0
	local alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
	local strLen = #str

	-- 3 octets become 4 hextets
	for i = 1, strLen - 2, 3 do
		local b1, b2, b3 = str:byte(i, i + 3)
		local word = b3 + b2 * 256 + b1 * 256 * 256

		local h4 = word % 64 + 1
		word = floor(word / 64)
		local h3 = word % 64 + 1
		word = floor(word / 64)
		local h2 = word % 64 + 1
		word = floor(word / 64)
		local h1 = word % 64 + 1

		out[nOut + 1] = alphabet:sub(h1, h1)
		out[nOut + 2] = alphabet:sub(h2, h2)
		out[nOut + 3] = alphabet:sub(h3, h3)
		out[nOut + 4] = alphabet:sub(h4, h4)
		nOut = nOut + 4
	end

	local remainder = strLen % 3

	if remainder == 2 then
		-- 16 input bits -> 3 hextets (2 full, 1 partial)
		local b1, b2 = str:byte(-2, -1)
		-- partial is 4 bits long, leaving 2 bits of zero padding ->
		-- offset = 4
		local word = b2 * 4 + b1 * 4 * 256

		local h3 = word % 64 + 1
		word = floor(word / 64)
		local h2 = word % 64 + 1
		word = floor(word / 64)
		local h1 = word % 64 + 1

		out[nOut + 1] = alphabet:sub(h1, h1)
		out[nOut + 2] = alphabet:sub(h2, h2)
		out[nOut + 3] = alphabet:sub(h3, h3)
		out[nOut + 4] = "="
	elseif remainder == 1 then
		-- 8 input bits -> 2 hextets (2 full, 1 partial)
		local b1 = str:byte(-1, -1)
		-- partial is 2 bits long, leaving 4 bits of zero padding ->
		-- offset = 16
		local word = b1 * 16

		local h2 = word % 64 + 1
		word = floor(word / 64)
		local h1 = word % 64 + 1

		out[nOut + 1] = alphabet:sub(h1, h1)
		out[nOut + 2] = alphabet:sub(h2, h2)
		out[nOut + 3] = "="
		out[nOut + 4] = "="
	end
	-- if the remainder is 0, then no work is needed

	return table.concat(out, "")
end

local function decodeBase64(str)
	local out = {}
	local nOut = 0
	local alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
	local strLen = #str
	local acc = 0
	local nAcc = 0

	local alphabetLut = {}
	for i = 1, #alphabet do
		alphabetLut[alphabet:sub(i, i)] = i - 1
	end

	-- 4 hextets become 3 octets
	for i = 1, strLen do
		local ch = str:sub(i, i)
		local byte = alphabetLut[ch]
		if byte then
			acc = acc * 64 + byte
			nAcc = nAcc + 1
		end

		if nAcc == 4 then
			local b3 = acc % 256
			acc = floor(acc / 256)
			local b2 = acc % 256
			acc = floor(acc / 256)
			local b1 = acc % 256

			out[nOut + 1] = char(b1)
			out[nOut + 2] = char(b2)
			out[nOut + 3] = char(b3)
			nOut = nOut + 3
			nAcc = 0
			acc = 0
		end
	end

	if nAcc == 3 then
		-- 3 hextets -> 16 bit output
		acc = acc * 64
		acc = floor(acc / 256)
		local b2 = acc % 256
		acc = floor(acc / 256)
		local b1 = acc % 256

		out[nOut + 1] = char(b1)
		out[nOut + 2] = char(b2)
	elseif nAcc == 2 then
		-- 2 hextets -> 8 bit output
		acc = acc * 64
		acc = floor(acc / 256)
		acc = acc * 64
		acc = floor(acc / 256)
		local b1 = acc % 256

		out[nOut + 1] = char(b1)
	elseif nAcc == 1 then
		error("Base64 has invalid length")
	end

	return table.concat(out, "")
end

return {
	decode = decodeBase64,
	encode = encodeBase64,
}
