local rojoValueToRobloxValue = require(script.Parent.rojoValueToRobloxValue)

return function()
	it("should convert primitives", function()
		local inputString = {
			Type = "String",
			Value = "Hello, world!",
		}

		local inputFloat32 = {
			Type = "Float32",
			Value = 12341.512,
		}

		expect(rojoValueToRobloxValue(inputString)).to.equal(inputString.Value)
		expect(rojoValueToRobloxValue(inputFloat32)).to.equal(inputFloat32.Value)
	end)

	it("should convert properties with direct constructors", function()
		local inputColor3 = {
			Type = "Color3",
			Value = {0, 1, 0.5},
		}
		local outputColor3 = Color3.new(0, 1, 0.5)

		local inputCFrame = {
			Type = "CFrame",
			Value = {
				1, 2, 3,
				4, 5, 6,
				7, 8, 9,
				10, 11, 12,
			},
		}
		local outputCFrame = CFrame.new(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12)

		expect(rojoValueToRobloxValue(inputColor3)).to.equal(outputColor3)
		expect(rojoValueToRobloxValue(inputCFrame)).to.equal(outputCFrame)
	end)
end