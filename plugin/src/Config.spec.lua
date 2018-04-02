return function()
	local Config = require(script.Parent.Config)

	it("should have 'dev' disabled", function()
		expect(Config.dev).to.equal(false)
	end)
end
