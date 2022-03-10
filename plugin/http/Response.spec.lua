return function()
	local Log = require(script.Parent.Parent.Log)
	local Response = require(script.Parent.Response)

	it("should decode JSON 5 IEEE 754 tokens", function()

		local response = {
			Body = "{\"infinity\": Infinity, \"NegativeInfinity\": -Infinity, \"NaN\": NaN}",
			StatusCode = 200,
			Headers = {},
		}

		local decoded = Response.fromRobloxResponse(response)

		expect(decoded.body).to.be.ok()

		local json = decoded.json()

		expect(json.Infinity).to.equal(math.huge)
		expect(json.NegativeInfinity).to.equal(-math.huge)
		expect(json.NaN).to.equal(0/0)
	end)
end