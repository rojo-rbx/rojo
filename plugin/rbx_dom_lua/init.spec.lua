return function()
	local RbxDom = require(script.Parent)

	it("should load", function()
		expect(RbxDom).to.be.ok()
	end)
end
