return function()
	local Log = require(script.Parent.Parent.Log)
	local Http = require(script.Parent)
	local ieee754body = "{\"Infinity\":Infinity,\"NaN\":NaN,\"NegativeInfinity\":-Infinity}"

	it("should decode JSON 5 IEEE 754 tokens", function()
		local json = Http.jsonDecode(ieee754body)

		expect(json.Infinity).to.equal("Infinity")
		expect(json.NegativeInfinity).to.equal("-Infinity")
		expect(json.NaN).to.equal("NaN")
	end)

	it("should encode JSON 5 IEEE 754 tokens", function()
		local data = {
			Infinity = "Infinity",
			NegativeInfinity = "-Infinity",
			NaN = "NaN",
		}

		local body = Http.jsonEncode(data)
		expect(body).to.equal(ieee754body)
	end)
end