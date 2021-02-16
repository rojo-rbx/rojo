return function()
	local HttpService = game:GetService("HttpService")
	
	local RbxDom = require(script.Parent)
	local EncodedValue = require(script.Parent.EncodedValue)
	local allValues = require(script.Parent.allValues)

	local function deepEq(a, b)
		if typeof(a) ~= typeof(b) then
			return false
		end

		local ty = typeof(a)

		if ty == "table" then
			local visited = {}
			
			for key, valueA in pairs(a) do
				visited[key] = true
				
				if not deepEq(valueA, b[key]) then
					return false
				end
			end

			for key, valueB in pairs(b) do
				if visited[key] then
					continue
				end

				if not deepEq(valueB, a[key]) then
					return false
				end
			end

			return true
		else
			return a == b
		end
	end

	for testName, testEntry in pairs(allValues) do
		it("round trip " .. testName, function()
			local ok, decoded = EncodedValue.decode(testEntry.value)
			assert(ok, decoded)

			local ok, encoded = EncodedValue.encode(decoded, testEntry.ty)
			assert(ok, encoded)

			if not deepEq(encoded, testEntry.value) then
				local expected = HttpService:JSONEncode(testEntry.value)
				local actual = HttpService:JSONEncode(encoded)

				local message = string.format(
					"Round-trip results did not match.\nExpected:\n%s\nActual:\n%s",
					expected, actual
				)

				error(message)
			end
		end)
	end
end
