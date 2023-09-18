return function()
	local Version = require(script.Parent.Version)

	it("should compare equal versions", function()
		expect(Version.compare({ 1, 2, 3 }, { 1, 2, 3 })).to.equal(0)
		expect(Version.compare({ 0, 4, 0 }, { 0, 4 })).to.equal(0)
		expect(Version.compare({ 0, 0, 123 }, { 0, 0, 123 })).to.equal(0)
		expect(Version.compare({ 26 }, { 26 })).to.equal(0)
		expect(Version.compare({ 26, 42 }, { 26, 42 })).to.equal(0)
		expect(Version.compare({ 1, 0, 0 }, { 1 })).to.equal(0)
	end)

	it("should compare newer, older versions", function()
		expect(Version.compare({ 1 }, { 0 })).to.equal(1)
		expect(Version.compare({ 1, 1 }, { 1, 0 })).to.equal(1)
	end)

	it("should compare different major versions", function()
		expect(Version.compare({ 1, 3, 2 }, { 2, 2, 1 })).to.equal(-1)
		expect(Version.compare({ 1, 2 }, { 2, 1 })).to.equal(-1)
		expect(Version.compare({ 1 }, { 2 })).to.equal(-1)
	end)

	it("should compare different minor versions", function()
		expect(Version.compare({ 1, 2, 3 }, { 1, 3, 2 })).to.equal(-1)
		expect(Version.compare({ 50, 1 }, { 50, 2 })).to.equal(-1)
	end)
end
