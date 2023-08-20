return function()
	local HttpService = game:GetService("HttpService")

	local EncodedValue = require(script.Parent.EncodedValue)
	local allValues = require(script.Parent.allValues)

	local function deepEq(a, b)
		if typeof(a) ~= typeof(b) then
			return false
		end

		local ty = typeof(a)

		if ty == "table" then
			local visited = {}

			for key, valueA in a do
				visited[key] = true

				if not deepEq(valueA, b[key]) then
					return false
				end
			end

			for key, valueB in b do
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

	local extraAssertions = {
		CFrame = function(value)
			expect(value).to.equal(CFrame.new(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12))
		end,
	}

	for testName, testEntry in allValues do
		it("round trip " .. testName, function()
			local decodeOk, decoded = EncodedValue.decode(testEntry.value)
			assert(decodeOk, decoded)

			if extraAssertions[testName] ~= nil then
				extraAssertions[testName](decoded)
			end

			local encodeOk, encoded = EncodedValue.encode(decoded, testEntry.ty)
			assert(encodeOk, encoded)

			if not deepEq(encoded, testEntry.value) then
				local expected = HttpService:JSONEncode(testEntry.value)
				local actual = HttpService:JSONEncode(encoded)

				error(`Round-trip results did not match.\nExpected:\n{expected}\nActual:\n{actual}`)
			end
		end)
	end
end
