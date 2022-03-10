return function()
	local Log = require(script.Parent.Parent.Log)
	local Http = require(script.Parent)
	local ieee754body = "{\"NaN\":NaN,\"Infinity\":Infinity,\"NegativeInfinity\":-Infinity,\"Boolean\":true}"

	it("should decode and encode JSON 5 IEEE 754 tokens", function()
		local json = Http.jsonDecode(ieee754body)

		expect(json.Infinity).to.equal("Infinity")
		expect(json.NegativeInfinity).to.equal("-Infinity")
		expect(json.NaN).to.equal("NaN")
		expect(json.Boolean).to.equal(true)

		local encoded = Http.jsonEncode(json);
		local decoded = Http.jsonDecode(encoded);

		expect(decoded.Infinity).to.equal("Infinity")
		expect(decoded.NegativeInfinity).to.equal("-Infinity")
		expect(decoded.NaN).to.equal("NaN")
		expect(decoded.Boolean).to.equal(true)
	end)
end