return function()
	local Log = require(script.Parent.Parent.Log)
	local Http = require(script.Parent)
	local ieee754body = "{\"NaN\":NaN,\"Infinity\":\nInfinity,\"NegativeInfinity\": -Infinity,\"Boolean\":true,\"Array\":[10, Infinity, -Infinity, NaN,100]}"

	it("should decode and encode JSON 5 IEEE 754 tokens", function()
		local json = Http.jsonDecode(ieee754body)

		expect(json).to.be.ok()
		expect(json.Infinity).to.equal("Infinity")
		expect(json.NegativeInfinity).to.equal("-Infinity")
		expect(json.NaN).to.equal("NaN")
		expect(json.Boolean).to.equal(true)
		expect(json.Array[1]).to.equal(10)
		expect(json.Array[2]).to.equal("Infinity")
		expect(json.Array[3]).to.equal("-Infinity")
		expect(json.Array[4]).to.equal("NaN")
		expect(json.Array[5]).to.equal(100)

		local encoded = Http.jsonEncode(json);
		local decoded = Http.jsonDecode(encoded);

		expect(decoded).to.be.ok()
		expect(decoded.Infinity).to.equal("Infinity")
		expect(decoded.NegativeInfinity).to.equal("-Infinity")
		expect(decoded.NaN).to.equal("NaN")
		expect(decoded.Boolean).to.equal(true)
		expect(json.Array[1]).to.equal(10)
		expect(json.Array[2]).to.equal("Infinity")
		expect(json.Array[3]).to.equal("-Infinity")
		expect(json.Array[4]).to.equal("NaN")
		expect(json.Array[5]).to.equal(100)
	end)
end