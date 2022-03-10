return function()
	local Log = require(script.Parent.Parent.Log)
	local Response = require(script.Parent.Response)

	it("should decode JSON 5 IEEE 754 tokens", function()

		local response = {
			Body = "{\"Infinity\": Infinity, \"NegativeInfinity\": -Infinity, \"NaN\": NaN}",
			StatusCode = 200,
			Headers = {},
		}

		local decoded = Response.fromRobloxResponse(response)

		expect(decoded.body).to.be.ok()

		local json = decoded:json()

		expect(json.Infinity).to.equal("Infinity")
		expect(json.NegativeInfinity).to.equal("-Infinity")
		expect(json.NaN).to.equal("NaN")
	end)
end